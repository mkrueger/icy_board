use std::fmt;

use crate::{
    executable::StatementDefinition,
    parser::lexer::{CommentType, Spanned, Token},
};

use super::{
    AstVisitor, AstVisitorMut, Constant, ConstantExpression, Expression, UnaryExpression, UnaryOp,
    VariableDeclarationStatement, VariableSpecifier,
};

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    Comment(CommentAstNode),
    Block(BlockStatement),

    If(IfStatement),
    IfThen(IfThenStatement),
    Select(SelectStatement),

    While(WhileStatement),
    WhileDo(WhileDoStatement),
    For(ForStatement),
    Break(BreakStatement),
    Continue(ContinueStatement),
    Gosub(GosubStatement),
    Return(ReturnStatement),
    Let(LetStatement),
    Goto(GotoStatement),
    Label(LabelStatement),
    Call(ProcedureCallStatement),
    PredifinedCall(PredefinedCallStatement),

    VariableDeclaration(VariableDeclarationStatement),
}

impl Statement {
    pub fn visit<T: Default, V: AstVisitor<T>>(&self, visitor: &mut V) -> T {
        match self {
            Statement::Comment(s) => visitor.visit_comment(s),
            Statement::Block(s) => visitor.visit_block_statement(s),
            Statement::If(s) => visitor.visit_if_statement(s),
            Statement::IfThen(s) => visitor.visit_if_then_statement(s),
            Statement::Select(s) => visitor.visit_select_statement(s),
            Statement::While(s) => visitor.visit_while_statement(s),
            Statement::WhileDo(s) => visitor.visit_while_do_statement(s),
            Statement::For(s) => visitor.visit_for_statement(s),
            Statement::Break(s) => visitor.visit_break_statement(s),
            Statement::Continue(s) => visitor.visit_continue_statement(s),
            Statement::Gosub(s) => visitor.visit_gosub_statement(s),
            Statement::Return(s) => visitor.visit_return_statement(s),
            Statement::Let(s) => visitor.visit_let_statement(s),
            Statement::Goto(s) => visitor.visit_goto_statement(s),
            Statement::Label(s) => visitor.visit_label_statement(s),
            Statement::Call(s) => visitor.visit_procedure_call_statement(s),
            Statement::PredifinedCall(s) => visitor.visit_predefined_call_statement(s),
            Statement::VariableDeclaration(s) => visitor.visit_variable_declaration_statement(s),
        }
    }

    #[must_use]
    pub fn visit_mut<V: AstVisitorMut>(&self, visitor: &mut V) -> Self {
        match self {
            Statement::Comment(s) => visitor.visit_comment_statement(s),
            Statement::Block(s) => Statement::Block(visitor.visit_block(s)),
            Statement::If(s) => visitor.visit_if_statement(s),
            Statement::IfThen(s) => visitor.visit_if_then_statement(s),
            Statement::Select(s) => visitor.visit_select_statement(s),
            Statement::While(s) => visitor.visit_while_statement(s),
            Statement::WhileDo(s) => visitor.visit_while_do_statement(s),
            Statement::For(s) => visitor.visit_for_statement(s),
            Statement::Break(s) => visitor.visit_break_statement(s),
            Statement::Continue(s) => visitor.visit_continue_statement(s),
            Statement::Gosub(s) => visitor.visit_gosub_statement(s),
            Statement::Return(s) => visitor.visit_return_statement(s),
            Statement::Let(s) => visitor.visit_let_statement(s),
            Statement::Goto(s) => visitor.visit_goto_statement(s),
            Statement::Label(s) => visitor.visit_label_statement(s),
            Statement::Call(s) => visitor.visit_procedure_call_statement(s),
            Statement::PredifinedCall(s) => visitor.visit_predefined_call_statement(s),
            Statement::VariableDeclaration(s) => visitor.visit_variable_declaration_statement(s),
        }
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn is_similar(&self, check: &Statement) -> bool {
        match (self, check) {
            (Statement::Comment(c1), Statement::Comment(c2)) => c1.get_comment() == c2.get_comment(),
            (Statement::Return(_), Statement::Return(_)) |
            (Statement::Break(_), Statement::Break(_)) |
            (Statement::Continue(_), Statement::Continue(_)) => true,

            (Statement::Block(_), Statement::Block(_)) => panic!("Not implemented PPL has no blocks, it's just used as a container for statements during compiling."),

            (Statement::If(i1), Statement::If(i2)) => i1.get_condition().is_similar(i2.get_condition()) && i1.get_statement().is_similar(i2.get_statement()),
            (Statement::IfThen(i1), Statement::IfThen(i2)) => {
                if !(i1.get_condition().is_similar(i2.get_condition())) || i1.get_statements().len() != i2.get_statements().len() {
                    return false;
                }
                for i in 0..i1.get_statements().len() {
                    if !i1.get_statements()[i].is_similar(&i2.get_statements()[i]) {
                        return false;
                    }
                }
                true
            }
            (Statement::Select(s1), Statement::Select(s2)) => {
                if !s1.get_expression().is_similar(s2.get_expression()) {
                    return false;
                }
                if s1.get_case_blocks().len() != s2.get_case_blocks().len() {
                    return false;
                }
                for i in 0..s1.get_case_blocks().len() {
                    if !s1.get_case_blocks()[i].is_similar(&s2.get_case_blocks()[i]) {
                        return false;
                    }
                }
                if s1.get_default_statements().len() != s2.get_default_statements().len() {
                    return false;
                }
                for i in 0..s1.get_default_statements().len() {
                    if !s1.get_default_statements()[i].is_similar(&s2.get_default_statements()[i]) {
                        return false;
                    }
                }
                true
            }
            (Statement::While(w1), Statement::While(w2)) => {
                w1.get_condition().is_similar(w2.get_condition()) && w1.get_statement().is_similar(w2.get_statement())
            }
            (Statement::WhileDo(i1), Statement::WhileDo(i2)) => {
                if !(i1.get_condition().is_similar(i2.get_condition())) || i1.get_statements().len() != i2.get_statements().len() {
                    return false;
                }
                for i in 0..i1.get_statements().len() {
                    if !i1.get_statements()[i].is_similar(&i2.get_statements()[i]) {
                        return false;
                    }
                }
                true
            }
            (Statement::For(f1), Statement::For(f2)) => {
                if f1.get_identifier() != f2.get_identifier() {
                    return false;
                }
                if !f1.get_start_expr().is_similar(f2.get_start_expr()) {
                    return false;
                }
                if !f1.get_end_expr().is_similar(f2.get_end_expr()) {
                    return false;
                }
                if f1.get_step_expr().is_none() && f2.get_step_expr().is_none() {
                    return true;
                }
                if f1.get_step_expr().is_none() || f2.get_step_expr().is_none() {
                    return false;
                }
                if !f1.get_step_expr().as_ref().unwrap().is_similar(f2.get_step_expr().as_ref().unwrap()) {
                    return false;
                }

                if f1.get_statements().len() != f2.get_statements().len() {
                    return false;
                }
                for i in 0..f1.get_statements().len() {
                    if !f1.get_statements()[i].is_similar(&f2.get_statements()[i]) {
                        return false;
                    }
                }
                true
            }
            (Statement::Let(l1), Statement::Let(l2)) =>
                l1.get_identifier() == l2.get_identifier() &&
                l1.get_value_expression().is_similar(l2.get_value_expression()),
            (Statement::Goto(g1), Statement::Goto(g2)) => g1.get_label() == g2.get_label(),
            (Statement::Gosub(g1), Statement::Gosub(g2)) => g1.get_label() == g2.get_label(),
            (Statement::Label(g1), Statement::Label(g2)) => g1.get_label() == g2.get_label(),
            (Statement::Call(c1), Statement::Call(c2)) => {
                if c1.get_identifier() != c2.get_identifier() {
                    return false;
                }
                if c1.get_arguments().len() != c2.get_arguments().len() {
                    return false;
                }
                for i in 0..c1.get_arguments().len() {
                    if !c1.get_arguments()[i].is_similar(&c2.get_arguments()[i]) {
                        return false;
                    }
                }
                true
            }
            (Statement::PredifinedCall(p1), Statement::PredifinedCall(p2)) => {
                if p1.get_identifier() != p2.get_identifier() {
                    return false;
                }
                if p1.get_arguments().len() != p2.get_arguments().len() {
                    return false;
                }
                for i in 0..p1.get_arguments().len() {
                    if !p1.get_arguments()[i].is_similar(&p2.get_arguments()[i]) {
                        return false;
                    }
                }
                true
            },
            (Statement::VariableDeclaration(v1), Statement::VariableDeclaration(v2)) => {
                if v1.get_variable_type() != v2.get_variable_type() {
                    return false;
                }
                if v1.get_variables().len() != v2.get_variables().len() {
                    return false;
                }
                for i in 0..v1.get_variables().len() {
                    if v1.get_variables()[i].get_identifier() != v2.get_variables()[i].get_identifier() {
                        return false;
                    }
                    for (a, b) in v1.get_variables()[i].get_dimensions().iter().zip(v2.get_variables()[i].get_dimensions().iter()) {
                        if a.get_dimension() != b.get_dimension() {
                            return false;
                        }
                    }
                }

                true
            }
            _ => false,
        }
    }
}

impl fmt::Display for Statement {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output_visitor = crate::ast::output_visitor::OutputVisitor::default();
        self.visit(&mut output_visitor);
        write!(f, "{}", output_visitor.output)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct CommentAstNode {
    comment_token: Spanned<Token>,
}

impl CommentAstNode {
    pub fn new(comment_token: Spanned<Token>) -> Self {
        Self { comment_token }
    }

    pub fn empty(comment: impl Into<String>) -> Self {
        Self {
            comment_token: Spanned::create_empty(Token::Comment(
                CommentType::SingleLineSemicolon,
                comment.into(),
            )),
        }
    }

    pub fn get_comment_token(&self) -> &Spanned<Token> {
        &self.comment_token
    }

    /// Returns a reference to the get identifier of this [`IdentifierExpression`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn get_comment(&self) -> &String {
        if let Token::Comment(_ct, id) = &self.comment_token.token {
            return id;
        }
        if let Token::UseFuncs(_ct, id) = &self.comment_token.token {
            return id;
        }
        panic!("Expected identifier token")
    }

    /// # Panics
    ///
    /// Panics if .
    pub fn get_comment_type(&self) -> CommentType {
        if let Token::Comment(ct, _id) = &self.comment_token.token {
            return *ct;
        }
        if let Token::UseFuncs(ct, _id) = &self.comment_token.token {
            return *ct;
        }
        panic!("Expected comment token")
    }

    pub fn create_empty_statement(identifier: impl Into<String>) -> Statement {
        Statement::Comment(CommentAstNode::empty(identifier))
    }
}

impl fmt::Display for CommentAstNode {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}{}", self.get_comment_type(), self.get_comment())
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct BreakStatement {
    break_token: Spanned<Token>,
}

impl BreakStatement {
    pub fn new(break_token: Spanned<Token>) -> Self {
        Self { break_token }
    }

    pub fn empty() -> Self {
        Self {
            break_token: Spanned::create_empty(Token::Break),
        }
    }

    pub fn get_break_token(&self) -> &Spanned<Token> {
        &self.break_token
    }

    pub fn create_empty_statement() -> Statement {
        Statement::Break(BreakStatement::empty())
    }
}

impl fmt::Display for BreakStatement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Break")
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ContinueStatement {
    continue_token: Spanned<Token>,
}

impl ContinueStatement {
    pub fn new(continue_token: Spanned<Token>) -> Self {
        Self { continue_token }
    }

    pub fn empty() -> Self {
        Self {
            continue_token: Spanned::create_empty(Token::Break),
        }
    }

    pub fn get_continue_token(&self) -> &Spanned<Token> {
        &self.continue_token
    }

    pub fn create_empty_statement() -> Statement {
        Statement::Continue(ContinueStatement::empty())
    }
}

impl fmt::Display for ContinueStatement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Continue")
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ReturnStatement {
    return_token: Spanned<Token>,
}

impl ReturnStatement {
    pub fn new(return_token: Spanned<Token>) -> Self {
        Self { return_token }
    }

    pub fn empty() -> Self {
        Self {
            return_token: Spanned::create_empty(Token::Return),
        }
    }

    pub fn get_return_token(&self) -> &Spanned<Token> {
        &self.return_token
    }

    pub fn create_empty_statement() -> Statement {
        Statement::Return(ReturnStatement::empty())
    }
}

impl fmt::Display for ReturnStatement {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "Return")
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct BlockStatement {
    begin_token: Spanned<Token>,
    statements: Vec<Statement>,
    end_token: Spanned<Token>,
}

impl BlockStatement {
    pub fn new(
        begin_token: Spanned<Token>,
        statements: Vec<Statement>,
        end_token: Spanned<Token>,
    ) -> Self {
        Self {
            begin_token,
            statements,
            end_token,
        }
    }

    pub fn empty(statements: Vec<Statement>) -> Self {
        Self {
            begin_token: Spanned::create_empty(Token::Identifier(unicase::Ascii::new(
                "BEGIN".to_string(),
            ))),
            statements,
            end_token: Spanned::create_empty(Token::Identifier(unicase::Ascii::new(
                "END".to_string(),
            ))),
        }
    }

    pub fn get_begin_token(&self) -> &Spanned<Token> {
        &self.begin_token
    }

    pub fn get_statements(&self) -> &Vec<Statement> {
        &self.statements
    }

    pub fn get_statements_mut(&mut self) -> &mut Vec<Statement> {
        &mut self.statements
    }

    pub fn get_end_token(&self) -> &Spanned<Token> {
        &self.begin_token
    }

    pub fn create_empty_statement(statements: Vec<Statement>) -> Statement {
        Statement::Block(BlockStatement::empty(statements))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct IfStatement {
    if_token: Spanned<Token>,
    leftpar_token: Spanned<Token>,
    condition: Box<Expression>,
    rightpar_token: Spanned<Token>,
    statement: Box<Statement>,
}

impl IfStatement {
    pub fn new(
        if_token: Spanned<Token>,
        leftpar_token: Spanned<Token>,
        condition: Expression,
        rightpar_token: Spanned<Token>,
        statement: Statement,
    ) -> Self {
        Self {
            if_token,
            leftpar_token,
            condition: Box::new(condition),
            rightpar_token,
            statement: Box::new(statement),
        }
    }

    pub fn empty(condition: Expression, statement: Statement) -> Self {
        Self {
            if_token: Spanned::create_empty(Token::While),
            leftpar_token: Spanned::create_empty(Token::LPar),
            condition: Box::new(condition),
            rightpar_token: Spanned::create_empty(Token::RPar),
            statement: Box::new(statement),
        }
    }

    pub fn get_if_token(&self) -> &Spanned<Token> {
        &self.if_token
    }
    pub fn get_lpar_token(&self) -> &Spanned<Token> {
        &self.leftpar_token
    }

    pub fn get_condition(&self) -> &Expression {
        &self.condition
    }

    pub fn get_condition_mut(&mut self) -> &mut Expression {
        &mut self.condition
    }

    pub fn get_rpar_token(&self) -> &Spanned<Token> {
        &self.rightpar_token
    }

    pub fn get_statement(&self) -> &Statement {
        &self.statement
    }

    pub fn get_statement_mut(&mut self) -> &mut Statement {
        &mut self.statement
    }

    pub fn set_statement(&mut self, statement: Statement) {
        self.statement = Box::new(statement);
    }

    pub fn create_empty_statement(condition: Expression, statement: Statement) -> Statement {
        Statement::If(IfStatement::empty(condition, statement))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElseIfBlock {
    elseif_token: Spanned<Token>,
    leftpar_token: Spanned<Token>,
    cond: Box<Expression>,
    rightpar_token: Spanned<Token>,
    then_token: Option<Spanned<Token>>,
    statements: Vec<Statement>,
}

impl ElseIfBlock {
    pub fn new(
        elseif_token: Spanned<Token>,
        leftpar_token: Spanned<Token>,
        cond: Expression,
        rightpar_token: Spanned<Token>,
        then_token: Option<Spanned<Token>>,
        statements: Vec<Statement>,
    ) -> Self {
        Self {
            elseif_token,
            leftpar_token,
            cond: Box::new(cond),
            rightpar_token,
            then_token,
            statements,
        }
    }

    pub fn empty(cond: Expression, statements: Vec<Statement>) -> Self {
        Self {
            elseif_token: Spanned::create_empty(Token::ElseIf),
            leftpar_token: Spanned::create_empty(Token::LPar),
            cond: Box::new(cond),
            rightpar_token: Spanned::create_empty(Token::RPar),
            then_token: None,
            statements,
        }
    }

    pub fn get_elseif_token(&self) -> &Spanned<Token> {
        &self.elseif_token
    }

    pub fn get_leftpar_token(&self) -> &Spanned<Token> {
        &self.leftpar_token
    }

    pub fn get_condition(&self) -> &Expression {
        &self.cond
    }

    pub fn get_condition_mut(&mut self) -> &mut Expression {
        &mut self.cond
    }

    pub fn get_rightpar_token(&self) -> &Spanned<Token> {
        &self.rightpar_token
    }

    pub fn get_then_token(&self) -> &Option<Spanned<Token>> {
        &self.then_token
    }

    pub fn get_statements(&self) -> &Vec<Statement> {
        &self.statements
    }

    pub fn get_statements_mut(&mut self) -> &mut Vec<Statement> {
        &mut self.statements
    }

    #[must_use]
    pub fn visit_mut<V: AstVisitorMut>(&self, visitor: &mut V) -> Self {
        visitor.visit_else_if_block(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct ElseBlock {
    else_token: Spanned<Token>,
    statements: Vec<Statement>,
}

impl ElseBlock {
    pub fn new(else_token: Spanned<Token>, statements: Vec<Statement>) -> Self {
        Self {
            else_token,
            statements,
        }
    }

    pub fn empty(statements: Vec<Statement>) -> Self {
        Self {
            else_token: Spanned::create_empty(Token::Else),
            statements,
        }
    }

    pub fn get_else_token(&self) -> &Spanned<Token> {
        &self.else_token
    }

    pub fn get_statements(&self) -> &Vec<Statement> {
        &self.statements
    }

    pub fn get_statements_mut(&mut self) -> &mut Vec<Statement> {
        &mut self.statements
    }

    #[must_use]
    pub fn visit_mut<V: AstVisitorMut>(&self, visitor: &mut V) -> Self {
        visitor.visit_else_block(self)
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct IfThenStatement {
    if_token: Spanned<Token>,
    leftpar_token: Spanned<Token>,
    condition: Box<Expression>,
    rightpar_token: Spanned<Token>,
    then_token: Spanned<Token>,
    statements: Vec<Statement>,

    else_if_blocks: Vec<ElseIfBlock>,
    else_block: Option<ElseBlock>,
    endif_token: Spanned<Token>,
}

impl IfThenStatement {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        if_token: Spanned<Token>,
        leftpar_token: Spanned<Token>,
        condition: Expression,
        rightpar_token: Spanned<Token>,
        then_token: Spanned<Token>,
        statements: Vec<Statement>,
        else_if_blocks: Vec<ElseIfBlock>,
        else_block: Option<ElseBlock>,
        endif_token: Spanned<Token>,
    ) -> Self {
        Self {
            if_token,
            leftpar_token,
            condition: Box::new(condition),
            rightpar_token,
            then_token,
            statements,
            else_if_blocks,
            else_block,
            endif_token,
        }
    }

    pub fn get_if_token(&self) -> &Spanned<Token> {
        &self.if_token
    }
    pub fn get_lpar_token(&self) -> &Spanned<Token> {
        &self.leftpar_token
    }

    pub fn get_condition(&self) -> &Expression {
        &self.condition
    }

    pub fn get_condition_mut(&mut self) -> &mut Expression {
        &mut self.condition
    }

    pub fn get_rpar_token(&self) -> &Spanned<Token> {
        &self.rightpar_token
    }

    pub fn get_then_token(&self) -> &Spanned<Token> {
        &self.then_token
    }

    pub fn get_statements(&self) -> &Vec<Statement> {
        &self.statements
    }

    pub fn get_statements_mut(&mut self) -> &mut Vec<Statement> {
        &mut self.statements
    }

    pub fn get_else_if_blocks(&self) -> &Vec<ElseIfBlock> {
        &self.else_if_blocks
    }

    pub fn get_else_if_blocks_mut(&mut self) -> &mut Vec<ElseIfBlock> {
        &mut self.else_if_blocks
    }

    pub fn get_else_block(&self) -> &Option<ElseBlock> {
        &self.else_block
    }

    pub fn get_else_block_mut(&mut self) -> &mut Option<ElseBlock> {
        &mut self.else_block
    }

    pub fn get_endif_token(&self) -> &Spanned<Token> {
        &self.endif_token
    }

    pub fn empty(
        condition: Expression,
        statements: Vec<Statement>,
        else_if_blocks: Vec<ElseIfBlock>,
        else_block: Option<ElseBlock>,
    ) -> Self {
        Self {
            if_token: Spanned::create_empty(Token::If),
            leftpar_token: Spanned::create_empty(Token::LPar),
            condition: Box::new(condition),
            rightpar_token: Spanned::create_empty(Token::RPar),
            then_token: Spanned::create_empty(Token::Identifier(unicase::Ascii::new(
                "THEN".to_string(),
            ))),
            statements,
            else_if_blocks,
            else_block,
            endif_token: Spanned::create_empty(Token::EndIf),
        }
    }

    pub fn create_empty_statement(
        condition: Expression,
        statements: Vec<Statement>,
        else_if_blocks: Vec<ElseIfBlock>,
        else_block: Option<ElseBlock>,
    ) -> Statement {
        Statement::IfThen(IfThenStatement::empty(
            condition,
            statements,
            else_if_blocks,
            else_block,
        ))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct WhileStatement {
    while_token: Spanned<Token>,
    leftpar_token: Spanned<Token>,
    condition: Box<Expression>,
    rightpar_token: Spanned<Token>,
    statement: Box<Statement>,
}

impl WhileStatement {
    pub fn new(
        while_token: Spanned<Token>,
        leftpar_token: Spanned<Token>,
        condition: Expression,
        rightpar_token: Spanned<Token>,
        statement: Statement,
    ) -> Self {
        Self {
            while_token,
            leftpar_token,
            condition: Box::new(condition),
            rightpar_token,
            statement: Box::new(statement),
        }
    }

    pub fn empty(condition: Expression, statement: Statement) -> Self {
        Self {
            while_token: Spanned::create_empty(Token::While),
            leftpar_token: Spanned::create_empty(Token::LPar),
            condition: Box::new(condition),
            rightpar_token: Spanned::create_empty(Token::RPar),
            statement: Box::new(statement),
        }
    }

    pub fn get_while_token(&self) -> &Spanned<Token> {
        &self.while_token
    }
    pub fn get_lpar_token(&self) -> &Spanned<Token> {
        &self.leftpar_token
    }

    pub fn get_condition(&self) -> &Expression {
        &self.condition
    }

    pub fn get_condition_mut(&mut self) -> &mut Expression {
        &mut self.condition
    }

    pub fn get_rpar_token(&self) -> &Spanned<Token> {
        &self.rightpar_token
    }

    pub fn get_statement(&self) -> &Statement {
        &self.statement
    }

    pub fn get_statement_mut(&mut self) -> &mut Statement {
        &mut self.statement
    }

    pub fn create_empty_statement(condition: Expression, statement: Statement) -> Statement {
        Statement::While(WhileStatement::empty(condition, statement))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct WhileDoStatement {
    while_token: Spanned<Token>,
    leftpar_token: Spanned<Token>,
    condition: Box<Expression>,
    rightpar_token: Spanned<Token>,
    do_token: Spanned<Token>,
    statements: Vec<Statement>,
    endwhile_token: Spanned<Token>,
}

impl WhileDoStatement {
    pub fn new(
        while_token: Spanned<Token>,
        leftpar_token: Spanned<Token>,
        condition: Expression,
        rightpar_token: Spanned<Token>,
        do_token: Spanned<Token>,
        statements: Vec<Statement>,
        endwhile_token: Spanned<Token>,
    ) -> Self {
        Self {
            while_token,
            leftpar_token,
            condition: Box::new(condition),
            rightpar_token,
            do_token,
            statements,
            endwhile_token,
        }
    }

    pub fn empty(condition: Expression, statements: Vec<Statement>) -> Self {
        Self {
            while_token: Spanned::create_empty(Token::While),
            leftpar_token: Spanned::create_empty(Token::LPar),
            condition: Box::new(condition),
            rightpar_token: Spanned::create_empty(Token::RPar),
            do_token: Spanned::create_empty(Token::Identifier(unicase::Ascii::new(
                "Do".to_string(),
            ))),
            statements,
            endwhile_token: Spanned::create_empty(Token::EndWhile),
        }
    }

    pub fn get_while_token(&self) -> &Spanned<Token> {
        &self.while_token
    }

    pub fn get_lpar_token(&self) -> &Spanned<Token> {
        &self.leftpar_token
    }

    pub fn get_condition(&self) -> &Expression {
        &self.condition
    }

    pub fn get_condition_mut(&mut self) -> &mut Expression {
        &mut self.condition
    }

    pub fn get_rpar_token(&self) -> &Spanned<Token> {
        &self.rightpar_token
    }

    pub fn get_do_token(&self) -> &Spanned<Token> {
        &self.do_token
    }

    pub fn get_statements(&self) -> &Vec<Statement> {
        &self.statements
    }

    pub fn get_statements_mut(&mut self) -> &mut Vec<Statement> {
        &mut self.statements
    }

    pub fn get_endwhile_token(&self) -> &Spanned<Token> {
        &self.endwhile_token
    }

    pub fn create_empty_statement(condition: Expression, statements: Vec<Statement>) -> Statement {
        Statement::WhileDo(WhileDoStatement::empty(condition, statements))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ForStatement {
    for_token: Spanned<Token>,
    identifier_token: Spanned<Token>,
    eq_token: Spanned<Token>,
    start_expr: Box<Expression>,
    to_token: Spanned<Token>,
    end_expr: Box<Expression>,
    step_token: Option<Spanned<Token>>,
    step_expr: Option<Box<Expression>>,
    statements: Vec<Statement>,
    next_token: Spanned<Token>,
    next_identifier_token: Option<Spanned<Token>>,
}

impl ForStatement {
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        for_token: Spanned<Token>,
        identifier_token: Spanned<Token>,
        eq_token: Spanned<Token>,
        start_expr: Expression,
        to_token: Spanned<Token>,
        end_expr: Expression,
        step_token: Option<Spanned<Token>>,
        step_expr: Option<Box<Expression>>,
        statements: Vec<Statement>,
        next_token: Spanned<Token>,
        next_identifier_token: Option<Spanned<Token>>,
    ) -> Self {
        Self {
            for_token,
            identifier_token,
            eq_token,
            start_expr: Box::new(start_expr),
            to_token,
            end_expr: Box::new(end_expr),
            step_token,
            step_expr,
            statements,
            next_token,
            next_identifier_token,
        }
    }

    pub fn empty(
        variable_name: unicase::Ascii<String>,
        start_expr: Expression,
        end_expr: Expression,
        step_expr: Option<Box<Expression>>,
        statements: Vec<Statement>,
    ) -> Self {
        Self {
            for_token: Spanned::create_empty(Token::For),
            identifier_token: Spanned::create_empty(Token::Identifier(variable_name)),
            eq_token: Spanned::create_empty(Token::Eq),
            start_expr: Box::new(start_expr),
            to_token: Spanned::create_empty(Token::Identifier(unicase::Ascii::new(
                "TO".to_string(),
            ))),
            end_expr: Box::new(end_expr),
            step_token: step_expr.as_ref().map(|_| {
                Spanned::create_empty(Token::Identifier(unicase::Ascii::new("Step".to_string())))
            }),
            step_expr,
            statements,
            next_token: Spanned::create_empty(Token::Next),
            next_identifier_token: None,
        }
    }

    pub fn get_for_token(&self) -> &Spanned<Token> {
        &self.for_token
    }

    pub fn get_identifier_token(&self) -> &Spanned<Token> {
        &self.identifier_token
    }

    /// Returns a reference to the get identifier of this [`ForStatement`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn get_identifier(&self) -> &unicase::Ascii<String> {
        if let Token::Identifier(id) = &self.identifier_token.token {
            return id;
        }
        panic!("Expected identifier token")
    }

    pub fn set_identifier(&mut self, new_id: unicase::Ascii<String>) {
        if let Token::Identifier(id) = &mut self.identifier_token.token {
            *id = new_id;
        }
    }

    pub fn get_eq_token(&self) -> &Spanned<Token> {
        &self.eq_token
    }

    pub fn get_start_expr(&self) -> &Expression {
        &self.start_expr
    }

    pub fn get_start_expr_mut(&mut self) -> &mut Expression {
        &mut self.start_expr
    }

    pub fn get_to_token(&self) -> &Spanned<Token> {
        &self.to_token
    }

    pub fn get_end_expr(&self) -> &Expression {
        &self.end_expr
    }

    pub fn get_end_expr_mut(&mut self) -> &mut Expression {
        &mut self.end_expr
    }

    pub fn get_step_token(&self) -> &Option<Spanned<Token>> {
        &self.step_token
    }

    pub fn get_step_expr(&self) -> &Option<Box<Expression>> {
        &self.step_expr
    }

    pub fn get_step_expr_mut(&mut self) -> &mut Option<Box<Expression>> {
        &mut self.step_expr
    }

    pub fn get_statements(&self) -> &Vec<Statement> {
        &self.statements
    }

    pub fn get_statements_mut(&mut self) -> &mut Vec<Statement> {
        &mut self.statements
    }

    pub fn get_next_token(&self) -> &Spanned<Token> {
        &self.next_token
    }

    pub fn get_next_identifier_token(&self) -> Option<&Spanned<Token>> {
        if let Some(ni) = &self.next_identifier_token {
            return Some(ni);
        }
        None
    }

    /// Returns a reference to the get identifier of this [`ForStatement`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn get_next_identifier(&self) -> Option<&unicase::Ascii<String>> {
        if let Some(ni) = &self.next_identifier_token {
            if let Token::Identifier(id) = &ni.token {
                return Some(id);
            }
        }
        None
    }
    pub fn create_empty_statement(
        variable_name: unicase::Ascii<String>,
        start_expr: Expression,
        end_expr: Expression,
        step_expr: Option<Box<Expression>>,
        statements: Vec<Statement>,
    ) -> Statement {
        Statement::For(ForStatement::empty(
            variable_name,
            start_expr,
            end_expr,
            step_expr,
            statements,
        ))
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum CaseSpecifier {
    Expression(Box<Expression>),
    FromTo(Box<Expression>, Box<Expression>),
}
impl CaseSpecifier {
    fn is_similar(&self, i: &CaseSpecifier) -> bool {
        match (self, i) {
            (CaseSpecifier::Expression(e1), CaseSpecifier::Expression(e2)) => e1.is_similar(e2),
            (CaseSpecifier::FromTo(f1, t1), CaseSpecifier::FromTo(f2, t2)) => {
                f1.is_similar(f2) && t1.is_similar(t2)
            }
            _ => false,
        }
    }

    pub fn visit<T: Default, V: AstVisitor<T>>(&self, visitor: &mut V) -> T {
        visitor.visit_case_specifier(self)
    }

    #[must_use]
    pub fn visit_mut<V: AstVisitorMut>(&self, visitor: &mut V) -> Self {
        visitor.visit_case_specifier(self)
    }
}

#[derive(Debug, Clone, PartialEq)]
pub struct CaseBlock {
    case_token: Spanned<Token>,
    case_specifiers: Vec<CaseSpecifier>,
    statements: Vec<Statement>,
}

impl CaseBlock {
    pub fn new(
        case_token: Spanned<Token>,
        case_specifiers: Vec<CaseSpecifier>,
        statements: Vec<Statement>,
    ) -> Self {
        Self {
            case_token,
            case_specifiers,
            statements,
        }
    }

    pub fn empty(case_specifiers: Vec<CaseSpecifier>, statements: Vec<Statement>) -> Self {
        Self {
            case_token: Spanned::create_empty(Token::Case),
            case_specifiers,
            statements,
        }
    }

    pub fn get_case_token(&self) -> &Spanned<Token> {
        &self.case_token
    }

    pub fn get_case_specifiers(&self) -> &Vec<CaseSpecifier> {
        &self.case_specifiers
    }

    pub fn get_case_specifiers_mut(&mut self) -> &mut Vec<CaseSpecifier> {
        &mut self.case_specifiers
    }

    pub fn get_statements(&self) -> &Vec<Statement> {
        &self.statements
    }

    pub fn get_statements_mut(&mut self) -> &mut Vec<Statement> {
        &mut self.statements
    }

    #[must_use]
    pub fn visit_mut<V: AstVisitorMut>(&self, visitor: &mut V) -> Self {
        visitor.visit_case_block(self)
    }

    fn is_similar(&self, other: &CaseBlock) -> bool {
        if self.get_case_specifiers().len() != other.get_case_specifiers().len() {
            return false;
        }
        for i in 0..self.get_case_specifiers().len() {
            if !self.get_case_specifiers()[i].is_similar(&other.get_case_specifiers()[i]) {
                return false;
            }
        }
        if self.get_statements().len() != other.get_statements().len() {
            return false;
        }
        for i in 0..self.get_statements().len() {
            if !self.get_statements()[i].is_similar(&other.get_statements()[i]) {
                return false;
            }
        }
        true
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct SelectStatement {
    select_token: Spanned<Token>,
    case_token: Spanned<Token>,
    expression: Box<Expression>,
    case_blocks: Vec<CaseBlock>,
    default_token: Option<Spanned<Token>>,
    default_statements: Vec<Statement>,
    endselect_token: Spanned<Token>,
}

impl SelectStatement {
    pub fn new(
        select_token: Spanned<Token>,
        case_token: Spanned<Token>,
        expr: Expression,
        case_blocks: Vec<CaseBlock>,
        default_token: Option<Spanned<Token>>,
        default_statements: Vec<Statement>,
        endselect_token: Spanned<Token>,
    ) -> Self {
        Self {
            select_token,
            case_token,
            expression: Box::new(expr),
            case_blocks,
            default_token,
            default_statements,
            endselect_token,
        }
    }

    pub fn empty(
        expr: Expression,
        case_blocks: Vec<CaseBlock>,
        default_statements: Vec<Statement>,
    ) -> Self {
        Self {
            select_token: Spanned::create_empty(Token::Select),
            case_token: Spanned::create_empty(Token::Case),
            expression: Box::new(expr),
            case_blocks,
            default_token: if default_statements.is_empty() {
                None
            } else {
                Some(Spanned::create_empty(Token::Identifier(
                    unicase::Ascii::new("DEFAULT".to_string()),
                )))
            },
            default_statements,
            endselect_token: Spanned::create_empty(Token::EndSelect),
        }
    }

    pub fn get_select_token(&self) -> &Spanned<Token> {
        &self.select_token
    }
    pub fn get_case_token(&self) -> &Spanned<Token> {
        &self.case_token
    }

    pub fn get_lpar_token(&self) -> &Spanned<Token> {
        &self.case_token
    }

    pub fn get_expression(&self) -> &Expression {
        &self.expression
    }

    pub fn get_expression_mut(&mut self) -> &mut Expression {
        &mut self.expression
    }

    pub fn get_case_blocks(&self) -> &Vec<CaseBlock> {
        &self.case_blocks
    }

    pub fn get_case_blocks_mut(&mut self) -> &mut Vec<CaseBlock> {
        &mut self.case_blocks
    }

    pub fn get_default_token(&self) -> &Option<Spanned<Token>> {
        &self.default_token
    }

    pub fn get_default_statements(&self) -> &Vec<Statement> {
        &self.default_statements
    }

    pub fn get_default_statements_mut(&mut self) -> &mut Vec<Statement> {
        &mut self.default_statements
    }

    pub fn get_endselect_token(&self) -> &Spanned<Token> {
        &self.endselect_token
    }

    pub fn create_empty_statement(
        expr: Expression,
        case_blocks: Vec<CaseBlock>,
        default_statements: Vec<Statement>,
    ) -> Statement {
        Statement::Select(SelectStatement::empty(
            expr,
            case_blocks,
            default_statements,
        ))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct GosubStatement {
    gosub_token: Spanned<Token>,
    label_token: Spanned<Token>,
}

impl GosubStatement {
    pub fn new(gosub_token: Spanned<Token>, mut label_token: Spanned<Token>) -> Self {
        if !matches!(label_token.token, Token::Identifier(_)) {
            label_token = Spanned {
                token: Token::Identifier(unicase::Ascii::new(label_token.token.to_string())),
                span: label_token.span,
            };
        }
        Self {
            gosub_token,
            label_token,
        }
    }

    pub fn empty(label: unicase::Ascii<String>) -> Self {
        Self {
            gosub_token: Spanned::create_empty(Token::Gosub),
            label_token: Spanned::create_empty(Token::Identifier(label)),
        }
    }

    pub fn get_gosub_token(&self) -> &Spanned<Token> {
        &self.gosub_token
    }

    pub fn get_label_token(&self) -> &Spanned<Token> {
        &self.label_token
    }

    /// Returns a reference to the get label of this [`GosubStatement`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn get_label(&self) -> &unicase::Ascii<String> {
        if let Token::Identifier(id) = &self.label_token.token {
            return id;
        }
        panic!("Expected identifier token")
    }

    pub fn set_label(&mut self, new_id: unicase::Ascii<String>) {
        if let Token::Identifier(id) = &mut self.label_token.token {
            *id = new_id;
        }
    }

    pub fn create_empty_statement(label: unicase::Ascii<String>) -> Statement {
        Statement::Gosub(GosubStatement::empty(label))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct GotoStatement {
    goto_token: Spanned<Token>,
    label_token: Spanned<Token>,
}

impl GotoStatement {
    pub fn new(goto_token: Spanned<Token>, mut label_token: Spanned<Token>) -> Self {
        if !matches!(label_token.token, Token::Identifier(_)) {
            label_token = Spanned {
                token: Token::Identifier(unicase::Ascii::new(label_token.token.to_string())),
                span: label_token.span,
            };
        }
        Self {
            goto_token,
            label_token,
        }
    }

    pub fn empty(label: unicase::Ascii<String>) -> Self {
        Self {
            goto_token: Spanned::create_empty(Token::Goto),
            label_token: Spanned::create_empty(Token::Identifier(label)),
        }
    }

    pub fn get_goto_token(&self) -> &Spanned<Token> {
        &self.goto_token
    }

    pub fn get_label_token(&self) -> &Spanned<Token> {
        &self.label_token
    }

    /// Returns a reference to the get label of this [`GotoStatement`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn get_label(&self) -> &unicase::Ascii<String> {
        if let Token::Identifier(id) = &self.label_token.token {
            return id;
        }
        panic!("Expected identifier token")
    }

    pub fn set_label(&mut self, new_id: unicase::Ascii<String>) {
        if let Token::Identifier(id) = &mut self.label_token.token {
            *id = new_id;
        }
    }

    pub fn create_empty_statement(label: unicase::Ascii<String>) -> Statement {
        Statement::Goto(GotoStatement::empty(label))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct LabelStatement {
    label_token: Spanned<Token>,
}

impl LabelStatement {
    pub fn new(label_token: Spanned<Token>) -> Self {
        Self { label_token }
    }

    pub fn empty(label: unicase::Ascii<String>) -> Self {
        Self {
            label_token: Spanned::create_empty(Token::Label(label)),
        }
    }

    pub fn get_label_token(&self) -> &Spanned<Token> {
        &self.label_token
    }

    /// Returns a reference to the get identifier of this [`IdentifierExpression`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn get_label(&self) -> &unicase::Ascii<String> {
        if let Token::Label(id) = &self.label_token.token {
            return id;
        }
        panic!("Expected label token")
    }

    pub fn set_label(&mut self, new_id: unicase::Ascii<String>) {
        if let Token::Label(id) = &mut self.label_token.token {
            *id = new_id;
        }
    }

    /// # Panics
    ///
    /// Panics if .
    pub fn get_comment_type(&self) -> CommentType {
        if let Token::Comment(ct, _id) = &self.label_token.token {
            return *ct;
        }
        panic!("Expected identifier token")
    }

    pub fn create_empty_statement(identifier: unicase::Ascii<String>) -> Statement {
        Statement::Label(LabelStatement::empty(identifier))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct ProcedureCallStatement {
    identifier_token: Spanned<Token>,
    leftpar_token: Spanned<Token>,
    arguments: Vec<Expression>,
    rightpar_token: Spanned<Token>,
}

impl ProcedureCallStatement {
    pub fn new(
        identifier_token: Spanned<Token>,
        leftpar_token: Spanned<Token>,
        arguments: Vec<Expression>,
        rightpar_token: Spanned<Token>,
    ) -> Self {
        Self {
            identifier_token,
            leftpar_token,
            arguments,
            rightpar_token,
        }
    }

    pub fn empty(identifier: unicase::Ascii<String>, arguments: Vec<Expression>) -> Self {
        Self {
            identifier_token: Spanned::create_empty(Token::Identifier(identifier)),
            leftpar_token: Spanned::create_empty(Token::LPar),
            arguments,
            rightpar_token: Spanned::create_empty(Token::RPar),
        }
    }

    pub fn get_identifier_token(&self) -> &Spanned<Token> {
        &self.identifier_token
    }
    pub fn get_leftpar_token(&self) -> &Spanned<Token> {
        &self.leftpar_token
    }

    pub fn get_arguments(&self) -> &Vec<Expression> {
        &self.arguments
    }

    pub fn get_arguments_mut(&mut self) -> &mut Vec<Expression> {
        &mut self.arguments
    }

    pub fn get_rightpar_token(&self) -> &Spanned<Token> {
        &self.rightpar_token
    }

    /// Returns a reference to the get identifier of this [`IdentifierExpression`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn get_identifier(&self) -> &unicase::Ascii<String> {
        if let Token::Identifier(id) = &self.identifier_token.token {
            return id;
        }
        panic!("Expected identifier token")
    }

    pub fn set_identifier(&mut self, new_id: unicase::Ascii<String>) {
        if let Token::Identifier(id) = &mut self.identifier_token.token {
            *id = new_id;
        }
    }

    pub fn create_empty_statement(
        identifier: unicase::Ascii<String>,
        arguments: Vec<Expression>,
    ) -> Statement {
        Statement::Call(ProcedureCallStatement::empty(identifier, arguments))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct PredefinedCallStatement {
    identifier_token: Spanned<Token>,
    func: &'static StatementDefinition,
    arguments: Vec<Expression>,
}

impl PredefinedCallStatement {
    pub fn new(
        identifier_token: Spanned<Token>,
        func: &'static StatementDefinition,
        arguments: Vec<Expression>,
    ) -> Self {
        Self {
            identifier_token,
            func,
            arguments,
        }
    }

    pub fn empty(func: &'static StatementDefinition, arguments: Vec<Expression>) -> Self {
        Self {
            identifier_token: Spanned::create_empty(Token::Identifier(unicase::Ascii::new(
                func.name.to_string(),
            ))),
            func,
            arguments,
        }
    }

    pub fn get_identifier_token(&self) -> &Spanned<Token> {
        &self.identifier_token
    }

    /// Returns a reference to the get identifier of this [`IdentifierExpression`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn get_identifier(&self) -> &unicase::Ascii<String> {
        if let Token::Identifier(id) = &self.identifier_token.token {
            return id;
        }
        panic!("Expected identifier token")
    }

    pub fn get_func(&self) -> &'static StatementDefinition {
        self.func
    }

    pub fn get_arguments(&self) -> &Vec<Expression> {
        &self.arguments
    }

    pub fn get_arguments_mut(&mut self) -> &mut Vec<Expression> {
        &mut self.arguments
    }

    pub fn create_empty_statement(
        func: &'static StatementDefinition,
        arguments: Vec<Expression>,
    ) -> Statement {
        Statement::PredifinedCall(PredefinedCallStatement::empty(func, arguments))
    }
}

#[derive(Debug, PartialEq, Clone)]
pub struct LetStatement {
    let_token: Option<Spanned<Token>>,
    identifier_token: Spanned<Token>,
    leftpar_token: Option<Spanned<Token>>,
    arguments: Vec<Expression>,
    rightpar_token: Option<Spanned<Token>>,
    eq_token: Spanned<Token>,
    value_expression: Box<Expression>,
}

impl LetStatement {
    pub fn new(
        let_token: Option<Spanned<Token>>,
        identifier_token: Spanned<Token>,
        leftpar_token: Option<Spanned<Token>>,
        arguments: Vec<Expression>,
        rightpar_token: Option<Spanned<Token>>,
        eq_token: Spanned<Token>,
        value_expression: Expression,
    ) -> Self {
        Self {
            let_token,
            identifier_token,
            leftpar_token,
            arguments,
            rightpar_token,
            eq_token,
            value_expression: Box::new(value_expression),
        }
    }

    pub fn empty(
        identifier: unicase::Ascii<String>,
        arguments: Vec<Expression>,
        value_expression: Expression,
    ) -> Self {
        Self {
            let_token: None,
            identifier_token: Spanned::create_empty(Token::Identifier(identifier)),
            leftpar_token: None,
            arguments,
            rightpar_token: None,
            eq_token: Spanned::create_empty(Token::Eq),
            value_expression: Box::new(value_expression),
        }
    }

    pub fn get_let_token(&self) -> &Option<Spanned<Token>> {
        &self.let_token
    }

    pub fn get_identifier_token(&self) -> &Spanned<Token> {
        &self.identifier_token
    }

    /// Returns a reference to the get identifier of this [`IdentifierExpression`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn get_identifier(&self) -> &unicase::Ascii<String> {
        if let Token::Identifier(id) = &self.identifier_token.token {
            return id;
        }
        panic!("Expected identifier token")
    }

    pub fn set_identifier(&mut self, new_id: unicase::Ascii<String>) {
        if let Token::Identifier(id) = &mut self.identifier_token.token {
            *id = new_id;
        }
    }

    pub fn get_lpar_token(&self) -> &Option<Spanned<Token>> {
        &self.leftpar_token
    }

    pub fn get_arguments(&self) -> &Vec<Expression> {
        &self.arguments
    }

    pub fn get_arguments_mut(&mut self) -> &mut Vec<Expression> {
        &mut self.arguments
    }

    pub fn get_rpar_token_token(&self) -> &Option<Spanned<Token>> {
        &self.rightpar_token
    }

    pub fn get_eq_token(&self) -> &Spanned<Token> {
        &self.eq_token
    }

    pub fn get_value_expression(&self) -> &Expression {
        &self.value_expression
    }

    pub fn get_value_expression_mut(&mut self) -> &mut Expression {
        &mut self.value_expression
    }
    pub fn create_empty_statement(
        identifier: unicase::Ascii<String>,
        arguments: Vec<Expression>,
        value_expression: Expression,
    ) -> Statement {
        Statement::Let(LetStatement::empty(identifier, arguments, value_expression))
    }
}

pub fn get_var_name(expr: &Expression) -> unicase::Ascii<String> {
    if let Expression::FunctionCall(call_expr) = expr {
        call_expr.get_identifier().clone()
    } else {
        unicase::Ascii::new(expr.to_string())
    }
}

impl Statement {
    pub fn param_list_to_string(l: &Vec<Expression>) -> String {
        let mut res = String::new();
        for expr in l {
            if !res.is_empty() {
                res.push_str(", ");
            }
            res.push_str(expr.to_string().as_str());
        }
        res
    }

    pub fn variable_list_to_string(l: &[VariableSpecifier]) -> String {
        let mut res = String::new();
        for expr in l {
            if !res.is_empty() {
                res.push_str(", ");
            }
            res.push_str(expr.get_identifier());
            if !expr.get_dimensions().is_empty() {
                res.push('(');
                for (j, d) in expr.get_dimensions().iter().enumerate() {
                    if j > 0 {
                        res.push_str(", ");
                    }
                    res.push_str(d.get_dimension().to_string().as_str());
                }
                res.push(')');
            }
        }
        res
    }

    pub fn try_boolean_conversion(expr: &Expression) -> Expression {
        match expr {
            Expression::Const(c) => ConstantExpression::create_empty_expression(Constant::Boolean(
                c.get_constant_value().get_value().as_bool(),
            )),
            Expression::Parens(expr) => Statement::try_boolean_conversion(expr.get_expression()),
            Expression::Unary(un_expr) => {
                if !matches!(un_expr.get_op(), UnaryOp::Not) {
                    return expr.clone();
                }

                match un_expr.get_expression() {
                    Expression::Const(c) => Expression::Const(ConstantExpression::empty(
                        Constant::Boolean(!c.get_constant_value().get_value().as_bool()),
                    )),
                    Expression::Unary(notexpr) => {
                        if matches!(notexpr.get_op(), UnaryOp::Not) {
                            return Statement::try_boolean_conversion(notexpr.get_expression());
                        }
                        UnaryExpression::create_empty_expression(
                            un_expr.get_op(),
                            Statement::try_boolean_conversion(un_expr.get_expression()),
                        )
                    }
                    _ => expr.clone(),
                }
            }
            _ => expr.clone(),
        }
    }

    pub fn get_indent(indent: i32) -> String {
        let mut res = String::new();
        for _ in 0..indent {
            res.push_str("    ");
        }
        res
    }
}
