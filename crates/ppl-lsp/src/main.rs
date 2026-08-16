use std::collections::HashMap;
use std::mem;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};

use dashmap::DashMap;
use icy_board_engine::ast::{
    Ast, AstVisitor, Constant, ConstantExpression, Expression, FunctionCallExpression, ParameterSpecifier, ProcedureCallStatement, RecordLiteralExpression,
    walk_function_call_expression, walk_function_declaration, walk_function_implementation, walk_predefined_call_statement, walk_procedure_call_statement,
    walk_variable_declaration_statement,
};
use icy_board_engine::compiler::{CompilationErrorType, CompilationWarningType, workspace::Workspace};
use icy_board_engine::executable::{FUNCTION_DEFINITIONS, FunctionDefinition, FunctionSignature, VariableType};
use icy_board_engine::formatting::FormattingVisitor;
use icy_board_engine::icy_board::read_data_with_encoding_detection;
use icy_board_engine::parser::lexer::LexingErrorType;
use icy_board_engine::parser::{Encoding, ErrorReporter, ParserWarningType, UserTypeRegistry, parse_ast_with_predeclared_types, preparse_type_declarations};
use icy_board_engine::semantic::{FunctionDeclaration, ReferenceType, SemanticVisitor};
use icyboard_ppl::completion::get_completion;
use icyboard_ppl::document_symbol::get_document_symbols;
use icyboard_ppl::documentation::{get_const_hover, get_function_hover, get_statement_hover, get_type_hover};
use icyboard_ppl::formatting::VSCodeFormattingBackend;
use icyboard_ppl::hover::get_user_hover;
use icyboard_ppl::jump_definition::get_definition;
use icyboard_ppl::reference::get_reference;
use icyboard_ppl::semantic_tokens::{get_semantic_tokens, legend_modifiers, legend_types};
use icyboard_ppl::signature_help::get_signature_help;
use icyboard_ppl::{line_before_cursor, offset_to_position, position_to_offset};
use ropey::Rope;
use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,

    workspace: Mutex<Workspace>,
    workspace_visitor: Mutex<SemanticVisitor>,
    workspace_map: DashMap<Url, Ast>,

    ast_map: Arc<Mutex<HashMap<Url, (Ast, SemanticVisitor)>>>,
    document_map: DashMap<Url, Rope>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, params: InitializeParams) -> Result<InitializeResult> {
        if let Some(root) = params.root_uri {
            if let Ok(root) = root.to_file_path() {
                self.load_workspace(root);
            }
        }
        Ok(InitializeResult {
            server_info: None,
            offset_encoding: None,
            capabilities: ServerCapabilities {
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),

                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: Some(vec![".".to_string(), "{".to_string()]),
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),

                signature_help_provider: Some(SignatureHelpOptions {
                    trigger_characters: Some(vec!["(".to_string(), ",".to_string(), " ".to_string()]),
                    retrigger_characters: Some(vec![",".to_string()]),
                    work_done_progress_options: Default::default(),
                }),

                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),

                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                document_symbol_provider: Some(OneOf::Left(true)),
                document_highlight_provider: Some(OneOf::Left(true)),
                code_action_provider: Some(CodeActionProviderCapability::Simple(true)),
                document_formatting_provider: Some(OneOf::Left(true)),
                document_range_formatting_provider: Some(OneOf::Left(true)),
                semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
                    work_done_progress_options: Default::default(),
                    legend: SemanticTokensLegend {
                        token_types: legend_types(),
                        token_modifiers: legend_modifiers(),
                    },
                    range: Some(false),
                    full: Some(SemanticTokensFullOptions::Bool(true)),
                })),
                ..ServerCapabilities::default()
            },
        })
    }

    async fn initialized(&self, _: InitializedParams) {
        self.client.log_message(MessageType::INFO, "initialized!").await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri,
            text: params.text_document.text,
            //version: params.text_document.version,
        })
        .await
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        let Some(change) = params.content_changes.get_mut(0) else {
            return;
        };
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri,
            text: std::mem::take(&mut change.text),
            // version: params.text_document.version,
        })
        .await;
    }

    async fn did_save(&self, _: DidSaveTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "file saved!").await;
    }
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        self.client.publish_diagnostics(uri.clone(), Vec::new(), None).await;
        self.ast_map.lock().unwrap().remove(&uri);
        self.document_map.remove(&uri);
    }

    async fn code_action(&self, params: CodeActionParams) -> Result<Option<CodeActionResponse>> {
        let uri = params.text_document.uri;
        let Some(rope) = self.document_map.get(&uri) else {
            return Ok(None);
        };
        let mut actions = Vec::new();
        for diagnostic in params.context.diagnostics {
            let Some(NumberOrString::String(code)) = diagnostic.code.as_ref() else {
                continue;
            };
            let (title, target_uri, edits) = match code.as_str() {
                "ppl.procedure-closed-with-endfunc" => (
                    "Replace ENDFUNC with ENDPROC",
                    uri.clone(),
                    vec![TextEdit::new(diagnostic.range, "ENDPROC".to_string())],
                ),
                "ppl.function-closed-with-endproc" => (
                    "Replace ENDPROC with ENDFUNC",
                    uri.clone(),
                    vec![TextEdit::new(diagnostic.range, "ENDFUNC".to_string())],
                ),
                "ppl.obsolete-brace-open" => ("Replace { with (", uri.clone(), vec![TextEdit::new(diagnostic.range, "(".to_string())]),
                "ppl.obsolete-brace-close" => ("Replace } with )", uri.clone(), vec![TextEdit::new(diagnostic.range, ")".to_string())]),
                "ppl.obsolete-pow" => ("Replace ** with ^", uri.clone(), vec![TextEdit::new(diagnostic.range, "^".to_string())]),
                "ppl.var-not-allowed" => (
                    "Remove VAR",
                    uri.clone(),
                    vec![TextEdit::new(with_trailing_blanks(&rope, diagnostic.range), String::new())],
                ),
                "ppl.routine-needs-call" => {
                    let at = Range::new(diagnostic.range.end, diagnostic.range.end);
                    ("Call the routine", uri.clone(), vec![TextEdit::new(at, "()".to_string())])
                }
                "ppl.duplicate-record-field" => {
                    let Some(start) = position_to_offset(&rope, diagnostic.range.start) else {
                        continue;
                    };
                    let edit = self.get_ast(&uri, |ast, _| duplicate_field_edit(ast, &rope, start)).ok().flatten();
                    let Some(edit) = edit else {
                        continue;
                    };
                    ("Remove duplicate field", uri.clone(), vec![edit])
                }
                "ppl.unused-label" => ("Remove unused label", uri.clone(), vec![remove_line_edit(&rope, diagnostic.range.start.line)]),
                "ppl.unused-routine" => {
                    let Some(edits) = remove_routine_edits(&rope, diagnostic.range) else {
                        continue;
                    };
                    ("Remove unused routine", uri.clone(), edits)
                }
                "ppl.unused-variable" => {
                    let Some(edit) = remove_variable_edit(&rope, diagnostic.range) else {
                        continue;
                    };
                    ("Remove unused variable", uri.clone(), vec![edit])
                }
                "ppl.missing-implementation" => {
                    let Some(edit) = implementation_stub_edit(&rope, diagnostic.range.start.line) else {
                        continue;
                    };
                    ("Create missing routine implementation", uri.clone(), vec![edit])
                }
                "ppl.unknown-identifier" | "ppl.unknown-enum-member" | "ppl.unknown-record-field" | "ppl.next-identifier-mismatch" => {
                    let Some(replacement) = diagnostic.data.as_ref().and_then(|data| data.get("replacement")).and_then(Value::as_str) else {
                        continue;
                    };
                    let title = format!("Replace with {replacement}");
                    let edits = vec![TextEdit::new(diagnostic.range, replacement.to_string())];
                    actions.push(quick_fix(&title, uri.clone(), diagnostic, edits, true));
                    continue;
                }
                "ppl.end-is-not-a-statement" => {
                    let legacy = diagnostic.data.as_ref().and_then(|data| data.get("legacy")).and_then(Value::as_u64);
                    let upgrade = vec![TextEdit::new(diagnostic.range, "EXIT".to_string())];
                    actions.push(quick_fix("Replace END with EXIT", uri.clone(), diagnostic.clone(), upgrade, true));
                    if let Some(legacy) = legacy {
                        if let Some(edit) = language_version_directive_edit(&rope, legacy as u16) {
                            let title = format!("Read this file as language version {legacy}");
                            actions.push(quick_fix(&title, uri.clone(), diagnostic, vec![edit], false));
                        }
                    }
                    continue;
                }
                "ppl.too-many-arguments" => {
                    let Some(expected) = diagnostic.data.as_ref().and_then(|data| data.get("expected")).and_then(Value::as_u64) else {
                        continue;
                    };
                    let Some(start) = position_to_offset(&rope, diagnostic.range.start) else {
                        continue;
                    };
                    let edit = self
                        .get_ast(&uri, |ast, _| excess_arguments_edit(ast, &rope, start, expected as usize))
                        .ok()
                        .flatten();
                    let Some(edit) = edit else {
                        continue;
                    };
                    ("Remove excess arguments", uri.clone(), vec![edit])
                }
                "ppl.runtime-too-old" | "ppl.language-version-too-old" => {
                    let Some(required) = diagnostic.data.as_ref().and_then(|data| data.get("required")).and_then(Value::as_u64) else {
                        continue;
                    };
                    let setting = if code == "ppl.runtime-too-old" {
                        ManifestSetting::Runtime
                    } else {
                        ManifestSetting::LanguageVersion
                    };
                    let workspace = self.workspace.lock().unwrap();
                    let Some((manifest_uri, edit)) = manifest_version_edit(&workspace.file_name, setting, required as u16) else {
                        continue;
                    };
                    let title = format!("Set project version to {required}");
                    actions.push(quick_fix(&title, manifest_uri, diagnostic, vec![edit], true));
                    continue;
                }
                _ => continue,
            };
            actions.push(quick_fix(title, target_uri, diagnostic, edits, true));
        }
        Ok((!actions.is_empty()).then_some(actions))
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        if !self.document_map.contains_key(&uri) {
            return Ok(None);
        }
        self.get_ast(&uri, |ast, visitor| {
            let rope = self.document_map.get(&uri)?;

            let offset = position_to_offset(&rope, params.text_document_position_params.position)?;

            get_tooltip(ast, offset).or_else(|| get_user_hover(ast, visitor, offset))
        })
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        let uri2 = params.text_document_position_params.text_document.uri.clone();
        let uri = params.text_document_position_params.text_document.uri;
        if !self.document_map.contains_key(&uri) {
            return Ok(None);
        }
        let res = self.get_ast(&uri, |ast, visitor| {
            let rope = self.document_map.get(&uri2)?;

            let offset = position_to_offset(&rope, params.text_document_position_params.position)?;
            if let Some((path, r)) = get_definition(&ast, visitor, offset) {
                return self.location(&path, r.span).map(GotoDefinitionResponse::Scalar);
            }
            None
        });
        if let Ok(Some(r)) = &res {
            self.client.log_message(MessageType::INFO, format!("{:?}!", r)).await;
        }

        res
    }

    async fn document_symbol(&self, params: DocumentSymbolParams) -> Result<Option<DocumentSymbolResponse>> {
        let uri = params.text_document.uri;
        let Some(rope) = self.document_map.get(&uri) else {
            return Ok(None);
        };
        let symbols = self.get_ast(&uri, |ast, _| get_document_symbols(ast, &rope))?;
        Ok(Some(DocumentSymbolResponse::Nested(symbols)))
    }

    async fn document_highlight(&self, params: DocumentHighlightParams) -> Result<Option<Vec<DocumentHighlight>>> {
        let uri = params.text_document_position_params.text_document.uri;
        let Some(rope) = self.document_map.get(&uri) else {
            return Ok(None);
        };
        let Some(offset) = position_to_offset(&rope, params.text_document_position_params.position) else {
            return Ok(None);
        };

        let path = uri.to_file_path().unwrap_or_default();
        let highlights = self.get_ast(&uri, |ast, visitor| {
            get_reference(ast, offset, visitor, true)
                .into_iter()
                .filter(|(reference_path, _)| *reference_path == path)
                .filter_map(|(_, r)| {
                    Some(DocumentHighlight {
                        range: Range::new(offset_to_position(r.span.start, &rope)?, offset_to_position(r.span.end, &rope)?),
                        kind: None,
                    })
                })
                .collect::<Vec<_>>()
        })?;
        if highlights.is_empty() {
            return Ok(None);
        }
        Ok(Some(highlights))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let Some(rope) = self.document_map.get(&uri) else {
            return Ok(None);
        };
        let completions = self.get_ast(&uri, |ast, visitor| {
            let offset = position_to_offset(&rope, position)?;
            let line = line_before_cursor(&rope, position)?;
            let completions = get_completion(ast, visitor, &line, offset);

            Some(completions)
        })?;
        Ok(completions.map(CompletionResponse::Array))
    }

    async fn signature_help(&self, params: SignatureHelpParams) -> Result<Option<SignatureHelp>> {
        let uri = params.text_document_position_params.text_document.uri;
        let position = params.text_document_position_params.position;
        let Some(rope) = self.document_map.get(&uri) else {
            return Ok(None);
        };
        self.get_ast(&uri, |_ast, visitor| {
            let line = line_before_cursor(&rope, position)?;
            get_signature_help(&line, visitor)
        })
    }

    async fn semantic_tokens_full(&self, params: SemanticTokensParams) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let Some(rope) = self.document_map.get(&uri) else {
            return Ok(None);
        };
        if !self.workspace_map.contains_key(&uri) && !self.ast_map.lock().unwrap().contains_key(&uri) {
            return Ok(None);
        }
        let source = rope.to_string();

        if self.workspace_map.contains_key(&uri) {
            let workspace = self.workspace.lock().map_err(|_| tower_lsp::jsonrpc::Error::internal_error())?;
            return self.get_ast(&uri, |ast, visitor| {
                Some(SemanticTokensResult::Tokens(SemanticTokens {
                    result_id: None,
                    data: get_semantic_tokens(ast, visitor, &rope, &source, &workspace),
                }))
            });
        }

        self.get_ast(&uri, |ast, visitor| {
            let mut workspace = Workspace::default();
            workspace.compiler = Some(icy_board_engine::compiler::workspace::CompilerData {
                language_version: Some(ast.language_version),
                defines: None,
            });
            Some(SemanticTokensResult::Tokens(SemanticTokens {
                result_id: None,
                data: get_semantic_tokens(ast, visitor, &rope, &source, &workspace),
            }))
        })
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;
        let Some(rope) = self.document_map.get(&uri) else {
            return Ok(None);
        };
        let Some(offset) = position_to_offset(&rope, params.text_document_position.position) else {
            return Ok(None);
        };

        self.client.log_message(MessageType::INFO, format!("OFFSET {offset}!")).await;

        let reference_list = self.get_ast(&uri, |ast: &Ast, visitor| get_reference(&ast, offset, visitor, true))?;

        self.client.log_message(MessageType::INFO, format!("got {} refs!", reference_list.len())).await;

        let list: Vec<Location> = reference_list
            .into_iter()
            .filter_map(|(path, reference)| self.location(&path, reference.span))
            .collect();
        if list.is_empty() { Ok(None) } else { Ok(Some(list)) }
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;
        let Some(rope) = self.document_map.get(&uri) else {
            return Ok(None);
        };
        let Some(offset) = position_to_offset(&rope, params.text_document_position.position) else {
            return Ok(None);
        };

        self.client.log_message(MessageType::INFO, format!("OFFSET {offset}!")).await;

        let reference_list = self.get_ast(&uri, |ast: &Ast, visitor| get_reference(&ast, offset, visitor, true))?;

        self.client.log_message(MessageType::INFO, format!("got {} refs!", reference_list.len())).await;

        let mut map: HashMap<Url, Vec<TextEdit>> = HashMap::new();
        for (path, reference) in reference_list {
            let Some(location) = self.location(&path, reference.span) else {
                continue;
            };
            map.entry(location.uri)
                .or_default()
                .push(TextEdit::new(location.range, params.new_name.clone()));
        }
        if map.is_empty() { Ok(None) } else { Ok(Some(WorkspaceEdit::new(map))) }
    }

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {
        self.client.log_message(MessageType::INFO, "configuration changed!").await;
    }

    async fn did_change_workspace_folders(&self, _params: DidChangeWorkspaceFoldersParams) {
        self.client.log_message(MessageType::INFO, "workspace folders changed!").await;
    }

    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
        self.client.log_message(MessageType::INFO, "watched files have changed!").await;
    }

    async fn formatting(&self, params: DocumentFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        self.format(&params.text_document.uri)
    }

    async fn range_formatting(&self, params: DocumentRangeFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        // The formatter reads the whole tree; only what falls into the selection is handed back.
        let selection = params.range;
        let Some(edits) = self.format(&params.text_document.uri)? else {
            return Ok(None);
        };
        Ok(Some(
            edits
                .into_iter()
                .filter(|edit| edit.range.start >= selection.start && edit.range.end <= selection.end)
                .collect(),
        ))
    }
}

fn quick_fix(title: &str, target_uri: Url, diagnostic: Diagnostic, edits: Vec<TextEdit>, preferred: bool) -> CodeActionOrCommand {
    let edit = WorkspaceEdit {
        changes: Some(HashMap::from([(target_uri, edits)])),
        ..WorkspaceEdit::default()
    };
    CodeActionOrCommand::CodeAction(CodeAction {
        title: title.to_string(),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: Some(vec![diagnostic]),
        edit: Some(edit),
        is_preferred: Some(preferred),
        ..CodeAction::default()
    })
}

/// The last language PCBoard actually released - 340 never left beta - and it
/// still takes END as a statement and braces as parentheses.
const LAST_LEGACY_LANGUAGE_VERSION: u16 = 330;

/// True for a name that can be called without arguments, so that adding `()` cannot
/// turn a report into a different one.
fn takes_no_arguments(name: &str, semantic_visitor: &SemanticVisitor) -> bool {
    if let Some(container) = semantic_visitor
        .function_containers
        .iter()
        .find(|container| container.name.eq_ignore_ascii_case(name))
    {
        return matches!(&container.functions, FunctionDeclaration::Function(declaration) if declaration.get_parameters().is_empty());
    }
    let definitions = FunctionDefinition::get_function_definitions(name);
    !definitions.is_empty()
        && definitions
            .iter()
            .all(|index| matches!(FUNCTION_DEFINITIONS[*index].signature, FunctionSignature::FixedParameters(0)))
}

fn with_trailing_blanks(rope: &Rope, range: Range) -> Range {
    let Some(mut end) = position_to_offset(rope, range.end) else {
        return range;
    };
    while rope.get_char(end).is_some_and(|ch| ch == ' ' || ch == '\t') {
        end += 1;
    }
    match offset_to_position(end, rope) {
        Some(end) => Range::new(range.start, end),
        None => range,
    }
}

struct DuplicateFieldVisitor {
    offset: usize,
    range: Option<core::ops::Range<usize>>,
}

impl AstVisitor<()> for DuplicateFieldVisitor {
    fn visit_record_literal_expression(&mut self, record: &RecordLiteralExpression) {
        let fields = record.get_fields();
        if let Some(index) = fields.iter().position(|field| field.get_identifier_token().span.start == self.offset) {
            let start = fields[index].get_identifier_token().span.start;
            let end = fields[index].get_value().get_span().end;
            // The separator has to go with the field, whichever side of it holds one.
            self.range = Some(if let Some(next) = fields.get(index + 1) {
                start..next.get_identifier_token().span.start
            } else if index > 0 {
                fields[index - 1].get_value().get_span().end..end
            } else {
                record.get_lbrace_token().span.end..record.get_rbrace_token().span.start
            });
            return;
        }
        for field in fields {
            field.get_value().visit(self);
        }
    }
}

fn duplicate_field_edit(ast: &Ast, rope: &Rope, offset: usize) -> Option<TextEdit> {
    let mut visitor = DuplicateFieldVisitor { offset, range: None };
    ast.visit(&mut visitor);
    let range = visitor.range?;
    Some(TextEdit::new(
        Range::new(offset_to_position(range.start, rope)?, offset_to_position(range.end, rope)?),
        String::new(),
    ))
}

fn language_version_directive_edit(rope: &Rope, version: u16) -> Option<TextEdit> {
    for line_index in 0..rope.len_lines() {
        let line = rope.get_line(line_index)?.to_string();
        let Some(directive) = line.trim_start().strip_prefix(';') else {
            continue;
        };
        let directive = directive.trim_start();
        if !directive.get(..12).is_some_and(|name| name.eq_ignore_ascii_case("$LANGVERSION")) {
            continue;
        }
        let line_start = rope.try_line_to_char(line_index).ok()?;
        let length = line.trim_end_matches(['\r', '\n']).chars().count();
        let start = offset_to_position(line_start, rope)?;
        let end = offset_to_position(line_start + length, rope)?;
        return Some(TextEdit::new(Range::new(start, end), format!(";$LANGVERSION {version}")));
    }
    // The directive only counts when nothing else comes before it.
    let at = Position::new(0, 0);
    Some(TextEdit::new(Range::new(at, at), format!(";$LANGVERSION {version}\n")))
}

struct TextDocumentItem {
    uri: Url,
    text: String,
}

impl Backend {
    fn location(&self, path: &std::path::Path, span: std::ops::Range<usize>) -> Option<Location> {
        let uri = Url::from_file_path(path).ok()?;
        let rope = if let Some(document) = self.document_map.get(&uri) {
            document.clone()
        } else {
            let data = std::fs::read(path).ok()?;
            Rope::from_str(&read_data_with_encoding_detection(&data).ok()?)
        };
        Some(Location::new(
            uri,
            Range::new(offset_to_position(span.start, &rope)?, offset_to_position(span.end, &rope)?),
        ))
    }

    fn format(&self, uri: &Url) -> Result<Option<Vec<TextEdit>>> {
        let Some(rope) = self.document_map.get(uri) else {
            return Ok(None);
        };
        let mut result = self.get_ast(uri, |ast, _| {
            let mut backend = VSCodeFormattingBackend {
                edits: Vec::new(),
                rope: &rope,
            };
            let options = self.workspace.lock().unwrap().formatting().clone();
            let mut visitor: FormattingVisitor<'_> = FormattingVisitor::new(&mut backend, &options);
            visitor.format(ast);
            backend.edits
        })?;
        result.sort_by(|a, b| b.range.start.cmp(&a.range.start));
        Ok(Some(result))
    }

    fn load_workspace(&self, roo_path: PathBuf) {
        let ws_file = roo_path.join("ppl.toml");
        if ws_file.exists() {
            if let Ok(ws) = Workspace::load(ws_file) {
                let errors = Arc::new(Mutex::new(ErrorReporter::default()));
                let registry = UserTypeRegistry::icy_board_registry();
                let mut sources = Vec::new();
                for file in ws.files() {
                    let Ok(data) = std::fs::read(&file) else {
                        continue;
                    };
                    let Ok(content) = read_data_with_encoding_detection(&data) else {
                        continue;
                    };
                    preparse_type_declarations(file.clone(), errors.clone(), &content, &registry, Encoding::Utf8, &ws);
                    sources.push((file, content));
                }

                let mut asts = Vec::new();
                for (file, content) in sources {
                    let ast = parse_ast_with_predeclared_types(file.clone(), errors.clone(), &content, &registry, Encoding::Utf8, &ws);
                    asts.push(ast);
                }

                let mut semantic_visitor = SemanticVisitor::new(&ws, errors, registry);
                for ast in asts {
                    ast.visit(&mut semantic_visitor);
                    if let Ok(uri) = Url::from_file_path(&ast.file_name) {
                        self.workspace_map.insert(uri, ast);
                    }
                }
                semantic_visitor.finish();

                let mut state = self.workspace.lock().unwrap();
                let _ = mem::replace(&mut *state, ws);
            }
        }
    }

    pub fn get_ast<T>(&self, uri: &Url, f: impl FnOnce(&Ast, &SemanticVisitor) -> T) -> Result<T> {
        if let Some(ast) = self.workspace_map.get(uri) {
            return Ok(f(&ast, &self.workspace_visitor.lock().unwrap()));
        }

        if let Some(result) = self.ast_map.lock().unwrap().get(uri) {
            Ok(f(&result.0, &result.1))
        } else {
            Err(tower_lsp::jsonrpc::Error::internal_error())
        }
    }

    async fn on_change(&self, params: TextDocumentItem) {
        let rope: Rope = ropey::Rope::from_str(&params.text);
        let uri = params.uri;
        self.document_map.insert(uri.clone(), rope.clone());
        self.client.publish_diagnostics(uri.clone(), Vec::new(), None).await;

        if self.workspace_map.get(&uri).is_some() {
            let semantic_visitor = {
                let workspace = self.workspace.lock().unwrap();
                let errors = Arc::new(Mutex::new(ErrorReporter::default()));
                let registry = UserTypeRegistry::icy_board_registry();
                let mut sources = Vec::new();

                for file in workspace.files() {
                    let Ok(cur_uri) = Url::from_file_path(&file) else {
                        continue;
                    };
                    let content = if uri == cur_uri {
                        params.text.clone()
                    } else if let Some(document) = self.document_map.get(&cur_uri) {
                        document.to_string()
                    } else {
                        let Ok(data) = std::fs::read(&file) else {
                            continue;
                        };
                        let Ok(content) = read_data_with_encoding_detection(&data) else {
                            continue;
                        };
                        content
                    };
                    preparse_type_declarations(file.clone(), errors.clone(), &content, &registry, Encoding::Utf8, &workspace);
                    sources.push((file, cur_uri, content));
                }

                let mut asts = Vec::new();
                for (file, cur_uri, content) in sources {
                    let ast = parse_ast_with_predeclared_types(file, errors.clone(), &content, &registry, Encoding::Utf8, &workspace);
                    asts.push((cur_uri, ast));
                }

                let mut semantic_visitor = SemanticVisitor::new(&workspace, errors, registry);
                for (cur_uri, ast) in asts {
                    semantic_visitor.errors.lock().unwrap().set_file_name(&ast.file_name);
                    ast.visit(&mut semantic_visitor);
                    self.workspace_map.insert(cur_uri, ast);
                }
                semantic_visitor.finish();
                semantic_visitor
            };
            self.add_diagnostics(&semantic_visitor).await;
            {
                let mut state: std::sync::MutexGuard<'_, SemanticVisitor> = self.workspace_visitor.lock().unwrap();
                let _ = mem::replace(&mut *state, semantic_visitor);
            }
        } else {
            let reg: UserTypeRegistry = UserTypeRegistry::icy_board_registry();
            let errors = Arc::new(Mutex::new(ErrorReporter::default()));
            let Ok(path) = uri.to_file_path() else {
                return;
            };
            preparse_type_declarations(path.clone(), errors.clone(), &params.text, &reg, Encoding::Utf8, &Workspace::default());
            let ast = parse_ast_with_predeclared_types(path, errors.clone(), &params.text, &reg, Encoding::Utf8, &Workspace::default());

            let mut semantic_visitor = SemanticVisitor::new(&Workspace::default(), errors, reg);
            ast.visit(&mut semantic_visitor);
            semantic_visitor.finish();

            self.add_diagnostics(&semantic_visitor).await;

            self.ast_map.lock().unwrap().insert(uri, (ast, semantic_visitor));
        }
    }

    async fn add_diagnostics(&self, semantic_visitor: &SemanticVisitor) {
        let mut diagnostics: HashMap<Url, Vec<Diagnostic>> = HashMap::new();
        // Every file that was looked at gets an answer, so that a report which no
        // longer applies is taken back rather than left standing.
        for uri in self.workspace_map.iter().map(|entry| entry.key().clone()) {
            diagnostics.insert(uri, Vec::new());
        }
        for uri in self.ast_map.lock().unwrap().keys() {
            diagnostics.insert(uri.clone(), Vec::new());
        }

        let mut ropes: HashMap<Url, Rope> = HashMap::new();
        {
            let reporter = semantic_visitor.errors.lock().unwrap();
            for (report, severity) in [(&reporter.errors, DiagnosticSeverity::ERROR), (&reporter.warnings, DiagnosticSeverity::WARNING)] {
                for err in report {
                    let Ok(uri) = Url::from_file_path(err.file_name.clone()) else {
                        continue;
                    };
                    if !ropes.contains_key(&uri) {
                        let rope = if let Some(document) = self.document_map.get(&uri) {
                            document.clone()
                        } else if let Ok(Ok(text)) = std::fs::read(&err.file_name).map(|data| read_data_with_encoding_detection(&data)) {
                            Rope::from_str(&text)
                        } else {
                            continue;
                        };
                        ropes.insert(uri.clone(), rope);
                    }
                    let rope = &ropes[&uri];

                    let start_position = offset_to_position(err.span.start, rope).unwrap_or(Position::new(0, 0));
                    let end_position = offset_to_position(err.span.end, rope).unwrap_or(Position::new(0, 0));
                    let mut diag = Diagnostic::new_simple(Range::new(start_position, end_position), format!("{}", err.error));
                    diag.severity = Some(severity);
                    diag.source = Some("ppl".to_string());
                    let (code, data) = diagnostic_details(&*err.error, rope, err.span.start, semantic_visitor);
                    diag.code = code;
                    diag.data = data;
                    if matches!(
                        diag.code.as_ref(),
                        Some(NumberOrString::String(code)) if matches!(code.as_str(), "ppl.unused-label" | "ppl.unused-routine" | "ppl.unused-variable")
                    ) {
                        diag.tags = Some(vec![DiagnosticTag::UNNECESSARY]);
                    }
                    diagnostics.entry(uri).or_default().push(diag);
                }
            }
        }

        for (uri, diagnostics) in diagnostics {
            self.client.publish_diagnostics(uri, diagnostics, None).await;
        }
    }
}

fn diagnostic_details(
    error: &(dyn std::error::Error + Send + Sync + 'static),
    rope: &Rope,
    start: usize,
    semantic_visitor: &SemanticVisitor,
) -> (Option<NumberOrString>, Option<Value>) {
    let mut data = None;
    let code = if let Some(warning) = error.downcast_ref::<ParserWarningType>() {
        match warning {
            ParserWarningType::ProcedureClosedWithEndFunc => "ppl.procedure-closed-with-endfunc",
            ParserWarningType::FunctionClosedWithEndProc => "ppl.function-closed-with-endproc",
            ParserWarningType::NextIdentifierInvalid(expected, _) => {
                data = Some(serde_json::json!({"replacement": expected.to_string()}));
                "ppl.next-identifier-mismatch"
            }
            _ => return (None, None),
        }
    } else if let Some(error) = error.downcast_ref::<LexingErrorType>() {
        match error {
            LexingErrorType::DontUseBraces => match rope.get_char(start) {
                Some('{') => "ppl.obsolete-brace-open",
                Some('}') => "ppl.obsolete-brace-close",
                _ => return (None, None),
            },
            LexingErrorType::PowWillGetRemoved => "ppl.obsolete-pow",
            _ => return (None, None),
        }
    } else if let Some(warning) = error.downcast_ref::<CompilationWarningType>() {
        match warning {
            CompilationWarningType::UnusedLabel(_) => "ppl.unused-label",
            _ => return (None, None),
        }
    } else if let Some(error) = error.downcast_ref::<CompilationErrorType>() {
        match error {
            CompilationErrorType::UnusedVariable(_) => "ppl.unused-variable",
            CompilationErrorType::UnusedFunction(_) => "ppl.unused-routine",
            CompilationErrorType::MissingImplementation(_) => "ppl.missing-implementation",
            CompilationErrorType::VariableNotFound(unknown) => {
                // A label is no name an expression could have meant.
                let names = semantic_visitor
                    .references
                    .iter()
                    .filter(|(kind, _)| !matches!(kind, ReferenceType::Label(_)))
                    .filter_map(|(_, references)| references.declaration.as_ref().map(|(_, declaration)| declaration.token.as_str()));
                let Some(replacement) = closest_name(unknown, names) else {
                    return (None, None);
                };
                data = Some(serde_json::json!({"replacement": replacement}));
                "ppl.unknown-identifier"
            }
            CompilationErrorType::EnumMemberNotFound(enum_name, unknown) => {
                let Some(definition) = semantic_visitor
                    .type_registry
                    .enums()
                    .into_iter()
                    .find(|definition| definition.name.eq_ignore_ascii_case(enum_name))
                else {
                    return (None, None);
                };
                let Some(replacement) = closest_name(unknown, definition.variants.iter().map(|(name, _)| name.as_str())) else {
                    return (None, None);
                };
                data = Some(serde_json::json!({"replacement": replacement}));
                "ppl.unknown-enum-member"
            }
            CompilationErrorType::UnknownRecordLiteralField(variable_type, unknown) | CompilationErrorType::RecordMemberNotFound(variable_type, unknown) => {
                let VariableType::UserData(type_id) = variable_type else {
                    return (None, None);
                };
                let Some(definition) = semantic_visitor.type_registry.get_user_type_from_id(*type_id) else {
                    return (None, None);
                };
                let Some(replacement) = closest_name(unknown, definition.fields.iter().map(|(name, _)| name.as_str())) else {
                    return (None, None);
                };
                data = Some(serde_json::json!({"replacement": replacement}));
                "ppl.unknown-record-field"
            }
            CompilationErrorType::TooManyArguments(_, expected) => {
                data = Some(serde_json::json!({"expected": expected}));
                "ppl.too-many-arguments"
            }
            CompilationErrorType::RecordLiteralNeedsRuntime(required) | CompilationErrorType::RoutineReferenceNeedsRuntime(required) => {
                data = Some(serde_json::json!({"required": required}));
                "ppl.runtime-too-old"
            }
            CompilationErrorType::DuplicateRecordLiteralField(_) => "ppl.duplicate-record-field",
            CompilationErrorType::FunctionUsedAsVariable(name) if takes_no_arguments(name, semantic_visitor) => "ppl.routine-needs-call",
            _ => return (None, None),
        }
    } else if let Some(error) = error.downcast_ref::<icy_board_engine::parser::ParserErrorType>() {
        match error {
            icy_board_engine::parser::ParserErrorType::TooManyArguments(_, _, expected) if *expected >= 0 => {
                data = Some(serde_json::json!({"expected": expected}));
                "ppl.too-many-arguments"
            }
            icy_board_engine::parser::ParserErrorType::TypeNeedsNewerRuntime(required) => {
                data = Some(serde_json::json!({"required": required}));
                "ppl.runtime-too-old"
            }
            icy_board_engine::parser::ParserErrorType::StatementVersionNotSupported(_, required, _)
            | icy_board_engine::parser::ParserErrorType::FunctionVersionNotSupported(_, required, _) => {
                data = Some(serde_json::json!({"required": required}));
                "ppl.language-version-too-old"
            }
            icy_board_engine::parser::ParserErrorType::EndIsNotAStatement => {
                data = Some(serde_json::json!({"legacy": LAST_LEGACY_LANGUAGE_VERSION}));
                "ppl.end-is-not-a-statement"
            }
            icy_board_engine::parser::ParserErrorType::VarNotAllowedInFunctions => "ppl.var-not-allowed",
            _ => return (None, None),
        }
    } else {
        return (None, None);
    };
    (Some(NumberOrString::String(code.to_string())), data)
}

fn closest_name<'a>(unknown: &str, candidates: impl Iterator<Item = &'a str>) -> Option<String> {
    let unknown = unknown.to_ascii_lowercase();
    let mut best: Option<(&str, usize)> = None;
    let mut tied = false;
    for candidate in candidates {
        let distance = edit_distance(&unknown, &candidate.to_ascii_lowercase());
        match best {
            None => {
                best = Some((candidate, distance));
                tied = false;
            }
            Some((_, best_distance)) if distance < best_distance => {
                best = Some((candidate, distance));
                tied = false;
            }
            Some((_, best_distance)) if distance == best_distance => tied = true,
            _ => {}
        }
    }
    let (candidate, distance) = best?;
    let allowed = 1.max(unknown.chars().count() / 3).min(3);
    (!tied && distance <= allowed).then(|| candidate.to_string())
}

fn edit_distance(left: &str, right: &str) -> usize {
    let mut previous: Vec<usize> = (0..=right.chars().count()).collect();
    for (left_index, left_char) in left.chars().enumerate() {
        let mut current = vec![left_index + 1];
        for (right_index, right_char) in right.chars().enumerate() {
            current.push(
                (current[right_index] + 1)
                    .min(previous[right_index + 1] + 1)
                    .min(previous[right_index] + usize::from(left_char != right_char)),
            );
        }
        previous = current;
    }
    previous.last().copied().unwrap_or_default()
}

struct ExcessArgumentsVisitor<'a> {
    offset: usize,
    expected: usize,
    range: Option<core::ops::Range<usize>>,
    source: &'a str,
}

impl ExcessArgumentsVisitor<'_> {
    fn call_range(&self, arguments: &[Expression], left: usize, right: usize) -> Option<core::ops::Range<usize>> {
        if arguments.len() <= self.expected {
            return None;
        }
        let start = if self.expected == 0 {
            left
        } else {
            arguments.get(self.expected - 1)?.get_span().end
        };
        let mut start = start;
        while start < right && self.source.get(start..)?.chars().next()?.is_whitespace() {
            start += self.source.get(start..)?.chars().next()?.len_utf8();
        }
        Some(start..right)
    }
}

impl AstVisitor<()> for ExcessArgumentsVisitor<'_> {
    fn visit_function_call_expression(&mut self, call: &FunctionCallExpression) {
        let span = call.get_expression().get_span();
        if span.start <= self.offset && self.offset <= span.end {
            self.range = self.call_range(call.get_arguments(), call.get_lpar_token().span.end, call.get_rpar_token().span.start);
            return;
        }
        walk_function_call_expression(self, call);
    }

    fn visit_procedure_call_statement(&mut self, call: &ProcedureCallStatement) {
        let span = &call.get_identifier_token().span;
        if span.start <= self.offset && self.offset <= span.end {
            self.range = self.call_range(call.get_arguments(), call.get_leftpar_token().span.end, call.get_rightpar_token().span.start);
            return;
        }
        walk_procedure_call_statement(self, call);
    }
}

fn excess_arguments_edit(ast: &Ast, rope: &Rope, offset: usize, expected: usize) -> Option<TextEdit> {
    let source = rope.to_string();
    let mut visitor = ExcessArgumentsVisitor {
        offset,
        expected,
        range: None,
        source: &source,
    };
    ast.visit(&mut visitor);
    let range = visitor.range?;
    Some(TextEdit::new(
        Range::new(offset_to_position(range.start, rope)?, offset_to_position(range.end, rope)?),
        String::new(),
    ))
}

#[derive(Clone, Copy)]
enum ManifestSetting {
    Runtime,
    LanguageVersion,
}

fn manifest_version_edit(path: &std::path::Path, setting: ManifestSetting, required: u16) -> Option<(Url, TextEdit)> {
    if path.as_os_str().is_empty() {
        return None;
    }
    let source = std::fs::read_to_string(path).ok()?;
    let rope = Rope::from_str(&source);
    let (section, key) = match setting {
        ManifestSetting::Runtime => ("package", "runtime"),
        ManifestSetting::LanguageVersion => ("compiler", "language_version"),
    };
    let mut section_line = None;
    let mut section_end = rope.len_lines();
    for line_index in 0..rope.len_lines() {
        let line = rope.get_line(line_index)?.to_string();
        let trimmed = line.trim();
        if trimmed == format!("[{section}]") {
            section_line = Some(line_index);
            continue;
        }
        if section_line.is_some() && trimmed.starts_with('[') && trimmed.ends_with(']') {
            section_end = line_index;
            break;
        }
        if section_line.is_some() {
            let Some((name, _)) = trimmed.split_once('=') else {
                continue;
            };
            if name.trim() == key {
                let equals = line.find('=')?;
                let value_start = equals + 1 + line[equals + 1..].len() - line[equals + 1..].trim_start().len();
                let value_end = line.trim_end_matches(['\r', '\n']).len();
                let line_start = rope.try_line_to_char(line_index).ok()?;
                let start = offset_to_position(line_start + line[..value_start].chars().count(), &rope)?;
                let end = offset_to_position(line_start + line[..value_end].chars().count(), &rope)?;
                return Some((Url::from_file_path(path).ok()?, TextEdit::new(Range::new(start, end), required.to_string())));
            }
        }
    }
    let (at, text) = if section_line.is_some() {
        (Position::new(section_end as u32, 0), format!("{key} = {required}\n"))
    } else {
        let at = offset_to_position(rope.len_chars(), &rope)?;
        let prefix = if source.is_empty() || source.ends_with('\n') { "" } else { "\n" };
        (at, format!("{prefix}\n[{section}]\n{key} = {required}\n"))
    };
    Some((Url::from_file_path(path).ok()?, TextEdit::new(Range::new(at, at), text)))
}

fn remove_line_edit(rope: &Rope, line: u32) -> TextEdit {
    let start = Position::new(line, 0);
    let end = if (line as usize) + 1 < rope.len_lines() {
        Position::new(line + 1, 0)
    } else {
        offset_to_position(rope.len_chars(), rope).unwrap_or(start)
    };
    TextEdit::new(Range::new(start, end), String::new())
}

fn remove_variable_edit(rope: &Rope, diagnostic: Range) -> Option<TextEdit> {
    let line_start = rope.try_line_to_char(diagnostic.start.line as usize).ok()?;
    let line = rope.get_line(diagnostic.start.line as usize)?.to_string();
    let identifier_start = position_to_offset(rope, diagnostic.start)?.checked_sub(line_start)?;
    let mut commas = Vec::new();
    let mut depth = 0usize;
    let mut quoted = false;
    for (offset, ch) in line.char_indices() {
        match ch {
            '"' => quoted = !quoted,
            '(' | '[' | '{' if !quoted => depth += 1,
            ')' | ']' | '}' if !quoted => depth = depth.saturating_sub(1),
            ',' if !quoted && depth == 0 => commas.push(offset),
            _ => {}
        }
    }
    let previous = commas.iter().copied().take_while(|comma| *comma < identifier_start).last();
    let next = commas.iter().copied().find(|comma| *comma > identifier_start);
    if previous.is_none() && next.is_none() {
        if line[identifier_start..].contains('=') {
            return None;
        }
        return Some(remove_line_edit(rope, diagnostic.start.line));
    }
    let (start_byte, mut end_byte) = match (previous, next) {
        (None, None) => unreachable!(),
        (None, Some(next)) => (identifier_start, next + 1),
        (Some(previous), _) => (previous, next.unwrap_or_else(|| line.trim_end_matches(['\r', '\n']).len())),
    };
    if line[start_byte..end_byte].contains('=') {
        return None;
    }
    if previous.is_none() {
        while line.as_bytes().get(end_byte).is_some_and(u8::is_ascii_whitespace) && !matches!(line.as_bytes()[end_byte], b'\r' | b'\n') {
            end_byte += 1;
        }
    }
    let start = offset_to_position(line_start + line[..start_byte].chars().count(), rope)?;
    let end = offset_to_position(line_start + line[..end_byte].chars().count(), rope)?;
    Some(TextEdit::new(Range::new(start, end), String::new()))
}

fn remove_routine_edits(rope: &Rope, diagnostic: Range) -> Option<Vec<TextEdit>> {
    let declaration = rope.get_line(diagnostic.start.line as usize)?.to_string();
    let declaration = declaration.trim_start();
    let header = declaration.get(7..)?.trim_start();
    let (keyword, terminator) = if header.get(..8).is_some_and(|prefix| prefix.eq_ignore_ascii_case("FUNCTION")) {
        ("FUNCTION", "ENDFUNC")
    } else if header.get(..9).is_some_and(|prefix| prefix.eq_ignore_ascii_case("PROCEDURE")) {
        ("PROCEDURE", "ENDPROC")
    } else {
        return None;
    };
    let name_start = keyword.len();
    let name = header[name_start..].trim_start().split(['(', ' ', '\r', '\n']).next()?;
    let mut implementation_start = None;
    for line in 0..rope.len_lines() {
        let text = rope.get_line(line)?.to_string();
        let trimmed = text.trim_start();
        if trimmed.get(..keyword.len()).is_some_and(|prefix| prefix.eq_ignore_ascii_case(keyword)) {
            let candidate = trimmed[keyword.len()..].trim_start().split(['(', ' ', '\r', '\n']).next();
            if candidate.is_some_and(|candidate| candidate.eq_ignore_ascii_case(name)) {
                implementation_start = Some(line as u32);
                break;
            }
        }
    }
    let start_line = implementation_start?;
    let mut end_line = start_line + 1;
    while (end_line as usize) < rope.len_lines() {
        let text = rope.get_line(end_line as usize)?.to_string();
        if text
            .trim_start()
            .get(..terminator.len())
            .is_some_and(|token| token.eq_ignore_ascii_case(terminator))
        {
            end_line += 1;
            return Some(vec![
                remove_line_edit(rope, diagnostic.start.line),
                TextEdit::new(Range::new(Position::new(start_line, 0), Position::new(end_line, 0)), String::new()),
            ]);
        }
        end_line += 1;
    }
    None
}

fn implementation_stub_edit(rope: &Rope, line: u32) -> Option<TextEdit> {
    let declaration = rope.get_line(line as usize)?.to_string();
    let declaration = declaration.trim_end_matches(['\r', '\n']).trim_start();
    let header = declaration.get(7..)?.trim_start();
    if !declaration.get(..7)?.eq_ignore_ascii_case("DECLARE") {
        return None;
    }
    let terminator = if header.get(..8).is_some_and(|prefix| prefix.eq_ignore_ascii_case("FUNCTION")) {
        "ENDFUNC"
    } else if header.get(..9).is_some_and(|prefix| prefix.eq_ignore_ascii_case("PROCEDURE")) {
        "ENDPROC"
    } else {
        return None;
    };
    let mut insertion_line = line + 1;
    while let Some(next) = rope.get_line(insertion_line as usize) {
        let text = next.to_string();
        let trimmed = text.trim_start();
        if trimmed.is_empty() || trimmed.starts_with(';') || trimmed.get(..7).is_some_and(|prefix| prefix.eq_ignore_ascii_case("DECLARE")) {
            insertion_line += 1;
        } else {
            break;
        }
    }
    let at = if insertion_line as usize >= rope.len_lines() {
        offset_to_position(rope.len_chars(), rope)?
    } else {
        Position::new(insertion_line, 0)
    };
    Some(TextEdit::new(Range::new(at, at), format!("\n{header}\n{terminator}\n")))
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| Backend {
        client,
        ast_map: Arc::new(Mutex::new(HashMap::new())),
        document_map: DashMap::new(),
        workspace: Mutex::new(Workspace::default()),
        workspace_visitor: Mutex::new(SemanticVisitor::new(
            &Workspace::default(),
            Arc::new(Mutex::new(ErrorReporter::default())),
            UserTypeRegistry::default(),
        )),
        workspace_map: DashMap::new(),
    })
    .finish();

    serde_json::json!({"test": 20});
    Server::new(stdin, stdout, socket).serve(service).await;
}

struct TooltipVisitor {
    pub tooltip: Option<Hover>,
    pub offset: usize,
}

impl AstVisitor<()> for TooltipVisitor {
    fn visit_constant_expression(&mut self, const_expr: &ConstantExpression) {
        if const_expr.get_constant_token().span.contains(&self.offset) {
            match const_expr.get_constant_value() {
                Constant::Builtin(c) => {
                    self.tooltip = get_const_hover(c);
                }
                _ => {}
            }
        }
    }

    fn visit_variable_declaration_statement(&mut self, var_decl: &icy_board_engine::ast::VariableDeclarationStatement) {
        if var_decl.get_type_token().span.contains(&self.offset) {
            self.tooltip = get_type_hover(var_decl.get_variable_type());
        }
        walk_variable_declaration_statement(self, var_decl);
    }

    fn visit_parameter_specifier(&mut self, param: &icy_board_engine::ast::ParameterSpecifier) {
        match param {
            ParameterSpecifier::Variable(param) => {
                if param.get_type_token().span.contains(&self.offset) {
                    self.tooltip = get_type_hover(param.get_variable_type());
                }
            }
            ParameterSpecifier::Function(f) => {
                if f.get_return_type_token().span.contains(&self.offset) {
                    self.tooltip = get_type_hover(f.get_return_type());
                }
                for p in f.get_parameters() {
                    p.visit(self);
                }
            }
            ParameterSpecifier::Procedure(f) => {
                for p in f.get_parameters() {
                    p.visit(self);
                }
            }
        }
    }

    fn visit_function_declaration(&mut self, func_decl: &icy_board_engine::ast::FunctionDeclarationAstNode) {
        if func_decl.get_return_type_token().span.contains(&self.offset) {
            self.tooltip = get_type_hover(func_decl.get_return_type());
        }
        walk_function_declaration(self, func_decl);
    }

    fn visit_function_implementation(&mut self, function: &icy_board_engine::ast::FunctionImplementation) {
        if function.get_return_type_token().span.contains(&self.offset) {
            self.tooltip = get_type_hover(function.get_return_type());
        }
        walk_function_implementation(self, function);
    }

    fn visit_predefined_call_statement(&mut self, call: &icy_board_engine::ast::PredefinedCallStatement) {
        if call.get_identifier_token().span.contains(&self.offset) {
            self.tooltip = get_statement_hover(call.get_func());
        }
        walk_predefined_call_statement(self, call);
    }

    fn visit_function_call_expression(&mut self, call: &icy_board_engine::ast::FunctionCallExpression) {
        icy_board_engine::ast::walk_function_call_expression(self, call);
        if let Expression::Identifier(identifier) = call.get_expression() {
            if identifier.get_identifier_token().span.contains(&self.offset) {
                let predef = FunctionDefinition::get_function_definitions(identifier.get_identifier());
                for p in predef {
                    if FUNCTION_DEFINITIONS[p].parameter_count() == call.get_arguments().len() {
                        self.tooltip = get_function_hover(&FUNCTION_DEFINITIONS[p]);
                        return;
                    }
                }
            }
        }
    }
}

fn get_tooltip(ast: &Ast, offset: usize) -> Option<Hover> {
    let mut visitor = TooltipVisitor { tooltip: None, offset };
    ast.visit(&mut visitor);
    visitor.tooltip
}
