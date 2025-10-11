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
use icy_board_engine::executable::{FUNCTION_DEFINITIONS, FunctionDefinition, LAST_PPLC};
use icy_board_engine::formatting::FormattingVisitor;
use icy_board_engine::icy_board::read_data_with_encoding_detection;
use icy_board_engine::parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast};
use icy_board_engine::semantic::SemanticVisitor;
use ppl_language_server::completion::get_completion;
use ppl_language_server::documentation::{get_const_hover, get_function_hover, get_statement_hover, get_type_hover};
use ppl_language_server::formatting::VSCodeFormattingBackend;
use ppl_language_server::jump_definition::get_definition;
use ppl_language_server::reference::get_reference;
use ppl_language_server::semantic_token::{LEGEND_TYPE, semantic_token_from_ast};
use ppl_language_server::{ImCompleteSemanticToken, offset_to_position};
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
    semantic_token_map: DashMap<Url, Vec<ImCompleteSemanticToken>>,
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
                    commands: vec!["ppl-lsp-vscode.run".to_string()],
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
        self.semantic_token_map.remove(&uri);
    }

    async fn hover(&self, params: HoverParams) -> Result<Option<Hover>> {
        let _ = params;
        let uri = params.text_document_position_params.text_document.uri;
        self.get_ast(&uri, |ast, _semantic_visitor| {
            let rope = self.document_map.get(&uri)?;

            let position = params.text_document_position_params.position;
            let char = rope.try_line_to_char(position.line as usize).ok()?;
            let offset = char + position.character as usize;

            get_tooltip(&ast, offset)
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

    async fn semantic_tokens_full(&self, params: SemanticTokensParams) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri;
        let mut im_complete_tokens = self.semantic_token_map.get_mut(&uri).unwrap();
        let rope = self.document_map.get(&uri).unwrap();
        let tokens = self.get_ast(&uri, |ast, _| {
            let extends_tokens = semantic_token_from_ast(ast);
            im_complete_tokens.extend(extends_tokens);
            im_complete_tokens.sort_by(|a: &ImCompleteSemanticToken, b| a.start.cmp(&b.start));
            let mut pre_line = 0;
            let mut pre_start: u32 = 0;
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
        })?;
        if let Some(semantic_token) = tokens {
            return Ok(Some(SemanticTokensResult::Tokens(SemanticTokens {
                result_id: None,
                data: semantic_token,
            })));
        }
        Ok(None)
    }

    async fn semantic_tokens_range(&self, params: SemanticTokensRangeParams) -> Result<Option<SemanticTokensRangeResult>> {
        let uri = params.text_document.uri;
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
        let uri = &params.text_document.uri;
        if self.get_ast(uri, |_, _| {}).is_err() {}
        let inlay_hint_list = Vec::new();
        Ok(Some(inlay_hint_list))
    }

    async fn completion(&self, params: CompletionParams) -> Result<Option<CompletionResponse>> {
        let uri = params.text_document_position.text_document.uri;
        let position = params.text_document_position.position;
        let rope = self.document_map.get(&uri).unwrap();
        let completions = self.get_ast(&uri, |ast, _| {
            let char = rope.try_line_to_char(position.line as usize).ok()?;
            let offset = char + position.character as usize;
            let completions = get_completion(&ast, offset);

            Some(completions)
        })?;
        Ok(completions.map(CompletionResponse::Array))
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

    async fn range_formatting(&self, params: DocumentRangeFormattingParams) -> Result<Option<Vec<TextEdit>>> {
        let uri = params.text_document.uri;
        let Some(rope) = self.document_map.get(&uri) else {
            return Ok(None);
        };
        self.client.log_message(MessageType::INFO, "format !").await;
        let mut result = self.get_ast(&uri, |ast, _| {
            let mut backend = VSCodeFormattingBackend {
                edits: Vec::new(),
                rope: &rope,
            };

            let options = self.workspace.lock().unwrap().formatting().clone();
            let mut visitor: FormattingVisitor<'_> = FormattingVisitor::new(&mut backend, &options);
            ast.visit(&mut visitor);
            backend.edits
        })?;
        result.sort_by(|a, b| b.range.start.cmp(&a.range.start));
        Ok(Some(result))
    }

    async fn execute_command(&self, params: ExecuteCommandParams) -> Result<Option<Value>> {
        match params.command.as_str() {
            "ppl-lsp-vscode.run" => {
                let ws_file: PathBuf = self.workspace.lock().unwrap().file_name.clone();
                if ws_file.exists() {
                    self.client.log_message(MessageType::INFO, "compile workspace!").await;

                    let output = process::Command::new("pplc").arg(ws_file).output().expect("failed to execute process");
                    if let Ok(output) = String::from_utf8(output.stdout) {
                        self.client.log_message(MessageType::INFO, format!("{}", output)).await;
                    }
                    let out_file: String = self.workspace.lock().unwrap().package.name().to_string();
                    let target_file = self.workspace.lock().unwrap().target_path(LAST_PPLC).join(out_file).with_extension("ppe");
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
    fn load_workspace(&self, roo_path: PathBuf) {
        let ws_file = roo_path.join("ppl.toml");
        if ws_file.exists() {
            if let Ok(ws) = Workspace::load(ws_file) {
                let mut semantic_visitor = SemanticVisitor::new(&ws, Arc::new(Mutex::new(ErrorReporter::default())), UserTypeRegistry::default());
                for file in ws.files() {
                    let content = read_data_with_encoding_detection(&std::fs::read(&file).unwrap()).unwrap();
                    let ast = parse_ast(
                        file.clone(),
                        semantic_visitor.errors.clone(),
                        &content,
                        &UserTypeRegistry::default(),
                        Encoding::Utf8,
                        &ws,
                    );
                    ast.visit(&mut semantic_visitor);
                    self.workspace_map.insert(Url::from_file_path(file).unwrap(), ast);
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
            let mut semantic_visitor = SemanticVisitor::new(
                &self.workspace.lock().unwrap(),
                Arc::new(Mutex::new(ErrorReporter::default())),
                UserTypeRegistry::default(),
            );
            let files = self.workspace.lock().unwrap().files();
            for file in files {
                let name = file.to_string_lossy().to_string();
                let cur_uri = Url::from_file_path(name).unwrap();

                if uri == cur_uri {
                    let ast = parse_ast(
                        file.clone(),
                        semantic_visitor.errors.clone(),
                        &params.text,
                        &UserTypeRegistry::default(),
                        Encoding::Utf8,
                        &self.workspace.lock().unwrap(),
                    );
                    let semantic_tokens: Vec<ImCompleteSemanticToken> = semantic_token_from_ast(&ast);
                    self.semantic_token_map.insert(cur_uri.clone(), semantic_tokens);
                    self.workspace_map.insert(cur_uri.clone(), ast);
                } else if self.workspace_map.get(&cur_uri).is_none() {
                    self.client.publish_diagnostics(cur_uri.clone(), Vec::new(), None).await;
                    let content = read_data_with_encoding_detection(&std::fs::read(&file).unwrap()).unwrap();
                    let ast = parse_ast(
                        file.clone(),
                        semantic_visitor.errors.clone(),
                        &content,
                        &UserTypeRegistry::default(),
                        Encoding::Utf8,
                        &self.workspace.lock().unwrap(),
                    );
                    let semantic_tokens = semantic_token_from_ast(&ast);
                    self.semantic_token_map.insert(cur_uri.clone(), semantic_tokens);
                    self.workspace_map.insert(cur_uri.clone(), ast);
                }

                if let Some(ast) = self.workspace_map.get(&cur_uri) {
                    semantic_visitor.errors.lock().unwrap().set_file_name(&ast.file_name);
                    ast.visit(&mut semantic_visitor);
                }
            }
            semantic_visitor.finish();
            self.add_diagnostics(&semantic_visitor).await;
            {
                let mut state: std::sync::MutexGuard<'_, SemanticVisitor> = self.workspace_visitor.lock().unwrap();
                let _ = mem::replace(&mut *state, semantic_visitor);
            }
        } else {
            let reg: UserTypeRegistry = UserTypeRegistry::default();
            let errors = Arc::new(Mutex::new(ErrorReporter::default()));
            let path = uri.to_file_path().unwrap();
            let ast = parse_ast(path, errors.clone(), &params.text, &reg, Encoding::Utf8, &Workspace::default());

            let mut semantic_visitor = SemanticVisitor::new(&Workspace::default(), errors, reg);
            ast.visit(&mut semantic_visitor);
            semantic_visitor.finish();

            let semantic_tokens: Vec<ImCompleteSemanticToken> = semantic_token_from_ast(&ast);
            self.semantic_token_map.insert(uri.clone(), semantic_tokens);

            self.add_diagnostics(&semantic_visitor).await;

            self.ast_map.lock().unwrap().insert(uri, (ast, semantic_visitor));
            // self.client
            //     .log_message(MessageType::INFO, &format!("{:?}", semantic_tokens))
            //     .await;
        }
    }

    async fn add_diagnostics(&self, semantic_visitor: &SemanticVisitor) {
        let mut diagnostics = HashMap::new();
        for err in &semantic_visitor.errors.lock().unwrap().errors {
            let uri = Url::from_file_path(err.file_name.clone()).unwrap();
            let Some(rope) = self.document_map.get(&uri) else {
                continue;
            };

            let start_position = offset_to_position(err.span.start, &rope).unwrap_or(Position::new(0, 0));
            let end_position = offset_to_position(err.span.end, &rope).unwrap_or(Position::new(0, 0));
            let mut diag = Diagnostic::new_simple(Range::new(start_position, end_position), format!("{}", err.error));
            //diag.source = Some(uri.clone());
            diag.severity = Some(DiagnosticSeverity::ERROR);
            if !diagnostics.contains_key(&uri) {
                diagnostics.insert(uri.clone(), Vec::new());
            }
            diagnostics.get_mut(&uri).unwrap().push(diag);
        }
        for err in &semantic_visitor.errors.lock().unwrap().warnings {
            let uri = Url::from_file_path(err.file_name.clone()).unwrap();
            let Some(rope) = self.document_map.get(&uri) else {
                continue;
            };
            let start_position = offset_to_position(err.span.start, &rope).unwrap_or(Position::new(0, 0));
            let end_position = offset_to_position(err.span.end, &rope).unwrap_or(Position::new(0, 0));
            let mut diag = Diagnostic::new_simple(Range::new(start_position, end_position), format!("{}", err.error));
            //diag.source = Some(uri.clone());
            diag.severity = Some(DiagnosticSeverity::WARNING);
            if !diagnostics.contains_key(&uri) {
                diagnostics.insert(uri.clone(), Vec::new());
            }
            diagnostics.get_mut(&uri).unwrap().push(diag);
        }

        for (uri, diagnostics) in diagnostics {
            self.client.log_message(MessageType::INFO, format!("Add Diagnostics for {} !", uri)).await;
            for d in &diagnostics {
                self.client.log_message(MessageType::INFO, format!("{} !", d.message)).await;
            }

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
        semantic_token_map: DashMap::new(),
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
