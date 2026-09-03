use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{ast::ModuleDeclaration, compiler::workspace::Workspace};
use unicase::Ascii;

use super::{
    Encoding, ErrorReporter, Parser, ParserErrorType, UserTypeRegistry,
    lexer::{Lexer, Spanned, Token},
};

static PROC_TOKEN: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("PROC".to_string()));
static FUNC_TOKEN: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("FUNC".to_string()));
static ON_TOKEN: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("ON".to_string()));
static ERROR_TOKEN: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new("ERROR".to_string()));

impl<'a> Parser<'a> {
    pub fn new(
        file: PathBuf,
        error_reporter: Arc<Mutex<ErrorReporter>>,
        type_registry: &'a UserTypeRegistry,
        text: &str,
        encoding: Encoding,
        workspace: &Workspace,
    ) -> Self {
        let implicit_module = workspace.dependency_module(&file).map(ModuleDeclaration::implicit);
        let dependency_imports = workspace.dependency_imports(&file).cloned().unwrap_or_default();
        let in_module = implicit_module.is_some();
        let lex = Lexer::new(file, workspace, text, encoding, error_reporter.clone());
        let lang_version = lex.lang_version();
        Parser {
            error_reporter,
            lang_version,
            cur_token: None,
            lookahead_token: None,
            lex,
            require_user_variables: false,
            type_registry,
            use_funcs: false,
            parsed_begin: false,
            parsed_block: false,
            got_statement: false,
            got_funcs: false,
            in_function: false,
            types_predeclared: false,
            module: implicit_module,
            imports: Vec::new(),
            dependency_imports,
            in_module,
            expression_depth: 0,
            statement_depth: 0,
        }
    }

    pub fn get_cur_token(&self) -> Option<Token> {
        self.cur_token.as_ref().map(|token| token.token.clone())
    }

    /// Advances to the next token, folding compound keywords into one parser token.
    pub fn next_token(&mut self) -> Option<Spanned<Token>> {
        if let Some(token) = self.lookahead_token.take() {
            self.cur_token = Some(token);
            return self.cur_token.clone();
        }

        if let Some(token) = self.lex.next_token() {
            let is_else = token == Token::Else;
            let is_end = token == Token::Identifier(Ascii::new("END".to_string()));
            let is_case = token == Token::Case;
            // Neither word is reserved on its own, so `ON` stays usable as a name.
            let is_on = self.lang_version >= 400 && token == Token::Identifier(ON_TOKEN.clone());
            self.cur_token = Some(Spanned::new(token, self.lex.span()));

            if is_on {
                let start = self.lex.span().start;
                let end = self.lex.span().end;
                if let Some(lookahead) = self.lex.next_token() {
                    if lookahead == Token::Identifier(ERROR_TOKEN.clone()) {
                        self.cur_token = Some(Spanned::new(Token::OnError, start..self.lex.span().end));
                    } else {
                        self.lookahead_token = Some(Spanned::new(lookahead, end..self.lex.span().end));
                    }
                }
            } else if is_else {
                let start = self.lex.span().start;
                let end = self.lex.span().end;
                if let Some(lookahead) = self.lex.next_token() {
                    if lookahead == Token::If {
                        self.cur_token = Some(Spanned::new(Token::ElseIf, start..self.lex.span().end));
                    } else {
                        self.lookahead_token = Some(Spanned::new(lookahead, end..self.lex.span().end));
                    }
                }
            } else if is_case {
                let start = self.lex.span().start;
                let end = self.lex.span().end;
                if let Some(lookahead) = self.lex.next_token() {
                    if lookahead == Token::Else {
                        self.cur_token = Some(Spanned::new(Token::Default, start..self.lex.span().end));
                    } else {
                        self.lookahead_token = Some(Spanned::new(lookahead, end..self.lex.span().end));
                    }
                }
            } else if is_end {
                let start = self.lex.span().start;
                let end = self.lex.span().end;
                if let Some(lookahead) = self.lex.next_token() {
                    match lookahead {
                        Token::If => {
                            self.cur_token = Some(Spanned::new(Token::EndIf, start..self.lex.span().end));
                        }
                        Token::While => {
                            self.cur_token = Some(Spanned::new(Token::EndWhile, start..self.lex.span().end));
                        }
                        Token::Select => {
                            self.cur_token = Some(Spanned::new(Token::EndSelect, start..self.lex.span().end));
                        }
                        Token::Loop => {
                            self.cur_token = Some(Spanned::new(Token::EndLoop, start..self.lex.span().end));
                        }
                        Token::Type => {
                            self.cur_token = Some(Spanned::new(Token::EndType, start..self.lex.span().end));
                        }
                        Token::Enum => {
                            self.cur_token = Some(Spanned::new(Token::EndEnum, start..self.lex.span().end));
                        }
                        Token::For => {
                            self.cur_token = Some(Spanned::new(Token::Next, start..self.lex.span().end));
                        }
                        _ => {
                            let set_lookahead = if let Token::Identifier(identifier) = &lookahead {
                                if *identifier == *PROC_TOKEN {
                                    self.cur_token = Some(Spanned::new(Token::EndProc, end..self.lex.span().end));
                                    false
                                } else if *identifier == *FUNC_TOKEN {
                                    self.cur_token = Some(Spanned::new(Token::EndFunc, end..self.lex.span().end));
                                    false
                                } else {
                                    true
                                }
                            } else {
                                true
                            };

                            if set_lookahead {
                                self.lookahead_token = Some(Spanned::new(lookahead, end..self.lex.span().end));
                            }
                        }
                    }
                }
            }
        } else {
            self.cur_token = None;
        }
        self.cur_token.clone()
    }

    pub(super) fn save_token_span(&self) -> std::ops::Range<usize> {
        if let Some(token) = &self.cur_token { token.span.clone() } else { 0..0 }
    }

    pub(super) fn save_token(&self) -> Token {
        if let Some(token) = &self.cur_token { token.token.clone() } else { Token::Eol }
    }

    pub(super) fn save_spanned_token(&self) -> Spanned<Token> {
        if let Some(token) = &self.cur_token {
            token.clone()
        } else {
            Spanned::new(Token::Eol, 0..0)
        }
    }

    pub(super) fn report_error(&mut self, span: std::ops::Range<usize>, error: ParserErrorType) {
        self.error_reporter.lock().unwrap().report_error(span, error);
        while self.get_cur_token().is_some() && self.get_cur_token() != Some(Token::Eol) && !matches!(self.get_cur_token(), Some(Token::Comment(_, _))) {
            self.next_token();
        }
    }

    pub(super) fn peek_after_current(&mut self, count: usize) -> Vec<Option<Token>> {
        let lexer = self.lex.clone();
        let current_token = self.cur_token.clone();
        let lookahead_token = self.lookahead_token.clone();
        let result = (0..count).map(|_| self.next_token().map(|token| token.token)).collect();
        self.lex = lexer;
        self.cur_token = current_token;
        self.lookahead_token = lookahead_token;
        result
    }
}
