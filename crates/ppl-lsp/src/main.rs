use std::collections::HashMap;
use std::path::PathBuf;

use dashmap::DashMap;
use i18n_embed_fl::fl;
use icy_ppe::ast::{walk_function_declaration, walk_function_implementation, Ast, AstVisitor};
use icy_ppe::executable::{OpCode, VariableType, LAST_PPLC};
use icy_ppe::parser::{parse_ast, Encoding};
use icy_ppe::semantic::SemanticVisitor;
use ppl_language_server::completion::get_completion;
use ppl_language_server::jump_definition::get_definition;
use ppl_language_server::reference::get_reference;
use ppl_language_server::semantic_token::{semantic_token_from_ast, LEGEND_TYPE};
use ppl_language_server::{ImCompleteSemanticToken, LANGUAGE_LOADER};
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
                text_document_sync: Some(TextDocumentSyncCapability::Kind(
                    TextDocumentSyncKind::FULL,
                )),
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
                semantic_tokens_provider: Some(
                    SemanticTokensServerCapabilities::SemanticTokensRegistrationOptions(
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
                    ),
                ),
                definition_provider: Some(OneOf::Left(true)),
                references_provider: Some(OneOf::Left(true)),
                rename_provider: Some(OneOf::Left(true)),
                ..ServerCapabilities::default()
            },
        })
    }
    async fn initialized(&self, _: InitializedParams) {
        self.client
            .log_message(MessageType::INFO, "initialized!")
            .await;
    }

    async fn shutdown(&self) -> Result<()> {
        Ok(())
    }

    async fn did_open(&self, params: DidOpenTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file opened!")
            .await;
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
        self.client
            .log_message(MessageType::INFO, "file saved!")
            .await;
    }
    async fn did_close(&self, _: DidCloseTextDocumentParams) {
        self.client
            .log_message(MessageType::INFO, "file closed!")
            .await;
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

    async fn goto_definition(
        &self,
        params: GotoDefinitionParams,
    ) -> Result<Option<GotoDefinitionResponse>> {
        let definition = async {
            let uri = params.text_document_position_params.text_document.uri;
            let ast = self.ast_map.get(uri.as_str())?;
            let rope = self.document_map.get(uri.as_str())?;

            let position = params.text_document_position_params.position;
            let char = rope.try_line_to_char(position.line as usize).ok()?;
            let offset = char + position.character as usize;
            let span = get_definition(&ast, offset);
            self.client
                .log_message(MessageType::INFO, &format!("{:?}, ", span))
                .await;
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

    async fn semantic_tokens_full(
        &self,
        params: SemanticTokensParams,
    ) -> Result<Option<SemanticTokensResult>> {
        let uri = params.text_document.uri.to_string();
        self.client
            .log_message(MessageType::LOG, "semantic_token_full")
            .await;
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
                    let delta_start = if delta_line == 0 {
                        start - pre_start
                    } else {
                        start
                    };
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

    async fn semantic_tokens_range(
        &self,
        params: SemanticTokensRangeParams,
    ) -> Result<Option<SemanticTokensRangeResult>> {
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
                        delta_start: if start >= pre_start {
                            start - pre_start
                        } else {
                            start
                        },
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

    async fn inlay_hint(
        &self,
        params: tower_lsp::lsp_types::InlayHintParams,
    ) -> Result<Option<Vec<InlayHint>>> {
        self.client
            .log_message(MessageType::INFO, "inlay hint")
            .await;
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
                        Some(TextEdit::new(
                            Range::new(start_position, end_position),
                            new_name.clone(),
                        ))
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
        self.client
            .log_message(MessageType::INFO, "configuration changed!")
            .await;
    }

    async fn did_change_workspace_folders(&self, _: DidChangeWorkspaceFoldersParams) {
        self.client
            .log_message(MessageType::INFO, "workspace folders changed!")
            .await;
    }

    async fn did_change_watched_files(&self, _: DidChangeWatchedFilesParams) {
        self.client
            .log_message(MessageType::INFO, "watched files have changed!")
            .await;
    }

    async fn execute_command(&self, _: ExecuteCommandParams) -> Result<Option<Value>> {
        self.client
            .log_message(MessageType::INFO, "command executed!")
            .await;

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
        let (ast, errors) = parse_ast(PathBuf::from(uri), &params.text, Encoding::Utf8, LAST_PPLC);

        let mut semantic_visitor = SemanticVisitor::new(LAST_PPLC, errors);

        ast.visit(&mut semantic_visitor);

        let semantic_tokens = semantic_token_from_ast(&ast);

        let mut diagnostics = Vec::new();

        for err in &semantic_visitor.errors.lock().unwrap().errors {
            let start_position =
                offset_to_position(err.span.start, &rope).unwrap_or(Position::new(0, 0));
            let end_position =
                offset_to_position(err.span.end, &rope).unwrap_or(Position::new(0, 0));
            let mut diag = Diagnostic::new_simple(
                Range::new(start_position, end_position),
                format!("{}", err.error),
            );
            diag.severity = Some(DiagnosticSeverity::ERROR);
            diagnostics.push(diag);
        }
        for err in &semantic_visitor.errors.lock().unwrap().warnings {
            let start_position =
                offset_to_position(err.span.start, &rope).unwrap_or(Position::new(0, 0));
            let end_position =
                offset_to_position(err.span.end, &rope).unwrap_or(Position::new(0, 0));
            let mut diag = Diagnostic::new_simple(
                Range::new(start_position, end_position),
                format!("{}", err.error),
            );
            diag.severity = Some(DiagnosticSeverity::WARNING);
            diagnostics.push(diag);
        }

        self.client
            .publish_diagnostics(params.uri.clone(), diagnostics, Some(params.version))
            .await;

        self.ast_map.insert(params.uri.to_string(), ast);
        // self.client
        //     .log_message(MessageType::INFO, &format!("{:?}", semantic_tokens))
        //     .await;
        self.semantic_token_map
            .insert(params.uri.to_string(), semantic_tokens);
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
    fn visit_variable_declaration_statement(
        &mut self,
        var_decl: &icy_ppe::ast::VariableDeclarationStatement,
    ) {
        if var_decl.get_type_token().span.contains(&self.offset) {
            self.tooltip = get_type_hover(var_decl.get_variable_type());
        }
    }

    fn visit_parameter_specifier(&mut self, param: &icy_ppe::ast::ParameterSpecifier) {
        if param.get_type_token().span.contains(&self.offset) {
            self.tooltip = get_type_hover(param.get_variable_type());
        }
    }

    fn visit_function_declaration(&mut self, func_decl: &icy_ppe::ast::FunctionDeclarationAstNode) {
        if func_decl
            .get_return_type_token()
            .span
            .contains(&self.offset)
        {
            self.tooltip = get_type_hover(func_decl.get_return_type());
        }
        walk_function_declaration(self, func_decl);
    }

    fn visit_function_implementation(&mut self, function: &icy_ppe::ast::FunctionImplementation) {
        if function.get_return_type_token().span.contains(&self.offset) {
            self.tooltip = get_type_hover(function.get_return_type());
        }

        walk_function_implementation(self, function);
    }

    fn visit_predefined_call_statement(&mut self, call: &icy_ppe::ast::PredefinedCallStatement) {
        if call.get_identifier_token().span.contains(&self.offset) {
            self.tooltip = get_statement_hover(call.get_func().opcode);
        }
    }
}

fn get_type_hover(var_type: VariableType) -> Option<Hover> {
    match var_type {
        VariableType::Boolean => get_hint(fl!(LANGUAGE_LOADER, "hint-type-boolean")),
        VariableType::Unsigned => get_hint(fl!(LANGUAGE_LOADER, "hint-type-unsigned")),
        VariableType::Date => get_hint(fl!(LANGUAGE_LOADER, "hint-type-date")),
        VariableType::EDate => get_hint(fl!(LANGUAGE_LOADER, "hint-type-edate")),
        VariableType::Integer => get_hint(fl!(LANGUAGE_LOADER, "hint-type-integer")),
        VariableType::Money => get_hint(fl!(LANGUAGE_LOADER, "hint-type-money")),
        VariableType::Float => get_hint(fl!(LANGUAGE_LOADER, "hint-type-float")),
        VariableType::String => get_hint(fl!(LANGUAGE_LOADER, "hint-type-string")),
        VariableType::Time => get_hint(fl!(LANGUAGE_LOADER, "hint-type-time")),
        VariableType::Byte => get_hint(fl!(LANGUAGE_LOADER, "hint-type-byte")),
        VariableType::Word => get_hint(fl!(LANGUAGE_LOADER, "hint-type-word")),
        VariableType::SByte => get_hint(fl!(LANGUAGE_LOADER, "hint-type-sbyte")),
        VariableType::SWord => get_hint(fl!(LANGUAGE_LOADER, "hint-type-sword")),
        VariableType::BigStr => get_hint(fl!(LANGUAGE_LOADER, "hint-type-bigstr")),
        VariableType::Double => get_hint(fl!(LANGUAGE_LOADER, "hint-type-double")),
        VariableType::DDate => get_hint(fl!(LANGUAGE_LOADER, "hint-type-ddate")),
        _ => None,
    }
}

fn offset_to_position(offset: usize, rope: &Rope) -> Option<Position> {
    let line = rope.try_char_to_line(offset).ok()?;
    let first_char_of_line = rope.try_line_to_char(line).ok()?;
    let column = offset - first_char_of_line;
    Some(Position::new(line as u32, column as u32))
}

fn get_tooltip(ast: &Ast, offset: usize) -> Option<Hover> {
    let mut visitor = TooltipVisitor {
        tooltip: None,
        offset,
    };
    ast.visit(&mut visitor);
    visitor.tooltip
}

fn get_statement_hover(opcode: OpCode) -> Option<Hover> {
    match opcode {
        OpCode::END => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-end")),
        OpCode::CLS => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-cls")),
        OpCode::CLREOL => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-clreol")),
        OpCode::MORE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-more")),
        OpCode::WAIT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-wait")),
        OpCode::COLOR => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-color")),
        OpCode::GOTO => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-goto")),
        OpCode::LET => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-let")),
        OpCode::PRINT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-print")),
        OpCode::PRINTLN => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-println")),
        OpCode::CONFFLAG => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-confflag")),
        OpCode::CONFUNFLAG => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-confunflag")),
        OpCode::DISPFILE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dispfile")),
        OpCode::INPUT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-input")),
        OpCode::FCREATE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fcreate")),
        OpCode::FOPEN => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fopen")),
        OpCode::FAPPEND => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fappend")),
        OpCode::FCLOSE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fclose")),
        OpCode::FGET => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fget")),
        OpCode::FPUT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fput")),
        OpCode::FPUTLN => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fputln")),
        OpCode::RESETDISP => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-resetdisp")),
        OpCode::STARTDISP => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-startdisp")),
        OpCode::FPUTPAD => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fputpad")),
        OpCode::HANGUP => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-hangup")),
        OpCode::GETUSER => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-getuser")),
        OpCode::PUTUSER => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-putuser")),
        OpCode::DEFCOLOR => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-defcolor")),
        OpCode::DELETE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-delete")),
        OpCode::DELUSER => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-deluser")),
        OpCode::ADJTIME => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-adjtime")),
        OpCode::LOG => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-log")),
        OpCode::INPUTSTR => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-inputstr")),
        OpCode::INPUTYN => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-inputyn")),
        OpCode::INPUTMONEY => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-inputmoney")),
        OpCode::INPUTINT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-inputint")),
        OpCode::INPUTCC => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-inputcc")),
        OpCode::INPUTDATE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-inputdate")),
        OpCode::INPUTTIME => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-inputtime")),
        OpCode::GOSUB => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-gosub")),
        OpCode::RETURN => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-return")),
        OpCode::PROMPTSTR => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-promptstr")),
        OpCode::DTRON => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dtron")),
        OpCode::DTROFF => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dtroff")),
        OpCode::CDCHKON => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-cdchkon")),
        OpCode::CDCHKOFF => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-cdchkoff")),
        OpCode::DELAY => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-delay")),
        OpCode::SENDMODEM => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-sendmodem")),
        OpCode::INC => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-inc")),
        OpCode::DEC => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dec")),
        OpCode::NEWLINE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-newline")),
        OpCode::NEWLINES => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-newlines")),
        OpCode::TOKENIZE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-tokenize")),
        OpCode::GETTOKEN => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-gettoken")),
        OpCode::SHELL => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-shell")),
        OpCode::DISPTEXT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-disptext")),
        OpCode::STOP => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-stop")),
        OpCode::INPUTTEXT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-inputtext")),
        OpCode::BEEP => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-beep")),
        OpCode::PUSH => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-push")),
        OpCode::POP => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-pop")),
        OpCode::KBDSTUFF => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-kbdstuff")),
        OpCode::CALL => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-call")),
        OpCode::JOIN => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-join")),
        OpCode::QUEST => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-quest")),
        OpCode::BLT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-blt")),
        OpCode::DIR => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dir")),
        OpCode::KBDFILE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-kbdfile")),
        OpCode::BYE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-bye")),
        OpCode::GOODBYE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-goodbye")),
        OpCode::BROADCAST => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-broadcast")),
        OpCode::WAITFOR => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-waitfor")),
        OpCode::KBDCHKON => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-kbdchkon")),
        OpCode::KBDCHKOFF => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-kbdchkoff")),
        OpCode::OPTEXT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-optext")),
        OpCode::DISPSTR => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dispstr")),
        OpCode::RDUNET => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-rdunet")),
        OpCode::WRUNET => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-wrunet")),
        OpCode::DOINTR => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dointr")),
        OpCode::VARSEG => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-varseg")),
        OpCode::VAROFF => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-varoff")),
        OpCode::POKEB => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-pokeb")),
        OpCode::POKEW => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-pokew")),
        OpCode::VARADDR => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-varaddr")),
        OpCode::ANSIPOS => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-ansipos")),
        OpCode::BACKUP => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-backup")),
        OpCode::FORWARD => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-forward")),
        OpCode::FRESHLINE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-freshline")),
        OpCode::WRUSYS => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-wrusys")),
        OpCode::RDUSYS => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-rdusys")),
        OpCode::NEWPWD => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-newpwd")),
        OpCode::OPENCAP => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-opencap")),
        OpCode::CLOSECAP => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-closecap")),
        OpCode::MESSAGE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-message")),
        OpCode::SAVESCRN => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-savescrn")),
        OpCode::RESTSCRN => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-restscrn")),
        OpCode::SOUND => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-sound")),
        OpCode::CHAT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-chat")),
        OpCode::SPRINT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-sprint")),
        OpCode::SPRINTLN => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-sprintln")),
        OpCode::MPRINT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-mprint")),
        OpCode::MPRINTLN => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-mprintln")),
        OpCode::RENAME => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-rename")),
        OpCode::FREWIND => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-frewind")),
        OpCode::POKEDW => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-pokedw")),
        OpCode::DBGLEVEL => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dbglevel")),
        OpCode::SHOWON => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-showon")),
        OpCode::SHOWOFF => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-showoff")),
        OpCode::PAGEON => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-pageon")),
        OpCode::PAGEOFF => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-pageoff")),
        OpCode::FSEEK => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fseek")),
        OpCode::FFLUSH => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fflush")),
        OpCode::FREAD => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fread")),
        OpCode::FWRITE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fwrite")),
        OpCode::FDEFIN => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fdefin")),
        OpCode::FDEFOUT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fdefout")),
        OpCode::FDGET => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fdget")),
        OpCode::FDPUT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fdput")),
        OpCode::FDPUTLN => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fdputln")),
        OpCode::FDPUTPAD => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fdputpad")),
        OpCode::FDREAD => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fdread")),
        OpCode::FDWRITE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fdwrite")),
        OpCode::ADJBYTES => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-adjbytes")),
        OpCode::KBDSTRING => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-kbdstring")),
        OpCode::ALIAS => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-alias")),
        OpCode::REDIM => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-redim")),
        OpCode::APPEND => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-append")),
        OpCode::COPY => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-copy")),
        OpCode::KBDFLUSH => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-kbdflush")),
        OpCode::MDMFLUSH => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-mdmflush")),
        OpCode::KEYFLUSH => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-keyflush")),
        OpCode::LASTIN => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-lastin")),
        OpCode::FLAG => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-flag")),
        OpCode::DOWNLOAD => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-download")),
        OpCode::WRUSYSDOOR => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-wrusysdoor")),
        OpCode::GETALTUSER => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-getaltuser")),
        OpCode::ADJDBYTES => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-adjdbytes")),
        OpCode::ADJTBYTES => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-adjtbytes")),
        OpCode::ADJTFILES => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-adjtfiles")),
        OpCode::LANG => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-lang")),
        OpCode::SORT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-sort")),
        OpCode::MOUSEREG => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-mousereg")),
        OpCode::SCRFILE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-scrfile")),
        OpCode::SEARCHINIT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-searchinit")),
        OpCode::SEARCHFIND => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-searchfind")),
        OpCode::SEARCHSTOP => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-searchstop")),
        OpCode::PRFOUND => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-prfound")),
        OpCode::PRFOUNDLN => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-prfoundln")),
        OpCode::TPAGET => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-tpaget")),
        OpCode::TPAPUT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-tpaput")),
        OpCode::TPACGET => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-tpacget")),
        OpCode::TPACPUT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-tpacput")),
        OpCode::TPAREAD => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-tparead")),
        OpCode::TPAWRITE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-tpawrite")),
        OpCode::TPACREAD => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-tpacread")),
        OpCode::TPACWRITE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-tpacwrite")),
        OpCode::BITSET => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-bitset")),
        OpCode::BITCLEAR => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-bitclear")),
        OpCode::BRAG => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-brag")),
        OpCode::FREALTUSER => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-frealtuser")),
        OpCode::SETLMR => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-setlmr")),
        OpCode::SETENV => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-setenv")),
        OpCode::FCLOSEALL => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fcloseall")),
        OpCode::DECLARE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-declare")),
        OpCode::FUNCTION => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-function")),
        OpCode::PROCEDURE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-procedure")),
        OpCode::PCALL => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-pcall")),
        OpCode::FPCLR => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fpclr")),
        OpCode::BEGIN => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-begin")),
        OpCode::FEND => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fend")),
        OpCode::STATIC => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-static")),
        OpCode::STACKABORT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-stackabort")),
        OpCode::DCREATE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dcreate")),
        OpCode::DOPEN => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dopen")),
        OpCode::DCLOSE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dclose")),
        OpCode::DSETALIAS => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dsetalias")),
        OpCode::DPACK => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dpack")),
        OpCode::DCLOSEALL => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dcloseall")),
        OpCode::DLOCK => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dlock")),
        OpCode::DLOCKR => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dlockr")),
        OpCode::DLOCKG => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dlockg")),
        OpCode::DUNLOCK => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dunlock")),
        OpCode::DNCREATE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dncreate")),
        OpCode::DNOPEN => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dnopen")),
        OpCode::DNCLOSE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dnclose")),
        OpCode::DNCLOSEALL => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dncloseall")),
        OpCode::DNEW => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dnew")),
        OpCode::DADD => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dadd")),
        OpCode::DAPPEND => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dappend")),
        OpCode::DTOP => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dtop")),
        OpCode::DGO => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dgo")),
        OpCode::DBOTTOM => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dbottom")),
        OpCode::DSKIP => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dskip")),
        OpCode::DBLANK => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dblank")),
        OpCode::DDELETE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-ddelete")),
        OpCode::DRECALL => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-drecall")),
        OpCode::DTAG => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dtag")),
        OpCode::DSEEK => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dseek")),
        OpCode::DFBLANK => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dfblank")),
        OpCode::DGET => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dget")),
        OpCode::DPUT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dput")),
        OpCode::DFCOPY => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-dfcopy")),
        OpCode::EVAL => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-eval")),
        OpCode::ACCOUNT => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-account")),
        OpCode::RECORDUSAGE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-recordusage")),
        OpCode::MSGTOFILE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-msgtofile")),
        OpCode::QWKLIMITS => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-qwklimits")),
        OpCode::COMMAND => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-command")),
        OpCode::USELMRS => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-uselmrs")),
        OpCode::CONFINFO => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-confinfo")),
        OpCode::ADJTUBYTES => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-adjtubytes")),
        OpCode::GRAFMODE => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-grafmode")),
        OpCode::ADDUSER => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-adduser")),
        OpCode::KILLMSG => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-killmsg")),
        OpCode::CHDIR => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-chdir")),
        OpCode::MKDIR => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-mkdir")),
        OpCode::REDIR => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-redir")),
        OpCode::FDOWRAKA => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fdowraka")),
        OpCode::FDOADDAKA => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fdoaddaka")),
        OpCode::FDOWRORG => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fdowrorg")),
        OpCode::FDOADDORG => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fdoaddorg")),
        OpCode::FDOQMOD => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fdoqmod")),
        OpCode::FDOQADD => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fdoqadd")),
        OpCode::FDOQDEL => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-fdoqdel")),
        OpCode::SOUNDDELAY => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-sounddelay")),
        OpCode::ShortDesc => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-shortdesc")),
        OpCode::MoveMsg => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-movemsg")),
        OpCode::SetBankBal => get_hint(fl!(LANGUAGE_LOADER, "hint-statement-setbankbal")),
        _ => None,
    }
}

fn get_hint(arg: String) -> Option<Hover> {
    Some(Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: arg,
        }),
        range: None,
    })
}
