use crate::{
    ast::{
        BlockStatement, BreakStatement, CaseBlock, CaseSpecifier, CommentAstNode, Constant,
        ContinueStatement, ElseBlock, ElseIfBlock, ForStatement, GosubStatement, GotoStatement,
        IdentifierExpression, IfStatement, IfThenStatement, LabelStatement, LetStatement,
        PredefinedCallStatement, ProcedureCallStatement, ReturnStatement, SelectStatement,
        Statement, VariableDeclarationStatement, WhileDoStatement, WhileStatement,
    },
    executable::{OpCode, StatementDefinition},
    parser::ParserErrorType,
};

use super::{
    lexer::{Spanned, Token},
    Parser, ParserWarningType,
};

impl Parser {
    pub fn skip_eol(&mut self) {
        while self.get_cur_token() == Some(Token::Eol) {
            self.next_token();
        }
    }
    pub fn skip_eol_and_comments(&mut self) {
        while self.get_cur_token() == Some(Token::Eol)
            || matches!(self.get_cur_token(), Some(Token::Comment(_, _)))
        {
            self.next_token();
        }
    }

    fn parse_while(&mut self) -> Option<Statement> {
        let while_token = self.save_spanned_token();
        self.next_token();

        if self.get_cur_token() != Some(Token::LPar) {
            self.report_error(self.lex.span(), ParserErrorType::IfWhileConditionNotFound);
            return None;
        }
        let lpar_token = self.save_spanned_token();

        self.next_token();
        let Some(cond) = self.parse_expression() else {
            self.report_error(self.lex.span(), ParserErrorType::IfWhileConditionNotFound);
            return None;
        };

        if self.get_cur_token() != Some(Token::RPar) {
            self.report_error(
                self.lex.span(),
                ParserErrorType::MissingCloseParens(self.save_token()),
            );
            return None;
        }
        let rightpar_token = self.save_spanned_token();
        self.next_token();

        if self.get_cur_token() == Some(Token::Identifier(unicase::Ascii::new("DO".to_string()))) {
            let do_token = self.save_spanned_token();
            self.next_token();

            let mut statements = Vec::new();
            self.skip_eol();
            let mut end_while_token = None;
            while self.get_cur_token() != Some(Token::EndWhile) {
                if self.get_cur_token().is_none() {
                    self.report_error(self.lex.span(), ParserErrorType::EndExpected);
                    return None;
                }
                statements.push(self.parse_statement());
                self.skip_eol();
            }
            if end_while_token.is_none() {
                end_while_token = Some(self.save_spanned_token());
            }
            self.next_token(); // skip ENDWHILE

            Some(Statement::WhileDo(WhileDoStatement::new(
                while_token,
                lpar_token,
                cond,
                rightpar_token,
                do_token,
                statements.into_iter().flatten().collect(),
                end_while_token.unwrap(),
            )))
        } else {
            self.skip_eol();
            let start = self.lex.span().start;
            if let Some(stmt) = self.parse_statement() {
                Some(Statement::While(WhileStatement::new(
                    while_token,
                    lpar_token,
                    cond,
                    rightpar_token,
                    stmt,
                )))
            } else {
                self.report_error(
                    start..self.lex.span().end,
                    ParserErrorType::StatementExpected,
                );
                None
            }
        }
    }

    fn _parse_block(&mut self, begin_token: Spanned<Token>) -> Option<Statement> {
        self.next_token();
        self.skip_eol();
        let mut statements = Vec::new();
        loop {
            let Some(token) = self.get_cur_token() else {
                self.report_error(self.lex.span(), ParserErrorType::EndExpected);
                return None;
            };
            if token == Token::Identifier(unicase::Ascii::new("END".to_string())) {
                break;
            }
            statements.push(self.parse_statement());
            self.skip_eol();
        }
        let end_token = self.save_spanned_token();
        Some(Statement::Block(BlockStatement::new(
            begin_token,
            statements.into_iter().flatten().collect(),
            end_token,
        )))
    }

    fn parse_for(&mut self) -> Option<Statement> {
        let for_token = self.save_spanned_token();
        self.next_token();
        let identifier_token = self.save_spanned_token();

        let _var = if let Some(Token::Identifier(id)) = self.get_cur_token() {
            self.next_token();
            IdentifierExpression::create_empty_expression(id)
        } else {
            self.report_error(
                self.lex.span(),
                ParserErrorType::IdentifierExpected(self.save_token()),
            );
            return None;
        };

        if self.get_cur_token() != Some(Token::Eq) {
            self.report_error(
                self.lex.span(),
                ParserErrorType::EqTokenExpected(self.save_token()),
            );
            return None;
        }

        let eq_token = self.save_spanned_token();
        self.next_token();
        let Some(start_expr) = self.parse_expression() else {
            self.report_error(
                self.lex.span(),
                ParserErrorType::ExpressionExpected(self.save_token()),
            );
            return None;
        };

        if let Some(Token::Identifier(id)) = self.get_cur_token() {
            if id != "TO" {
                self.report_error(
                    self.lex.span(),
                    ParserErrorType::ToExpected(self.save_token()),
                );
                return None;
            }
        }

        let to_token = self.save_spanned_token();
        self.next_token();
        let Some(end_expr) = self.parse_expression() else {
            self.report_error(
                self.lex.span(),
                ParserErrorType::ExpressionExpected(self.save_token()),
            );

            return None;
        };

        let (step_expr, step_token) = if self.get_cur_token()
            == Some(Token::Identifier(unicase::Ascii::new("STEP".to_string())))
        {
            let to_token = self.save_spanned_token();
            self.next_token();
            (self.parse_expression().map(Box::new), Some(to_token))
        } else {
            (None, None)
        };

        let mut statements = Vec::new();
        self.skip_eol();
        while self.get_cur_token() != Some(Token::Next) {
            if self.get_cur_token().is_none() {
                self.report_error(self.lex.span(), ParserErrorType::EndExpected);
                return None;
            }
            statements.push(self.parse_statement());
            self.skip_eol();
        }
        let next_token = self.save_spanned_token();
        // skip next
        self.next_token();

        let next_identifier_token = if let Some(Token::Identifier(next_id)) = &self.get_cur_token()
        {
            let start_id = identifier_token.token.get_identifier();
            if *next_id != start_id {
                self.errors.lock().unwrap().report_warning(
                    self.lex.span(),
                    ParserWarningType::NextIdentifierInvalid(start_id, self.save_token()),
                );
                return None;
            }

            let t = self.save_spanned_token();
            self.next_token();
            Some(t)
        } else {
            None
        };

        Some(Statement::For(ForStatement::new(
            for_token,
            identifier_token,
            eq_token,
            start_expr,
            to_token,
            end_expr,
            step_token,
            step_expr,
            statements.into_iter().flatten().collect(),
            next_token,
            next_identifier_token,
        )))
    }

    fn parse_if(&mut self) -> Option<Statement> {
        let if_token = self.save_spanned_token();
        self.next_token();

        if self.get_cur_token() != Some(Token::LPar) {
            self.report_error(self.lex.span(), ParserErrorType::IfWhileConditionNotFound);
            return None;
        }
        let lpar_token = self.save_spanned_token();
        self.next_token();
        let Some(cond) = self.parse_expression() else {
            self.report_error(self.lex.span(), ParserErrorType::IfWhileConditionNotFound);

            return None;
        };

        if self.get_cur_token() != Some(Token::RPar) {
            self.report_error(
                self.lex.span(),
                ParserErrorType::MissingCloseParens(self.save_token()),
            );

            return None;
        }
        let rightpar_token = self.save_spanned_token();
        self.next_token();

        if !is_do_then(&self.cur_token) {
            self.skip_eol();

            let start = self.lex.span().start;
            if let Some(stmt) = self.parse_statement() {
                return Some(Statement::If(IfStatement::new(
                    if_token,
                    lpar_token,
                    cond,
                    rightpar_token,
                    stmt,
                )));
            }
            self.report_error(
                start..self.lex.span().end,
                ParserErrorType::StatementExpected,
            );
            return None;
        }
        let then_token = self.save_spanned_token();

        self.next_token();
        let mut statements = Vec::new();
        self.skip_eol_and_comments();
        let mut end_if_token = None;

        while self.get_cur_token() != Some(Token::EndIf)
            && self.get_cur_token() != Some(Token::Else)
            && self.get_cur_token() != Some(Token::ElseIf)
        {
            if self.get_cur_token().is_none() {
                self.report_error(self.lex.span(), ParserErrorType::EndExpected);
                return None;
            }

            statements.push(self.parse_statement());
            self.skip_eol();
        }

        let mut else_if_blocks = Vec::new();
        while self.get_cur_token() == Some(Token::ElseIf) {
            let else_if_token = self.save_spanned_token();

            self.next_token();

            if self.get_cur_token() != Some(Token::LPar) {
                self.report_error(self.lex.span(), ParserErrorType::IfWhileConditionNotFound);
                return None;
            }
            let else_if_lpar_token = self.save_spanned_token();

            self.next_token();
            let Some(cond) = self.parse_expression() else {
                self.report_error(self.lex.span(), ParserErrorType::IfWhileConditionNotFound);

                return None;
            };

            if self.get_cur_token() != Some(Token::RPar) {
                self.report_error(
                    self.lex.span(),
                    ParserErrorType::MissingCloseParens(self.save_token()),
                );

                return None;
            }
            let else_if_rightpar_token = self.save_spanned_token();
            self.next_token();
            if !is_do_then(&self.cur_token)
                && self.get_cur_token() != Some(Token::Eol)
                && !matches!(self.get_cur_token(), Some(Token::Comment(_, _)))
            {
                self.report_error(
                    self.lex.span(),
                    ParserErrorType::ThenExpected(self.save_token()),
                );
                return None;
            }
            let then_token = if is_do_then(&self.cur_token) {
                Some(self.save_spanned_token())
            } else {
                None
            };
            self.next_token();

            let mut statements = Vec::new();
            while self.get_cur_token() != Some(Token::EndIf)
                && self.get_cur_token() != Some(Token::Else)
                && self.get_cur_token() != Some(Token::ElseIf)
            {
                if self.get_cur_token().is_none() {
                    self.report_error(self.lex.span(), ParserErrorType::EndExpected);
                    return None;
                }

                statements.push(self.parse_statement());
                self.skip_eol();
            }

            else_if_blocks.push(ElseIfBlock::new(
                else_if_token,
                else_if_lpar_token,
                cond,
                else_if_rightpar_token,
                then_token,
                statements.into_iter().flatten().collect(),
            ));
        }

        let else_block = if self.get_cur_token() == Some(Token::Else) {
            let else_token = self.save_spanned_token();

            self.next_token();
            let mut statements = Vec::new();
            self.skip_eol();
            while self.get_cur_token() != Some(Token::EndIf) {
                if self.get_cur_token().is_none() {
                    self.report_error(self.lex.span(), ParserErrorType::EndExpected);
                    return None;
                }

                if let Some(stmt) = self.parse_statement() {
                    statements.push(stmt);
                }
                self.skip_eol();
            }
            Some(ElseBlock::new(else_token, statements))
        } else {
            None
        };

        if self.get_cur_token() != Some(Token::EndIf) {
            self.report_error(
                self.lex.span(),
                ParserErrorType::InvalidToken(self.save_token()),
            );
            return None;
        }
        // skip endif token
        self.next_token();

        if end_if_token.is_none() {
            end_if_token = Some(self.save_spanned_token());
        }

        Some(Statement::IfThen(IfThenStatement::new(
            if_token,
            lpar_token,
            cond,
            rightpar_token,
            then_token,
            statements.into_iter().flatten().collect(),
            else_if_blocks,
            else_block,
            end_if_token.unwrap(),
        )))
    }

    fn parse_select(&mut self) -> Option<Statement> {
        let select_token = self.save_spanned_token();

        self.next_token();

        if self.get_cur_token() != Some(Token::Case) {
            self.report_error(
                select_token.span.start..self.lex.span().end,
                ParserErrorType::CaseExpectedAfterSelect,
            );
            return None;
        }

        let case_token = self.save_spanned_token();
        self.next_token();
        let Some(case_expr) = self.parse_expression() else {
            self.report_error(
                self.lex.span(),
                ParserErrorType::ExpressionExpected(self.save_token()),
            );

            return None;
        };
        self.next_token();
        self.skip_eol();

        let mut case_blocks = Vec::new();
        let mut default_token = None;
        let mut end_select_token = None;
        let mut default_statements = Vec::new();

        while self.get_cur_token() == Some(Token::Case) {
            let inner_case_token = self.save_spanned_token();
            self.next_token();

            let mut case_specifiers = Vec::new();
            if let Some(cs) = self.parse_case_specifier() {
                case_specifiers.push(cs);
            } else {
                return None;
            }
            while self.get_cur_token() == Some(Token::Comma) {
                self.next_token();
                if let Some(cs) = self.parse_case_specifier() {
                    case_specifiers.push(cs);
                } else {
                    return None;
                }
            }
            self.skip_eol();

            let mut statements = Vec::new();
            while self.get_cur_token() != Some(Token::Case)
                && self.get_cur_token() != Some(Token::EndSelect)
                && self.get_cur_token() != Some(Token::Default)
            {
                if self.get_cur_token().is_none() {
                    self.report_error(self.lex.span(), ParserErrorType::EndExpected);
                    return None;
                }

                statements.push(self.parse_statement());
                self.skip_eol();
            }
            case_blocks.push(CaseBlock::new(
                inner_case_token,
                case_specifiers,
                statements.into_iter().flatten().collect(),
            ));
        }

        if self.get_cur_token() == Some(Token::Default) {
            default_token = Some(self.save_spanned_token());
            self.next_token();
            self.parse_default_block(&mut default_statements);
        }
        // skip Default
        self.next_token();

        if end_select_token.is_none() {
            end_select_token = Some(self.save_spanned_token());
        }

        Some(Statement::Select(SelectStatement::new(
            select_token,
            case_token,
            case_expr,
            case_blocks,
            default_token,
            default_statements,
            end_select_token.unwrap(),
        )))
    }

    fn parse_default_block(&mut self, default_statements: &mut Vec<Statement>) {
        while self.get_cur_token() != Some(Token::EndSelect) {
            if self.get_cur_token().is_none() {
                self.report_error(self.lex.span(), ParserErrorType::EndExpected);
                return;
            }

            if let Some(stmt) = self.parse_statement() {
                default_statements.push(stmt);
            }
            self.skip_eol();
        }
    }

    fn parse_case_specifier(&mut self) -> Option<crate::ast::CaseSpecifier> {
        let Some(expr) = self.parse_expression() else {
            self.report_error(
                self.lex.span(),
                ParserErrorType::ExpressionExpected(self.save_token()),
            );

            return None;
        };

        if self.get_cur_token() == Some(Token::DotDot) {
            self.next_token();
            let Some(to) = self.parse_expression() else {
                self.report_error(
                    self.lex.span(),
                    ParserErrorType::ExpressionExpected(self.save_token()),
                );

                return None;
            };
            Some(CaseSpecifier::FromTo(Box::new(expr), Box::new(to)))
        } else {
            Some(CaseSpecifier::Expression(Box::new(expr)))
        }
    }
    /// Returns the parse statement of this [`Tokenizer`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn parse_statement(&mut self) -> Option<Statement> {
        match self.get_cur_token() {
            Some(Token::Eol) => {
                self.next_token();
                None
            }
            Some(Token::Comment(_, _)) => {
                let cmt = self.save_spanned_token();
                self.next_token();
                Some(Statement::Comment(CommentAstNode::new(cmt)))
            }
            Some(Token::UseFuncs(_, _)) => {
                self.errors
                    .lock()
                    .unwrap()
                    .report_warning(self.lex.span(), ParserWarningType::UsefuncsIgnored);
                self.next_token();
                None
            }
            /*
            Some(Token::Begin) => {
                let begin_token = self.save_spanned_token();
                self.parse_block(begin_token)
            }*/
            Some(Token::While) => self.parse_while(),
            Some(Token::Select) => self.parse_select(),
            Some(Token::If) => self.parse_if(),
            Some(Token::For) => self.parse_for(),
            Some(Token::Let) => {
                let let_token = self.save_spanned_token();
                self.next_token();
                let identifier_token = self.save_spanned_token();
                let _identifier = if let Token::Identifier(id) = &identifier_token.token {
                    self.next_token();
                    id
                } else {
                    self.report_error(
                        self.lex.span(),
                        ParserErrorType::IdentifierExpected(self.save_token()),
                    );
                    return None;
                };
                let mut leftpar_token = None;
                let mut rightpar_token = None;
                let mut params = Vec::new();

                if self.get_cur_token() == Some(Token::LPar) {
                    leftpar_token = Some(self.save_spanned_token());
                    self.next_token();

                    loop {
                        let Some(token) = self.get_cur_token() else {
                            self.report_error(self.lex.span(), ParserErrorType::EndExpected);
                            return None;
                        };

                        if token == Token::RPar {
                            break;
                        }
                        let Some(expr) = self.parse_expression() else {
                            self.report_error(self.lex.span(), ParserErrorType::EndExpected);
                            return None;
                        };
                        params.push(expr);
                        self.skip_eol();

                        if self.get_cur_token() == Some(Token::RPar) {
                            break;
                        }
                        if self.get_cur_token() == Some(Token::Comma) {
                            self.next_token();
                        } else {
                            self.report_error(
                                self.lex.span(),
                                ParserErrorType::CommaExpected(self.save_token()),
                            );
                            return None;
                        }
                    }
                    rightpar_token = Some(self.save_spanned_token());
                    self.next_token();
                }

                if self.get_cur_token() == Some(Token::Eq) {
                    let eq_token = self.save_spanned_token();
                    self.next_token();
                    let Some(value_expression) = self.parse_expression() else {
                        self.report_error(
                            self.lex.span(),
                            ParserErrorType::ExpressionExpected(self.save_token()),
                        );

                        return None;
                    };
                    return Some(Statement::Let(LetStatement::new(
                        Some(let_token),
                        identifier_token,
                        leftpar_token,
                        params,
                        rightpar_token,
                        eq_token,
                        value_expression,
                    )));
                }

                self.report_error(
                    self.lex.span(),
                    ParserErrorType::EqTokenExpected(self.save_token()),
                );
                None
            }
            Some(Token::Break) => {
                let tok = self.save_spanned_token();
                self.next_token();
                Some(Statement::Break(BreakStatement::new(tok)))
            }
            Some(Token::Continue) => {
                let tok = self.save_spanned_token();
                self.next_token();
                Some(Statement::Continue(ContinueStatement::new(tok)))
            }
            Some(Token::EndProc | Token::EndFunc | Token::Return) => {
                let tok = self.save_spanned_token();
                self.next_token();
                Some(Statement::Return(ReturnStatement::new(tok)))
            }
            Some(Token::Gosub) => {
                let gosub_token = self.save_spanned_token();
                self.next_token();
                if let Some(token) = self.get_cur_token() {
                    if token.token_can_be_identifier() {
                        let id_token = self.save_spanned_token();
                        self.next_token();
                        return Some(Statement::Gosub(GosubStatement::new(gosub_token, id_token)));
                    }
                    self.next_token();
                }
                self.report_error(
                    self.lex.span(),
                    ParserErrorType::LabelExpected(self.save_token()),
                );
                self.next_token();
                None
            }
            Some(Token::Goto) => {
                let goto_token = self.save_spanned_token();
                self.next_token();
                if let Some(token) = self.get_cur_token() {
                    if token.token_can_be_identifier() {
                        let id_token = self.save_spanned_token();
                        self.next_token();
                        return Some(Statement::Goto(GotoStatement::new(goto_token, id_token)));
                    }
                    self.next_token();
                }
                self.report_error(
                    self.lex.span(),
                    ParserErrorType::LabelExpected(self.save_token()),
                );
                self.next_token();
                None
            }
            Some(Token::Const(Constant::Builtin(c))) => {
                if let Some(value) = self.parse_call(&unicase::Ascii::new(c.name.to_string())) {
                    return Some(value);
                }
                self.next_token();
                self.parse_statement()
            }

            Some(Token::Identifier(id)) => {
                if let Some(var_type) = self.get_variable_type() {
                    let type_token = self.save_spanned_token();
                    self.next_token();
                    let mut vars = Vec::new();
                    if let Some(v) = self.parse_var_info() {
                        vars.push(v);
                    } else {
                        return None;
                    }
                    while self.get_cur_token() == Some(Token::Comma) {
                        self.next_token();
                        if let Some(v) = self.parse_var_info() {
                            vars.push(v);
                        } else {
                            return None;
                        }
                    }
                    return Some(Statement::VariableDeclaration(
                        VariableDeclarationStatement::new(type_token, var_type, vars),
                    ));
                }

                if let Some(value) = self.parse_call(&id) {
                    return Some(value);
                }
                self.next_token();
                self.parse_statement()
            }

            Some(Token::Label(_)) => {
                let label_token = self.save_spanned_token();
                self.next_token();
                Some(Statement::Label(LabelStatement::new(label_token)))
            }
            Some(Token::EndIf | Token::EndWhile | Token::Next | Token::EndSelect) => {
                self.report_error(
                    self.save_token_span(),
                    ParserErrorType::BlockEndBeforeBlockStart,
                );
                self.next_token();
                None
            }
            None => None,
            _ => {
                self.report_error(
                    self.save_token_span(),
                    ParserErrorType::InvalidToken(self.save_token()),
                );
                self.next_token();
                None
            }
        }
    }

    fn parse_call(&mut self, identifier: &unicase::Ascii<String>) -> Option<Statement> {
        let id_token = self.save_spanned_token();
        self.next_token();

        if self.get_cur_token() == Some(Token::Eq) {
            let eq_token = self.save_spanned_token();
            self.next_token();
            let Some(value_expression) = self.parse_expression() else {
                self.report_error(
                    self.lex.span(),
                    ParserErrorType::ExpressionExpected(self.save_token()),
                );

                return None;
            };
            return Some(Statement::Let(LetStatement::new(
                None,
                id_token,
                None,
                Vec::new(),
                None,
                eq_token,
                value_expression,
            )));
        }
        if self.get_cur_token() != Some(Token::LPar) {
            // check 'pseudo keywords'
            if *identifier == *QUIT_TOKEN {
                return Some(Statement::Break(BreakStatement::new(id_token)));
            }
            if *identifier == *LOOP_TOKEN {
                return Some(Statement::Continue(ContinueStatement::new(id_token)));
            }

            if *identifier == *BEGIN_TOKEN {
                return Some(Statement::Label(LabelStatement::new(Spanned {
                    token: Token::Label(BEGIN_LABEL.clone()),
                    span: id_token.span,
                })));
            }
        }
        if let Some(def) = StatementDefinition::get_statement_definition(identifier) {
            let mut params = Vec::new();
            while self.get_cur_token() != Some(Token::Eol)
                && !matches!(self.get_cur_token(), Some(Token::Comment(_, _)))
                && self.cur_token.is_some()
            {
                let Some(value) = self.parse_expression() else {
                    self.report_error(
                        self.lex.span(),
                        ParserErrorType::ExpressionExpected(self.save_token()),
                    );
                    return None;
                };
                params.push(value);

                if self.cur_token.is_none() {
                    break;
                }
                if self.get_cur_token() == Some(Token::Comma) {
                    self.next_token();
                } else {
                    break;
                }
            }

            if def.opcode == OpCode::GETUSER
                || def.opcode == OpCode::PUTUSER
                || def.opcode == OpCode::GETALTUSER
                || def.opcode == OpCode::FREALTUSER
                || def.opcode == OpCode::DELUSER
                || def.opcode == OpCode::ADDUSER
            {
                self.require_user_variables = true;
            }
            if self.version < def.version {
                self.report_error(
                    id_token.span,
                    ParserErrorType::StatementVersionNotSupported(
                        def.opcode,
                        def.version,
                        self.version,
                    ),
                );
                return None;
            }
            return Some(Statement::PredifinedCall(PredefinedCallStatement::new(
                id_token, def, params,
            )));
        }

        if self.get_cur_token() == Some(Token::LPar) {
            let lpar_token = self.save_spanned_token();

            self.next_token();
            let mut params = Vec::new();

            while self.get_cur_token() != Some(Token::RPar) {
                let Some(right) = self.parse_expression() else {
                    self.report_error(
                        self.lex.span(),
                        ParserErrorType::ExpressionExpected(self.save_token()),
                    );

                    return None;
                };
                params.push(right);
                if self.get_cur_token() == Some(Token::Comma) {
                    self.next_token();
                }
            }
            if self.get_cur_token() != Some(Token::RPar) {
                self.report_error(
                    self.save_token_span(),
                    ParserErrorType::MissingCloseParens(self.save_token()),
                );

                return None;
            }
            let rightpar_token = self.save_spanned_token();

            self.next_token();
            if self.get_cur_token() == Some(Token::Eq) {
                let eq_token = self.save_spanned_token();
                self.next_token();
                let Some(value_expression) = self.parse_expression() else {
                    self.report_error(
                        self.lex.span(),
                        ParserErrorType::ExpressionExpected(self.save_token()),
                    );

                    return None;
                };
                if !params.is_empty() && params.len() <= 3 {
                    return Some(Statement::Let(LetStatement::new(
                        None,
                        id_token,
                        Some(lpar_token),
                        params,
                        Some(rightpar_token),
                        eq_token,
                        value_expression,
                    )));
                }
                self.report_error(
                    self.lex.span(),
                    ParserErrorType::TooManyDimensions(params.len()),
                );
                return None;
            }

            return Some(Statement::Call(ProcedureCallStatement::new(
                id_token,
                lpar_token,
                params,
                rightpar_token,
            )));
        }

        self.report_error(
            id_token.span,
            ParserErrorType::UnknownIdentifier(id_token.token.to_string()),
        );
        None
    }
}

lazy_static::lazy_static! {
    static ref DO_TOKEN: unicase::Ascii<String> = unicase::Ascii::new("DO".to_string());
    static ref THEN_TOKEN: unicase::Ascii<String> = unicase::Ascii::new("THEN".to_string());


    // potential keywords
    static ref QUIT_TOKEN: unicase::Ascii<String> = unicase::Ascii::new("QUIT".to_string());
    static ref LOOP_TOKEN: unicase::Ascii<String> = unicase::Ascii::new("LOOP".to_string());

    static ref BEGIN_TOKEN: unicase::Ascii<String> = unicase::Ascii::new("BEGIN".to_string());
    pub static ref BEGIN_LABEL: unicase::Ascii<String> = unicase::Ascii::new("~BEGIN~".to_string());

}

fn is_do_then(token: &Option<Spanned<Token>>) -> bool {
    if let Some(t) = token {
        if let Token::Identifier(id) = &t.token {
            *id == *THEN_TOKEN || *id == *DO_TOKEN
        } else {
            false
        }
    } else {
        false
    }
}
/*
#[cfg(test)]
mod tests {
    use crate::{
        ast:: Statement,
    };

    fn parse_statement(src: &str) -> Statement {
        let mut tokenizer = Tokenizer::new(src);
        tokenizer.next_token();
        tokenizer.parse_statement().unwrap()
    }

    fn get_statement_definition(name: &str) -> Option<&'static StatementDefinition> {
        let name = name.to_uppercase();
        STATEMENT_DEFINITIONS
            .iter()
            .find(|&def| def.name.to_uppercase() == name)
    }

    #[test]
    fn test_let() {
        assert_eq!(
            Statement::Let(
                Box::new(VarInfo::Var0("foo_bar".to_string())),
                Box::new(Expression::Const(Constant::Integer(1)))
            ),
            parse_statement("foo_bar=1")
        );

        assert_eq!(
            Statement::Let(
                Box::new(VarInfo::Var0("FOO".to_string())),
                Box::new(Expression::Const(Constant::Builtin(&BuiltinConst::FALSE)))
            ),
            parse_statement("LET FOO = FALSE")
        );
    }

    #[test]
    fn test_parse_statement() {
        assert_eq!(
            Statement::Call(
                get_statement_definition("ADJTIME").unwrap(),
                vec![Expression::Const(Constant::Integer(1))]
            ),
            parse_statement("ADJTIME 1")
        );
        assert_eq!(
            Statement::Call(
                get_statement_definition("ANSIPOS").unwrap(),
                vec![
                    Expression::Const(Constant::Integer(1)),
                    Expression::Const(Constant::Integer(2))
                ]
            ),
            parse_statement("ANSIPOS 1, 2")
        );
        assert_eq!(
            Statement::Call(
                get_statement_definition("BROADCAST").unwrap(),
                vec![
                    Expression::Const(Constant::Integer(1)),
                    Expression::Const(Constant::Integer(2)),
                    Expression::Const(Constant::Integer(3))
                ]
            ),
            parse_statement("BROADCAST 1, 2, 3")
        );
        assert_eq!(
            Statement::Call(get_statement_definition("BYE").unwrap(), vec![]),
            parse_statement("BYE")
        );
        assert_eq!(
            Statement::Call(get_statement_definition("PRINTLN").unwrap(), vec![]),
            parse_statement("PRINTLN")
        );
        assert_eq!(
            Statement::Call(
                get_statement_definition("PRINTLN").unwrap(),
                vec![Expression::Const(Constant::String(
                    "Hello World".to_string()
                ))]
            ),
            parse_statement("PRINTLN \"Hello World\"")
        );
    }
*/
/*
#[test]
fn test_parse_hello_world() {
    // check_statements(";This is a comment\nPRINT \"Hello World\"\n\t\n\n", vec![Statement::Call(get_statement_definition("PRINT").unwrap(), vec![Expression::Const(Constant::String("Hello World".to_string()))])]);
}

#[test]
fn test_gotogosub() {
    assert_eq!(
        Statement::Goto("LABEL1".to_string()),
        parse_statement("GOTO LABEL1")
    );

    assert_eq!(
        Statement::Gosub("LABEL2".to_string()),
        parse_statement("GOSUB LABEL2")
    );
    assert_eq!(
        Statement::Label("LABEL1".to_string()),
        parse_statement(":LABEL1")
    );
}

#[test]
fn test_incdec() {
    assert_eq!(
        Statement::Call(
            get_statement_definition("INC").unwrap(),
            vec![Expression::Identifier("VAR1".to_string()),]
        ),
        parse_statement("INC VAR1\n")
    );
    assert_eq!(
        Statement::Call(
            get_statement_definition("DEC").unwrap(),
            vec![Expression::Identifier("VAR2".to_string()),]
        ),
        parse_statement("DEC VAR2\n")
    );
}

#[test]
fn test_parse_simple_noncalls() {
    assert_eq!(Statement::End, parse_statement("End ; Predifined End"));

    assert_eq!(Statement::Break, parse_statement("BREAK"));

    assert_eq!(Statement::Continue, parse_statement("CONTINUE"));

    assert_eq!(Statement::Return, parse_statement("RETURN"));
}*/
/*
    #[test]
    fn test_procedure_calls() {
        assert_eq!(
            Statement::ProcedureCall("PROC".to_string(), Vec::new()),
            parse_statement("PROC()")
        );
        assert_eq!(
            Statement::ProcedureCall(
                "PROC".to_string(),
                vec![Expression::Const(Constant::Builtin(&BuiltinConst::TRUE))]
            ),
            parse_statement("PROC(TRUE)")
        );
        assert_eq!(
            Statement::ProcedureCall(
                "PROC".to_string(),
                vec![
                    Expression::Const(Constant::Integer(5)),
                    Expression::Const(Constant::Integer(7))
                ]
            ),
            parse_statement("PROC(5, 7)")
        );
    }

    #[test]
    fn test_parse_if() {
        assert_eq!(
            Statement::If(
                Box::new(Expression::Const(Constant::Builtin(&BuiltinConst::FALSE))),
                Box::new(Statement::End)
            ),
            parse_statement(" IF (FALSE) END")
        );
        let print_hello = parse_statement("PRINT \"Hello Word\"");
        let print_5 = parse_statement("PRINT 5");
        assert_eq!(
            Statement::IfThen(
                Box::new(Expression::Const(Constant::Builtin(&BuiltinConst::TRUE))),
                vec![print_hello.clone()],
                Vec::new(),
                None
            ),
            parse_statement(" IF (TRUE) THEN PRINT \"Hello Word\" ENDIF")
        );

        assert_eq!(
            Statement::IfThen(
                Box::new(Expression::Const(Constant::Builtin(&BuiltinConst::TRUE))),
                vec![print_hello.clone()],
                vec![ElseIfBlock {
                    cond: Box::new(Expression::Const(Constant::Builtin(&BuiltinConst::TRUE))),
                    block: vec![print_5]
                }],
                None
            ),
            parse_statement("IF (TRUE) THEN PRINT \"Hello Word\" ELSEIF (TRUE) THEN PRINT 5 ENDIF")
        );

        let print_5 = parse_statement("PRINT 5");

        assert_eq!(
            Statement::IfThen(
                Box::new(Expression::Const(Constant::Builtin(&BuiltinConst::FALSE))),
                Vec::new(),
                Vec::new(),
                Some(vec![print_5])
            ),
            parse_statement("IF (FALSE) THEN ELSE\nPRINT 5 ENDIF")
        );
    }

    #[test]
    fn test_parse_while() {
        assert_eq!(
            Statement::While(
                Box::new(Expression::Const(Constant::Builtin(&BuiltinConst::FALSE))),
                Box::new(Statement::End)
            ),
            parse_statement("WHILE (FALSE) END")
        );
        assert_eq!(
            Statement::DoWhile(
                Box::new(Expression::Const(Constant::Builtin(&BuiltinConst::TRUE))),
                Vec::new()
            ),
            parse_statement("WHILE (TRUE) DO ENDWHILE")
        );
    }

        #[test]
        fn test_for_next() {
            assert_eq!(
                Statement::For(
                    Box::new(Expression::Identifier("i".to_string())),
                    Box::new(Expression::Const(Constant::Integer(1))),
                    Box::new(Expression::Const(Constant::Integer(10))),
                    None,
                    Vec::new()
                ),
                parse_statement("FOR i = 1 TO 10 NEXT")
            );
            assert_eq!(
                Statement::For(
                    Box::new(Expression::Identifier("i".to_string())),
                    Box::new(Expression::Const(Constant::Integer(1))),
                    Box::new(Expression::Const(Constant::Integer(10))),
                    Some(Box::new(Expression::Const(Constant::Integer(5)))),
                    Vec::new()
                ),
                parse_statement("FOR i = 1 TO 10 STEP 5 NEXT")
            );
            assert_eq!(
                Statement::For(
                    Box::new(Expression::Identifier("i".to_string())),
                    Box::new(Expression::Const(Constant::Integer(1))),
                    Box::new(Expression::Const(Constant::Integer(10))),
                    Some(Box::new(Expression::UnaryExpression(
                        crate::ast::UnaryOp::Minus,
                        Box::new(Expression::Const(Constant::Integer(1)))
                    ))),
                    Vec::new()
                ),
                parse_statement("FOR i = 1 TO 10 STEP -1 NEXT")
            );
        }

    #[test]
    fn test_parse_block() {
        assert_eq!(Statement::Block(Vec::new()), parse_statement("BEGIN END"));
    }
    #[test]
    fn test_select_case() {
        assert_eq!(
            Statement::Select(
                Box::new(Expression::Identifier("I".to_string())),
                vec![
                    ElseIfBlock {
                        cond: Box::new(Expression::Const(Constant::Integer(1))),
                        block: vec![Statement::Call(
                            get_statement_definition("PRINT").unwrap(),
                            vec![Expression::Const(Constant::Integer(1))]
                        )]
                    },
                    ElseIfBlock {
                        cond: Box::new(Expression::Const(Constant::Integer(2))),
                        block: vec![Statement::Call(
                            get_statement_definition("PRINT").unwrap(),
                            vec![Expression::Const(Constant::Integer(2))]
                        )]
                    }
                ],
                Some(vec![Statement::Call(
                    get_statement_definition("PRINT").unwrap(),
                    vec![Expression::Const(Constant::Integer(3))]
                )])
            ),
            parse_statement(
                "SELECT CASE I\nCASE 1\n PRINT 1\nCASE 2\n  PRINT 2\nCASE ELSE\nPRINT 3 ENDSELECT"
            )
        );
        assert_eq!(
            Statement::DoWhile(
                Box::new(Expression::Const(Constant::Builtin(&BuiltinConst::TRUE))),
                Vec::new()
            ),
            parse_statement("WHILE (TRUE) DO ENDWHILE")
        );
    }
}*/
