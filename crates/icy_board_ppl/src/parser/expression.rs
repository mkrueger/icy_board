use super::{Parser, lexer::Token};
use crate::{
    ast::{
        ArrayInitializerExpression, BinaryExpression, ConstantExpression, Expression, FunctionCallExpression, IdentifierExpression, IndexerExpression,
        MemberReferenceExpression, ParensExpression, RecordLiteralExpression, RecordLiteralField, UnaryExpression,
    },
    executable::VariableType,
    parser::ParserErrorType,
};

impl Parser<'_> {
    pub fn parse_expression(&mut self) -> Option<Expression> {
        self.parse_nested(Self::parse_bool)
    }

    /// Runs one more level of expression recursion, so a source that nests deeper than
    /// the parser can afford is reported instead of exhausting the stack.
    fn parse_nested(&mut self, parse: impl FnOnce(&mut Self) -> Option<Expression>) -> Option<Expression> {
        if self.expression_depth >= super::MAX_EXPRESSION_DEPTH {
            self.report_error(self.save_token_span(), ParserErrorType::ExpressionNestingTooDeep(super::MAX_EXPRESSION_DEPTH));
            return None;
        }
        self.expression_depth += 1;
        let expression = parse(self);
        self.expression_depth -= 1;
        expression
    }

    /// Counts one more link of a left associative operator chain. The loops that read those
    /// chains cost the parser no recursion, but the tree they build is as deep as the chain is
    /// long and everything that walks it later does recurse.
    fn take_operator_link(&mut self, links: &mut usize) -> bool {
        *links += 1;
        if *links > super::MAX_OPERATOR_CHAIN {
            self.report_error(self.save_token_span(), ParserErrorType::ExpressionNestingTooDeep(super::MAX_OPERATOR_CHAIN));
            return false;
        }
        true
    }

    fn parse_bool(&mut self) -> Option<Expression> {
        let mut expr = self.parse_comparison()?;
        let mut links = 0;
        while self.get_cur_token() == Some(Token::Or) || self.get_cur_token() == Some(Token::And) {
            if !self.take_operator_link(&mut links) {
                return None;
            }
            let op_token = self.save_spanned_token();
            self.next_token();
            let right = self.parse_comparison();
            if let Some(e) = right {
                expr = Expression::Binary(BinaryExpression::new(expr, op_token, e));
            } else {
                return None;
            }
        }
        Some(expr)
    }

    fn parse_comparison(&mut self) -> Option<Expression> {
        let mut expr = self.parse_term()?;
        let mut links = 0;
        while self.get_cur_token() == Some(Token::Greater)
            || self.get_cur_token() == Some(Token::GreaterEq)
            || self.get_cur_token() == Some(Token::Lower)
            || self.get_cur_token() == Some(Token::LowerEq)
            || self.get_cur_token() == Some(Token::Eq)
            || self.get_cur_token() == Some(Token::NotEq)
        {
            if !self.take_operator_link(&mut links) {
                return None;
            }
            let op_token = self.save_spanned_token();
            self.next_token();
            let right = self.parse_term();
            if let Some(e) = right {
                expr = Expression::Binary(BinaryExpression::new(expr, op_token, e));
            } else {
                return None;
            }
        }

        Some(expr)
    }

    fn parse_term(&mut self) -> Option<Expression> {
        let mut expr = self.parse_factor()?;
        let mut links = 0;
        while self.get_cur_token() == Some(Token::Add) || self.get_cur_token() == Some(Token::Sub) {
            if !self.take_operator_link(&mut links) {
                return None;
            }
            let op_token = self.save_spanned_token();
            self.next_token();
            let right = self.parse_factor();
            if let Some(e) = right {
                expr = Expression::Binary(BinaryExpression::new(expr, op_token, e));
            } else {
                return None;
            }
        }

        Some(expr)
    }

    fn parse_factor(&mut self) -> Option<Expression> {
        let mut expr = self.parse_pow()?;
        let mut links = 0;
        while self.get_cur_token() == Some(Token::Mul) || self.get_cur_token() == Some(Token::Div) || self.get_cur_token() == Some(Token::Mod) {
            if !self.take_operator_link(&mut links) {
                return None;
            }
            let op_token = self.save_spanned_token();
            self.next_token();
            let right = self.parse_pow();
            if let Some(e) = right {
                expr = Expression::Binary(BinaryExpression::new(expr, op_token, e));
            } else {
                return None;
            }
        }
        Some(expr)
    }

    fn parse_pow(&mut self) -> Option<Expression> {
        let mut expr = self.parse_unary()?;
        let mut links = 0;
        while self.get_cur_token() == Some(Token::PoW) {
            if !self.take_operator_link(&mut links) {
                return None;
            }
            let op_token = self.save_spanned_token();
            self.next_token();
            let right = self.parse_unary();
            if let Some(right_expression) = right {
                expr = Expression::Binary(BinaryExpression::new(expr, op_token, right_expression));
            } else {
                return None;
            }
        }
        Some(expr)
    }

    fn parse_unary(&mut self) -> Option<Expression> {
        if self.get_cur_token() == Some(Token::Add) {
            let token = self.save_spanned_token();
            self.next_token();
            let expr = self.parse_nested(Self::parse_unary);
            if let Some(e) = expr {
                return Some(Expression::Unary(UnaryExpression::new(token, e)));
            }
        }
        if self.get_cur_token() == Some(Token::Sub) {
            let token = self.save_spanned_token();
            self.next_token();
            let expr = self.parse_nested(Self::parse_unary);
            if let Some(e) = expr {
                return Some(Expression::Unary(UnaryExpression::new(token, e)));
            }
        }
        if self.get_cur_token() == Some(Token::Not) {
            let token = self.save_spanned_token();
            self.next_token();
            let expr = self.parse_nested(Self::parse_unary);
            if let Some(e) = expr {
                return Some(Expression::Unary(UnaryExpression::new(token, e)));
            }
        }
        self.parse_function_call_expression()
    }
    fn parse_function_call_expression(&mut self) -> Option<Expression> {
        let primary = self.parse_primary();

        if let Some(expr) = primary {
            if self.get_cur_token() == Some(Token::LPar) {
                let call = self.parse_call_suffix(expr)?;
                return self.parse_member_chain(call);
            }
            Some(expr)
        } else {
            None
        }
    }

    /// Parses `(arguments)` onto a callee that is already parsed. The current token is the
    /// opening parenthesis.
    fn parse_call_suffix(&mut self, expr: Expression) -> Option<Expression> {
        let leftpar_token = self.save_spanned_token();

        self.next_token();
        let mut arguments = Vec::new();

        while self.get_cur_token() != Some(Token::RPar) {
            let Some(value) = self.parse_expression() else {
                self.error_reporter
                    .lock()
                    .unwrap()
                    .report_error(self.save_token_span(), ParserErrorType::InvalidToken(self.save_token()));
                self.next_token();
                return None;
            };
            arguments.push(value);
            if self.get_cur_token() == Some(Token::Comma) {
                self.next_token();
                continue;
            }

            if self.get_cur_token() != Some(Token::RPar) && self.get_cur_token() != Some(Token::Comma) {
                break;
            }
        }

        if self.get_cur_token() != Some(Token::RPar) {
            self.error_reporter
                .lock()
                .unwrap()
                .report_error(self.save_token_span(), ParserErrorType::MissingCloseParens(self.save_token()));
            return None;
        }
        let rightpar_token = self.save_spanned_token();
        self.next_token();

        Some(Expression::FunctionCall(FunctionCallExpression::new(
            expr,
            leftpar_token,
            arguments,
            rightpar_token,
        )))
    }

    /// Follows `.member` as far as it goes, so what a member answers may have members
    /// of its own, and any member in the chain may be called.
    pub(super) fn parse_member_chain(&mut self, expr: Expression) -> Option<Expression> {
        let mut expr = expr;
        // The chain is read in a loop but nests one level per link, so it needs the same
        // bound the recursive part of the parser keeps.
        let mut links = 0;
        loop {
            if matches!(self.get_cur_token(), Some(Token::Dot | Token::LBracket)) {
                links += 1;
                if links > super::MAX_EXPRESSION_DEPTH {
                    self.report_error(self.save_token_span(), ParserErrorType::ExpressionNestingTooDeep(super::MAX_EXPRESSION_DEPTH));
                    return None;
                }
            }
            if self.get_cur_token() == Some(Token::Dot) {
                let dot_token: super::lexer::Spanned<Token> = self.save_spanned_token();
                self.next_token();
                let token = self.save_spanned_token();
                // After a dot a name is a member, so one that also names a built-in constant
                // is read as the member it is written as.
                let identifier_token = match &token.token {
                    Token::Identifier(_) => token,
                    Token::Const(crate::ast::Constant::Builtin(constant)) => {
                        super::lexer::Spanned::new(Token::Identifier(unicase::Ascii::new(constant.name.to_string())), token.span.clone())
                    }
                    _ => {
                        self.error_reporter
                            .lock()
                            .unwrap()
                            .report_error(self.save_token_span(), ParserErrorType::IdentifierExpected(self.save_token()));
                        return None;
                    }
                };
                self.next_token();
                expr = Expression::MemberReference(MemberReferenceExpression::new(expr, dot_token, identifier_token));
                if self.get_cur_token() == Some(Token::LPar) {
                    expr = self.parse_call_suffix(expr)?;
                }
                continue;
            }

            if self.get_cur_token() == Some(Token::LBracket) {
                let bracket_token = self.save_spanned_token();
                self.next_token();
                let mut arguments = Vec::new();
                while self.get_cur_token() != Some(Token::RBracket) {
                    let Some(value) = self.parse_expression() else {
                        self.report_error(self.save_token_span(), ParserErrorType::ExpressionExpected(self.save_token()));
                        return None;
                    };
                    arguments.push(value);
                    if self.get_cur_token() == Some(Token::Comma) {
                        self.next_token();
                    }
                }
                let close_token = self.save_spanned_token();
                self.next_token();
                let getter = super::lexer::Spanned::new(Token::Identifier(unicase::Ascii::new("<get>".to_string())), bracket_token.span.clone());
                expr = Expression::FunctionCall(FunctionCallExpression::new(
                    Expression::MemberReference(MemberReferenceExpression::new(expr, bracket_token, getter)),
                    self.save_spanned_token(),
                    arguments,
                    close_token,
                ));
                continue;
            }

            break;
        }
        Some(expr)
    }

    fn parse_primary(&mut self) -> Option<Expression> {
        let cur_token = self.cur_token.clone()?;

        let expr = match &cur_token.token {
            Token::Const(c) => {
                self.next_token();
                Some(Expression::Const(ConstantExpression::new(cur_token.clone(), c.clone())))
            }
            Token::Identifier(_id) => {
                let identifier_token = self.save_spanned_token();
                let variable_type = self.get_variable_type();
                self.next_token();
                if self.lang_version >= 400
                    && self.get_cur_token() == Some(Token::LBrace)
                    && let Some(VariableType::UserData(type_id)) = variable_type
                    && self.type_registry.is_record_type(type_id)
                {
                    let lbrace_token = self.save_spanned_token();
                    self.next_token();
                    let mut fields = Vec::new();
                    self.skip_eol_and_comments();
                    while self.get_cur_token() != Some(Token::RBrace) {
                        let Some(Token::Identifier(_)) = self.get_cur_token() else {
                            self.report_error(self.save_token_span(), ParserErrorType::IdentifierExpected(self.save_token()));
                            return None;
                        };
                        let field_token = self.save_spanned_token();
                        self.next_token();
                        if self.get_cur_token() != Some(Token::Eq) {
                            self.report_error(self.save_token_span(), ParserErrorType::InvalidToken(self.save_token()));
                            return None;
                        }
                        self.next_token();
                        let Some(value) = self.parse_expression() else {
                            self.report_error(self.save_token_span(), ParserErrorType::ExpressionExpected(self.save_token()));
                            return None;
                        };
                        fields.push(RecordLiteralField::new(field_token, value));
                        self.skip_eol_and_comments();
                        if self.get_cur_token() == Some(Token::Comma) {
                            self.next_token();
                            self.skip_eol_and_comments();
                        } else if self.get_cur_token() != Some(Token::RBrace) {
                            self.report_error(self.save_token_span(), ParserErrorType::CommaOrRBraceExpected);
                            return None;
                        }
                    }
                    let rbrace_token = self.save_spanned_token();
                    self.next_token();
                    return Some(Expression::RecordLiteral(RecordLiteralExpression::new(
                        identifier_token,
                        VariableType::UserData(type_id),
                        lbrace_token,
                        fields,
                        rbrace_token,
                    )));
                }
                if self.lang_version >= 350 && self.get_cur_token() == Some(Token::LBracket) {
                    let leftpar_token = self.save_spanned_token();

                    self.next_token();
                    let mut arguments = Vec::new();

                    while self.get_cur_token() != Some(Token::RBracket) {
                        let Some(value) = self.parse_expression() else {
                            self.error_reporter
                                .lock()
                                .unwrap()
                                .report_error(self.save_token_span(), ParserErrorType::InvalidToken(self.save_token()));
                            self.next_token();
                            return None;
                        };
                        arguments.push(value);
                        if self.get_cur_token() == Some(Token::Comma) {
                            self.next_token();
                            continue;
                        }

                        if self.get_cur_token() != Some(Token::RBracket) && self.get_cur_token() != Some(Token::Comma) {
                            break;
                        }
                    }

                    if self.get_cur_token() != Some(Token::RBracket) {
                        self.error_reporter
                            .lock()
                            .unwrap()
                            .report_error(self.save_token_span(), ParserErrorType::MissingCloseBracket(self.save_token()));
                        return None;
                    }
                    let rightpar_token = self.save_spanned_token();

                    self.next_token();

                    let indexer = Expression::Indexer(IndexerExpression::new(identifier_token, leftpar_token, arguments, rightpar_token));
                    return self.parse_member_chain(indexer);
                }
                Some(Expression::Identifier(IdentifierExpression::new(identifier_token)))
            }

            Token::LPar => {
                self.next_token();
                let Some(expr) = self.parse_expression() else {
                    self.error_reporter
                        .lock()
                        .unwrap()
                        .report_error(self.save_token_span(), ParserErrorType::ExpressionExpected(self.save_token()));
                    return None;
                };
                let rpar_token = self.save_spanned_token();
                if rpar_token.token != Token::RPar {
                    self.error_reporter
                        .lock()
                        .unwrap()
                        .report_error(self.save_token_span(), ParserErrorType::MissingCloseParens(self.save_token()));
                    return None;
                }
                self.next_token();
                let ret = Expression::Parens(ParensExpression::new(cur_token, expr, rpar_token));
                Some(ret)
            }

            Token::LBrace => {
                let lbrace_token = self.save_spanned_token();
                self.next_token();
                let mut list = Vec::new();
                while self.get_cur_token() != Some(Token::RBrace) {
                    self.skip_eol_and_comments();
                    let Some(expr) = self.parse_expression() else {
                        self.error_reporter
                            .lock()
                            .unwrap()
                            .report_error(self.save_token_span(), ParserErrorType::ExpressionExpected(self.save_token()));
                        return None;
                    };
                    list.push(expr);
                    self.skip_eol_and_comments();

                    match self.get_cur_token() {
                        Some(Token::RBrace) => break,
                        Some(Token::Comma) => {
                            self.next_token();
                            self.skip_eol_and_comments();
                        }
                        _ => {
                            self.error_reporter
                                .lock()
                                .unwrap()
                                .report_error(self.save_token_span(), ParserErrorType::CommaOrRBraceExpected);
                            return None;
                        }
                    }
                }
                let rbrace_token = self.save_spanned_token();

                self.next_token();
                Some(Expression::ArrayInitializer(ArrayInitializerExpression::new(lbrace_token, list, rbrace_token)))
            }
            _ => None,
        };

        match expr {
            Some(expr) if matches!(self.get_cur_token(), Some(Token::Dot | Token::LBracket)) => self.parse_member_chain(expr),
            expr => expr,
        }
    }
}
