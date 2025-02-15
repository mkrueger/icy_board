use std::collections::HashMap;
use std::path::PathBuf;

use dashmap::DashMap;
use icy_board_engine::ast::{
    walk_function_declaration, walk_function_implementation, walk_predefined_call_statement, walk_variable_declaration_statement, Ast, AstVisitor, Constant,
    ConstantExpression, Expression,
};
use icy_board_engine::executable::{FunctionDefinition, FUNCTION_DEFINITIONS, LAST_PPLC};
use icy_board_engine::parser::{parse_ast, Encoding, UserTypeRegistry};
use icy_board_engine::semantic::SemanticVisitor;
use ppl_language_server::completion::get_completion;
use ppl_language_server::documentation::{get_const_hover, get_function_hover, get_statement_hover, get_type_hover};
use ppl_language_server::jump_definition::get_definition;
use ppl_language_server::reference::get_reference;
use ppl_language_server::semantic_token::{semantic_token_from_ast, LEGEND_TYPE};
use ppl_language_server::ImCompleteSemanticToken;
use ropey::Rope;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

#[derive(Debug)]
struct Backend {
    client: Client,
    ast_map: DashMap<String, Ast>,
    document_map: DashMap<String, Rope>,
    semantic_token_map: DashMap<String, Vec<ImCompleteSemanticToken>>,
}

#[tower_lsp::async_trait]
impl LanguageServer for Backend {
    async fn initialize(&self, _: InitializeParams) -> Result<InitializeResult> {
        Ok(InitializeResult {
            server_info: None,
            offset_encoding: None,
            capabilities: ServerCapabilities {
                inlay_hint_provider: Some(OneOf::Left(true)),
                text_document_sync: Some(TextDocumentSyncCapability::Kind(TextDocumentSyncKind::FULL)),
                hover_provider: Some(HoverProviderCapability::Simple(true)),

                completion_provider: Some(CompletionOptions {
                    resolve_provider: Some(false),
                    trigger_characters: None,
                    work_done_progress_options: Default::default(),
                    all_commit_characters: None,
                    completion_item: None,
                }),

                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["pplc".to_string(), "pplx".to_string()],
                    work_done_progress_options: Default::default(),
                }),

                workspace: Some(WorkspaceServerCapabilities {
                    workspace_folders: Some(WorkspaceFoldersServerCapabilities {
                        supported: Some(true),
                        change_notifications: Some(OneOf::Left(true)),
                    }),
                    file_operations: None,
                }),
                semantic_tokens_provider: Some(SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(
                    SemanticTokensRegistrationOptions {
                        text_document_registration_options: {
                            TextDocumentRegistrationOptions {
                                document_selector: Some(vec![DocumentFilter {
                                    language: Some("ppl".to_string()),
                                    scheme: Some("file".to_string()),
                                    pattern: None,
                                }]),
                            }
                        },
                        semantic_tokens_options: SemanticTokensOptions {
                            work_done_progress_options: WorkDoneProgressOptions::default(),
                            legend: SemanticTokensLegend {
                                token_types: LEGEND_TYPE.into(),
                                token_modifiers: vec![],
                            },
                            range: Some(true),
                            full: Some(SemanticTokensFullOptions::Bool(true)),
                        },
                        static_registration_options: StaticRegistrationOptions::default(),
                    },
                )),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
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
        self.client.log_message(MessageType::INFO, "file opened!").await;
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri,
            text: params.text_document.text,
            version: params.text_document.version,
        })
        .await
    }

    async fn did_change(&self, mut params: DidChangeTextDocumentParams) {
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri,
            text: std::mem::take(&mut params.content_changes[0].text),
            version: params.text_document.version,
        })
        .await
    }

    async fn did_save(&self, _: DidSaveTextDocumentParams) {
        self.client.log_message(MessageType::INFO, "file saved!").await;
    }
    async fn did_close(&self, params: DidCloseTextDocumentParams) {
        let uri = params.text_document.uri;

        self.client.publish_diagnostics(uri.clone(), Vec::new(), None).await;
        self.ast_map.remove(uri.as_str());
        self.semantic_token_map.remove(uri.to_string().as_str());
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let _ = params;
        let result = async {
            let uri = params.text_document_position_params.text_document.uri;
            let ast = self.ast_map.get(uri.as_str())?;
            let rope = self.document_map.get(uri.as_str())?;

            let position = params.text_document_position_params.position;
            let char = rope.try_line_to_char(position.line as usize).ok()?;
            let offset = char + position.character as usize;

            get_tooltip(&ast, offset)
        }
        .await;
        Ok(result)
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        let definition = async {
            let uri = params.text_document_position_params.text_document.uri;
            let ast = self.ast_map.get(uri.as_str())?;
            let rope = self.document_map.get(uri.as_str())?;

            let position = params.text_document_position_params.position;
            let char = rope.try_line_to_char(position.line as usize).ok()?;
            let offset = char + position.character as usize;
            let span = get_definition(&ast, offset);
            self.client.log_message(MessageType::INFO, &format!("{:?}, ", span)).await;
            span.and_then(|r| {
                let start_position = offset_to_position(r.span.start, &rope)?;
                let end_position = offset_to_position(r.span.end, &rope)?;

                let range = Range::new(start_position, end_position);

                Some(GotoDefinitionResponse::Scalar(Location::new(uri, range)))
            })
        }
        .await;
        Ok(definition)
    }

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let reference_list = || -> Option<Vec<Location>> {
            let uri = params.text_document_position.text_document.uri;
            let ast = self.ast_map.get(&uri.to_string())?;
            let rope = self.document_map.get(&uri.to_string())?;

            let position = params.text_document_position.position;
            let char = rope.try_line_to_char(position.line as usize).ok()?;
            let offset = char + position.character as usize;
            let reference_list = get_reference(&ast, offset, true);
            let ret = reference_list
                .into_iter()
                .filter_map(|r| {
                    let start_position = offset_to_position(r.span.start, &rope)?;
                    let end_position = offset_to_position(r.span.end, &rope)?;

                    let range = Range::new(start_position, end_position);

                    Some(Location::new(uri.clone(), range))
                })
                .collect::<Vec<_>>();
            Some(ret)
        }();
        Ok(reference_list)
    }

    async fn semantic_tokens_full(&self, params: SemanticTokensParams) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri.to_string();
        self.client.log_message(MessageType::LOG, "semantic_token_full").await;
        let semantic_tokens = || -> Option<Vec<SemanticToken>> {
            let mut im_complete_tokens = self.semantic_token_map.get_mut(&uri)?;
            let rope = self.document_map.get(&uri)?;
            let ast = self.ast_map.get(&uri)?;
            let extends_tokens = semantic_token_from_ast(&ast);
            im_complete_tokens.extend(extends_tokens);
            im_complete_tokens.sort_by(|a, b| a.start.cmp(&b.start));
            let mut pre_line = 0;
            let mut pre_start = 0;
            let semantic_tokens = im_complete_tokens
                .iter()
                .filter_map(|token| {
                    let line = rope.try_char_to_line(token.start).ok()? as u32;
                    let first = rope.try_line_to_char(line as usize).ok()? as u32;
                    let start = token.start as u32 - first;
                    let delta_line = line - pre_line;
                    let delta_start = if delta_line == 0 { start - pre_start } else { start };
                    let ret = Some(SemanticToken {
                        delta_line,
                        delta_start,
                        length: token.length as u32,
                        token_type: token.token_type as u32,
                        token_modifiers_bitset: 0,
                    });
                    pre_line = line;
                    pre_start = start;
                    ret
                })
                .collect::<Vec<_>>();
            Some(semantic_tokens)
        }();
        if let Some(semantic_token) = semantic_tokens {
            return Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
                result_id: None,
                data: semantic_token,
            })));
        }
        Ok(None)
    }

    async fn semantic_tokens_range(&self, params: SemanticTokensRangeParams) -> Result<Option<SemanticTokensRangeResult>> {
        let uri = params.text_document.uri.to_string();
        let semantic_tokens = || -> Option<Vec<SemanticToken>> {
            let im_complete_tokens = self.semantic_token_map.get(&uri)?;
            let rope = self.document_map.get(&uri)?;
            let mut pre_line = 0;
            let mut pre_start = 0;
            let semantic_tokens = im_complete_tokens
                .iter()
                .filter_map(|token| {
                    let line = rope.try_char_to_line(token.start).ok()? as u32;
                    let first = rope.try_line_to_char(line as usize).ok()? as u32;
                    let start = token.start as u32 - first;
                    let ret = Some(SemanticToken {
                        delta_line: line.saturating_sub(pre_line),
                        delta_start: if start >= pre_start { start - pre_start } else { start },
                        length: token.length as u32,
                        token_type: token.token_type as u32,
                        token_modifiers_bitset: 0,
                    });
                    pre_line = line;
                    pre_start = start;
                    ret
                })
                .collect::<Vec<_>>();
            Some(semantic_tokens)
        }();
        if let Some(semantic_token) = semantic_tokens {
            return Ok(Some(SemanticTokensRangeResult::Tokens(SemanticTokens {
                result_id: None,
                data: semantic_token,
            })));
        }
        Ok(None)
    }

    async fn inlay_hint(&self, params: tower_lsp::lsp_types::InlayHintParams) -> Result<Option<Vec<InlayHint>>> {
        self.client.log_message(MessageType::INFO, "inlay hint").await;
        let uri = &params.text_document.uri;
        if let Some(_program) = self.ast_map.get(uri.as_str()) {}
        let inlay_hint_list = Vec::new();
        Ok(Some(inlay_hint_list))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let completions = || -> Option<Vec<CompletionItem>> {
            let rope = self.document_map.get(&uri.to_string())?;
            let ast = self.ast_map.get(&uri.to_string())?;
            let char = rope.try_line_to_char(position.line as usize).ok()?;
            let offset = char + position.character as usize;
            let completions = get_completion(&ast, offset);

            Some(completions)
        }();
        Ok(completions.map(CompletionResponse::Array))
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let workspace_edit = || -> Option<WorkspaceEdit> {
            let uri = params.text_document_position.text_document.uri;
            let ast = self.ast_map.get(&uri.to_string())?;
            let rope = self.document_map.get(&uri.to_string())?;

            let position = params.text_document_position.position;
            let char = rope.try_line_to_char(position.line as usize).ok()?;
            let offset = char + position.character as usize;
            let reference_list = get_reference(&ast, offset, true);
            let new_name = params.new_name;
            if !reference_list.is_empty() {
                let edit_list = reference_list
                    .into_iter()
                    .filter_map(|r| {
                        let start_position = offset_to_position(r.span.start, &rope)?;
                        let end_position = offset_to_position(r.span.end, &rope)?;
                        Some(TextEdit::new(Range::new(start_position, end_position), new_name.clone()))
                    })
                    .collect::<Vec<_>>();
                let mut map = HashMap::new();
                map.insert(uri, edit_list);
                let workspace_edit = WorkspaceEdit::new(map);
                Some(workspace_edit)
            } else {
                None
            }
        }();
        Ok(workspace_edit)
    }

    async fn did_change_configuration(&self, _: DidChangeConfigurationParams) {
        self.client.log_message(MessageType::INFO, "configuration changed!").await;
    }

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {
        self.client.log_message(MessageType::INFO, "workspace folders changed!").await;
    }

    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
        self.client.log_message(MessageType::INFO, "watched files have changed!").await;
    }

    async fn execute_command(&self, _: ExecuteCommandParams) -> Result<Option<Value>> {
        self.client.log_message(MessageType::INFO, "command executed!").await;

        match self.client.apply_edit(WorkspaceEdit::default()).await {
            Ok(res) if res.applied => self.client.log_message(MessageType::INFO, "applied").await,
            Ok(_) => self.client.log_message(MessageType::INFO, "rejected").await,
            Err(err) => self.client.log_message(MessageType::ERROR, err).await,
        }

        Ok(None)
    }
}

#[derive(Debug, Deserialize, Serialize)]
struct InlayHintParams {
    path: String,
}

struct TextDocumentItem {
    uri: Url,
    text: String,
    version: i32,
}

impl Backend {
    async fn on_change(&self, params: TextDocumentItem) {
        let rope = ropey::Rope::from_str(&params.text);
        let uri = params.uri.to_string();
        self.document_map.insert(uri.clone(), rope.clone());
        let reg = UserTypeRegistry::default();
        let (ast, errors) = parse_ast(PathBuf::from(uri.clone()), &params.text, &reg, Encoding::Utf8, LAST_PPLC);

        let mut semantic_visitor = SemanticVisitor::new(LAST_PPLC, errors, &reg);

        ast.visit(&mut semantic_visitor);

        let semantic_tokens = semantic_token_from_ast(&ast);

        let mut diagnostics = Vec::new();

        for err in &semantic_visitor.errors.lock().unwrap().errors {
            let start_position = offset_to_position(err.span.start, &rope).unwrap_or(Position::new(0, 0));
            let end_position = offset_to_position(err.span.end, &rope).unwrap_or(Position::new(0, 0));
            let mut diag = Diagnostic::new_simple(Range::new(start_position, end_position), format!("{}", err.error));
            diag.severity = Some(DiagnosticSeverity::ERROR);
            diagnostics.push(diag);
        }
        for err in &semantic_visitor.errors.lock().unwrap().warnings {
            let start_position = offset_to_position(err.span.start, &rope).unwrap_or(Position::new(0, 0));
            let end_position = offset_to_position(err.span.end, &rope).unwrap_or(Position::new(0, 0));
            let mut diag = Diagnostic::new_simple(Range::new(start_position, end_position), format!("{}", err.error));
            diag.source = Some(uri.clone());
            diag.severity = Some(DiagnosticSeverity::WARNING);
            diagnostics.push(diag);
        }

        self.client.publish_diagnostics(params.uri.clone(), diagnostics, Some(params.version)).await;

        self.ast_map.insert(params.uri.to_string(), ast);
        // self.client
        //     .log_message(MessageType::INFO, &format!("{:?}", semantic_tokens))
        //     .await;
        self.semantic_token_map.insert(params.uri.to_string(), semantic_tokens);
    }
}

#[tokio::main]
async fn main() {
    env_logger::init();

    let stdin = tokio::io::stdin();
    let stdout = tokio::io::stdout();

    let (service, socket) = LspService::build(|client| Backend {
        client,
        ast_map: DashMap::new(),
        document_map: DashMap::new(),
        semantic_token_map: DashMap::new(),
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
        if param.get_type_token().span.contains(&self.offset) {
            self.tooltip = get_type_hover(param.get_variable_type());
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
                    if FUNCTION_DEFINITIONS[p].arg_descr as usize == call.get_arguments().len() {
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

fn offset_to_position(offset: usize, rope: &Rope) -> Option<Position> {
    let line = rope.try_char_to_line(offset).ok()?;
    let first_char_of_line = rope.try_line_to_char(line).ok()?;
    let column = offset - first_char_of_line;
    Some(Position::new(line as u32, column as u32))
}
