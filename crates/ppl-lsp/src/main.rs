use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, Mutex};
use std::{env, mem, process};

use dashmap::DashMap;
use icy_board_engine::ast::{
    Ast, AstVisitor, Constant, ConstantExpression, Expression, ParameterSpecifier, walk_function_declaration, walk_function_implementation,
    walk_predefined_call_statement, walk_variable_declaration_statement,
};
use icy_board_engine::compiler::workspace::Workspace;
use icy_board_engine::executable::{FUNCTION_DEFINITIONS, FunctionDefinition, LAST_PPE_RUNTIME};
use icy_board_engine::formatting::FormattingVisitor;
use icy_board_engine::icy_board::read_data_with_encoding_detection;
use icy_board_engine::parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast, parse_ast_with_predeclared_types, preparse_type_declarations};
use icy_board_engine::semantic::SemanticVisitor;
use icyboard_ppl::completion::get_completion;
use icyboard_ppl::document_symbol::get_document_symbols;
use icyboard_ppl::documentation::{get_const_hover, get_function_hover, get_statement_hover, get_type_hover};
use icyboard_ppl::formatting::VSCodeFormattingBackend;
use icyboard_ppl::hover::get_user_hover;
use icyboard_ppl::jump_definition::get_definition;
use icyboard_ppl::reference::get_reference;
use icyboard_ppl::signature_help::get_signature_help;
use icyboard_ppl::{line_before_cursor, offset_to_position};
use ropey::Rope;
use serde_json::Value;
use tower_lsp::jsonrpc::Result;
use tower_lsp::lsp_types::*;
use tower_lsp::{Client, LanguageServer, LspService, Server};

struct Backend {
    client: Client,

    cur_process: Mutex<Option<process::Child>>,

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

                execute_command_provider: Some(ExecuteCommandOptions {
                    commands: vec!["icyboard-ppl.run".to_string()],
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
                document_formatting_provider: Some(OneOf::Left(true)),
                document_range_formatting_provider: Some(OneOf::Left(true)),
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
        self.on_change(TextDocumentItem {
            uri: params.text_document.uri,
            text: std::mem::take(&mut params.content_changes[0].text),
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
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let uri = params.text_document_position_params.text_document.uri;
        self.get_ast(&uri, |ast, visitor| {
            let rope = self.document_map.get(&uri)?;

            let position = params.text_document_position_params.position;
            let char = rope.try_line_to_char(position.line as usize).ok()?;
            let offset = char + position.character as usize;

            get_tooltip(ast, offset).or_else(|| get_user_hover(ast, visitor, offset))
        })
    }

    async fn goto_definition(&self, params: GotoDefinitionParams) -> Result<Option<GotoDefinitionResponse>> {
        let uri2 = params.text_document_position_params.text_document.uri.clone();
        let uri = params.text_document_position_params.text_document.uri;
        let res = self.get_ast(&uri, |ast, visitor| {
            let rope = self.document_map.get(&uri2)?;

            let position = params.text_document_position_params.position;
            let char = rope.try_line_to_char(position.line as usize).ok()?;
            let offset = char + position.character as usize;
            if let Some((path, r)) = get_definition(&ast, visitor, offset) {
                let start_position = offset_to_position(r.span.start, &rope)?;
                let end_position = offset_to_position(r.span.end, &rope)?;
                let range = Range::new(start_position, end_position);
                if let Ok(path) = Url::from_file_path(&path) {
                    return Some(GotoDefinitionResponse::Scalar(Location::new(path, range)));
                }
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
        let position = params.text_document_position_params.position;
        let Ok(line_start) = rope.try_line_to_char(position.line as usize) else {
            return Ok(None);
        };
        let offset = line_start + position.character as usize;

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
        let rope = self.document_map.get(&uri).unwrap();
        let completions = self.get_ast(&uri, |ast, visitor| {
            let char = rope.try_line_to_char(position.line as usize).ok()?;
            let offset = char + position.character as usize;
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

    async fn references(&self, params: ReferenceParams) -> Result<Option<Vec<Location>>> {
        let uri = params.text_document_position.text_document.uri;

        let rope = self.document_map.get(&uri).unwrap();
        let position = params.text_document_position.position;
        let char = rope.try_line_to_char(position.line as usize).ok().unwrap();
        let offset: usize = char + position.character as usize;

        self.client.log_message(MessageType::INFO, format!("OFFSET {offset}!")).await;

        let reference_list = self.get_ast(&uri, |ast: &Ast, visitor| get_reference(&ast, offset, visitor, true))?;

        self.client.log_message(MessageType::INFO, format!("got {} refs!", reference_list.len())).await;

        if !reference_list.is_empty() {
            let mut list = Vec::new();
            let mut rope_map = HashMap::new();
            for (path, r) in reference_list {
                let uri2 = Url::from_file_path(&path).ok().unwrap();
                let start_position;
                let end_position;
                if let Some(rope) = self.document_map.get(&uri2) {
                    start_position = offset_to_position(r.span.start, &rope).unwrap();
                    end_position = offset_to_position(r.span.end, &rope).unwrap();
                } else {
                    if !rope_map.contains_key(&path) {
                        let content = read_data_with_encoding_detection(&std::fs::read(&path).unwrap()).unwrap();
                        rope_map.insert(path.clone(), Rope::from_str(&content));
                    }
                    let rope = rope_map.get(&path).unwrap();
                    start_position = offset_to_position(r.span.start, &rope).unwrap();
                    end_position = offset_to_position(r.span.end, &rope).unwrap();
                };
                list.push(Location::new(uri2, Range::new(start_position, end_position)));
            }
            Ok(Some(list))
        } else {
            Ok(None)
        }
    }

    async fn rename(&self, params: RenameParams) -> Result<Option<WorkspaceEdit>> {
        let uri = params.text_document_position.text_document.uri;

        let rope = self.document_map.get(&uri).unwrap();
        let position = params.text_document_position.position;
        let char = rope.try_line_to_char(position.line as usize).ok().unwrap();
        let offset: usize = char + position.character as usize;

        self.client.log_message(MessageType::INFO, format!("OFFSET {offset}!")).await;

        let reference_list = self.get_ast(&uri, |ast: &Ast, visitor| get_reference(&ast, offset, visitor, true))?;

        self.client.log_message(MessageType::INFO, format!("got {} refs!", reference_list.len())).await;

        let new_name = params.new_name;
        if !reference_list.is_empty() {
            let mut map = HashMap::new();
            let mut rope_map = HashMap::new();
            for (path, r) in reference_list {
                let uri2 = Url::from_file_path(&path).ok().unwrap();
                let start_position;
                let end_position;
                if let Some(rope) = self.document_map.get(&uri2) {
                    start_position = offset_to_position(r.span.start, &rope).unwrap();
                    end_position = offset_to_position(r.span.end, &rope).unwrap();
                } else {
                    if !rope_map.contains_key(&path) {
                        let content = read_data_with_encoding_detection(&std::fs::read(&path).unwrap()).unwrap();
                        rope_map.insert(path.clone(), Rope::from_str(&content));
                    }
                    let rope = rope_map.get(&path).unwrap();
                    start_position = offset_to_position(r.span.start, &rope).unwrap();
                    end_position = offset_to_position(r.span.end, &rope).unwrap();
                };

                if !map.contains_key(&uri2) {
                    map.insert(uri2.clone(), Vec::new());
                }
                map.get_mut(&uri2)
                    .unwrap()
                    .push(TextEdit::new(Range::new(start_position, end_position), new_name.clone()));
            }
            Ok(Some(WorkspaceEdit::new(map)))
        } else {
            Ok(None)
        }
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

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        match params.command.as_str() {
            "icyboard-ppl.run" => {
                let ws_file: PathBuf = self.workspace.lock().unwrap().file_name.clone();
                if ws_file.exists() {
                    self.client.log_message(MessageType::INFO, "compile workspace!").await;

                    let output = process::Command::new("pplc").arg(ws_file).output().expect("failed to execute process");
                    if let Ok(output) = String::from_utf8(output.stdout) {
                        self.client.log_message(MessageType::INFO, format!("{}", output)).await;
                    }
                    let out_file: String = self.workspace.lock().unwrap().package.name().to_string();
                    let target_file = self
                        .workspace
                        .lock()
                        .unwrap()
                        .target_path(LAST_PPE_RUNTIME)
                        .join(out_file)
                        .with_extension("ppe");
                    self.client.log_message(MessageType::INFO, format!("Execute:{}", target_file.display())).await;

                    let shell = env::var("SHELL").unwrap_or("sh".to_string());
                    if let Ok(process) = process::Command::new(shell)
                        .arg("-c")
                        .arg(format!("\"icboard --ppe {}\"", target_file.display()))
                        .spawn()
                    {
                        let mut state: std::sync::MutexGuard<'_, Option<process::Child>> = self.cur_process.lock().unwrap();
                        if let Some(mut child) = mem::replace(&mut *state, Some(process)) {
                            child.kill().unwrap();
                        }
                    }
                } else {
                    self.client.log_message(MessageType::ERROR, "no workspace open!").await;
                }
            }
            _ => {
                self.client.log_message(MessageType::INFO, "unknown command!").await;
            }
        }

        Ok(None)
    }
}

struct TextDocumentItem {
    uri: Url,
    text: String,
}

impl Backend {
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
                    let content = read_data_with_encoding_detection(&std::fs::read(&file).unwrap()).unwrap();
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
                    self.workspace_map.insert(Url::from_file_path(&ast.file_name).unwrap(), ast);
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
                    let cur_uri = Url::from_file_path(&file).unwrap();
                    let content = if uri == cur_uri {
                        params.text.clone()
                    } else if let Some(document) = self.document_map.get(&cur_uri) {
                        document.to_string()
                    } else {
                        read_data_with_encoding_detection(&std::fs::read(&file).unwrap()).unwrap()
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
            let path = uri.to_file_path().unwrap();
            let ast = parse_ast(path, errors.clone(), &params.text, &reg, Encoding::Utf8, &Workspace::default());

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
                    diagnostics.entry(uri).or_default().push(diag);
                }
            }
        }

        for (uri, diagnostics) in diagnostics {
            self.client.publish_diagnostics(uri, diagnostics, None).await;
        }
    }
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
        cur_process: Mutex::new(None),
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
