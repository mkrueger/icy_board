use crate::parser::lexer::{Spanned, Token};

use super::{
    Ast, AstNode, BinaryExpression, BlockStatement, BreakStatement, CaseBlock, CaseSpecifier,
    CommentAstNode, ConstantExpression, ContinueStatement, ElseBlock, ElseIfBlock, Expression,
    ForStatement, FunctionCallExpression, FunctionDeclarationAstNode, FunctionImplementation,
    GosubStatement, GotoStatement, IdentifierExpression, IfStatement, IfThenStatement,
    LabelStatement, LetStatement, ParameterSpecifier, ParensExpression, PredefinedCallStatement,
    PredefinedFunctionCallExpression, ProcedureCallStatement, ProcedureDeclarationAstNode,
    ProcedureImplementation, ReturnStatement, SelectStatement, Statement, UnaryExpression,
    VariableDeclarationStatement, VariableSpecifier, WhileDoStatement, WhileStatement,
};

#[allow(unused_variables)]
pub trait AstVisitor<T: Default>: Sized {
    // visit expressions
    fn visit_identifier_expression(&mut self, identifier: &IdentifierExpression) -> T {
        T::default()
    }
    fn visit_constant_expression(&mut self, constant: &ConstantExpression) -> T {
        T::default()
    }
    fn visit_binary_expression(&mut self, binary: &BinaryExpression) -> T {
        walk_binary_expression(self, binary);
        T::default()
    }
    fn visit_unary_expression(&mut self, unary: &UnaryExpression) -> T {
        unary.get_expression().visit(self)
    }
    fn visit_predefined_function_call_expression(
        &mut self,
        call: &PredefinedFunctionCallExpression,
    ) -> T {
        walk_predefined_function_call_expression(self, call);
        T::default()
    }
    fn visit_function_call_expression(&mut self, call: &FunctionCallExpression) -> T {
        walk_function_call_expression(self, call);
        T::default()
    }
    fn visit_parens_expression(&mut self, parens: &ParensExpression) -> T {
        parens.get_expression().visit(self)
    }

    // visit statements
    fn visit_comment(&mut self, comment: &CommentAstNode) -> T {
        T::default()
    }
    fn visit_block_statement(&mut self, block: &BlockStatement) -> T {
        walk_block_stmt(self, block);
        T::default()
    }
    fn visit_if_statement(&mut self, if_stmt: &IfStatement) -> T {
        walk_if_stmt(self, if_stmt);
        T::default()
    }
    fn visit_if_then_statement(&mut self, if_then: &IfThenStatement) -> T {
        walk_if_then_stmt(self, if_then);
        T::default()
    }
    fn visit_select_statement(&mut self, select_stmt: &SelectStatement) -> T {
        walk_select_stmt(self, select_stmt);
        T::default()
    }
    fn visit_case_specifier(&mut self, case_specfier: &CaseSpecifier) -> T {
        match case_specfier {
            CaseSpecifier::Expression(expr) => {
                expr.visit(self);
            }
            CaseSpecifier::FromTo(from, to) => {
                from.visit(self);
                to.visit(self);
            }
        }
        T::default()
    }

    fn visit_while_statement(&mut self, while_stmt: &WhileStatement) -> T {
        walk_while_stmt(self, while_stmt);
        T::default()
    }
    fn visit_while_do_statement(&mut self, while_do: &WhileDoStatement) -> T {
        walk_while_do_stmt(self, while_do);
        T::default()
    }
    fn visit_for_statement(&mut self, for_stmt: &ForStatement) -> T {
        walk_for_stmt(self, for_stmt);
        T::default()
    }
    fn visit_break_statement(&mut self, break_stmt: &BreakStatement) -> T {
        T::default()
    }
    fn visit_continue_statement(&mut self, continue_stmt: &ContinueStatement) -> T {
        T::default()
    }
    fn visit_gosub_statement(&mut self, gosub: &GosubStatement) -> T {
        T::default()
    }
    fn visit_return_statement(&mut self, return_stmt: &ReturnStatement) -> T {
        T::default()
    }
    fn visit_let_statement(&mut self, let_stmt: &LetStatement) -> T {
        walk_let_stmt(self, let_stmt);
        T::default()
    }
    fn visit_goto_statement(&mut self, goto: &GotoStatement) -> T {
        T::default()
    }
    fn visit_label_statement(&mut self, label: &LabelStatement) -> T {
        T::default()
    }
    fn visit_procedure_call_statement(&mut self, call: &ProcedureCallStatement) -> T {
        walk_procedure_call_statement(self, call);
        T::default()
    }
    fn visit_predefined_call_statement(&mut self, call: &PredefinedCallStatement) -> T {
        walk_predefined_call_statement(self, call);
        T::default()
    }

    // visit declarations
    fn visit_variable_declaration_statement(
        &mut self,
        var_decl: &VariableDeclarationStatement,
    ) -> T {
        T::default()
    }
    fn visit_procedure_declaration(&mut self, proc_decl: &ProcedureDeclarationAstNode) -> T {
        walk_procedure_declaration(self, proc_decl);
        T::default()
    }
    fn visit_function_declaration(&mut self, func_decl: &FunctionDeclarationAstNode) -> T {
        walk_function_declaration(self, func_decl);
        T::default()
    }

    // visit implementations

    fn visit_parameter_specifier(&mut self, param: &ParameterSpecifier) -> T {
        T::default()
    }

    fn visit_function_implementation(&mut self, function: &FunctionImplementation) -> T {
        walk_function_implementation(self, function);
        T::default()
    }

    fn visit_procedure_implementation(&mut self, procedure: &ProcedureImplementation) -> T {
        walk_procedure_implementation(self, procedure);
        T::default()
    }

    fn visit_ast(&mut self, program: &Ast) -> T {
        walk_program(self, program);
        T::default()
    }

    fn visit_main(&mut self, main: &BlockStatement) -> T {
        walk_block_stmt(self, main);
        T::default()
    }
}

pub fn walk_procedure_declaration<T: Default, V: AstVisitor<T>>(
    visitor: &mut V,
    proc_decl: &ProcedureDeclarationAstNode,
) {
    for p in proc_decl.get_parameters() {
        p.visit(visitor);
    }
}

pub fn walk_function_declaration<T: Default, V: AstVisitor<T>>(
    visitor: &mut V,
    func_decl: &FunctionDeclarationAstNode,
) {
    for p in func_decl.get_parameters() {
        p.visit(visitor);
    }
}

pub fn walk_program<T: Default, V: AstVisitor<T>>(visitor: &mut V, program: &Ast) {
    for node in &program.nodes {
        node.visit(visitor);
    }
}

pub fn walk_select_stmt<T: Default, V: AstVisitor<T>>(
    visitor: &mut V,
    select_stmt: &SelectStatement,
) {
    select_stmt.get_expression().visit(visitor);

    for case_block in select_stmt.get_case_blocks() {
        for specifier in case_block.get_case_specifiers() {
            specifier.visit(visitor);
        }
        for stmt in case_block.get_statements() {
            stmt.visit(visitor);
        }
    }

    for stmt in select_stmt.get_default_statements() {
        stmt.visit(visitor);
    }
}

pub fn walk_while_stmt<T: Default, V: AstVisitor<T>>(
    visitor: &mut V,
    while_do_stmt: &WhileStatement,
) {
    while_do_stmt.get_condition().visit(visitor);
    while_do_stmt.get_statement().visit(visitor);
}

pub fn walk_while_do_stmt<T: Default, V: AstVisitor<T>>(
    visitor: &mut V,
    while_do_stmt: &WhileDoStatement,
) {
    while_do_stmt.get_condition().visit(visitor);
    for stmt in while_do_stmt.get_statements() {
        stmt.visit(visitor);
    }
}

pub fn walk_for_stmt<T: Default, V: AstVisitor<T>>(visitor: &mut V, for_stmt: &ForStatement) {
    for_stmt.get_start_expr().visit(visitor);
    for_stmt.get_end_expr().visit(visitor);
    if let Some(step) = for_stmt.get_step_expr() {
        step.visit(visitor);
    }
    for stmt in for_stmt.get_statements() {
        stmt.visit(visitor);
    }
}

pub fn walk_function_implementation<T: Default, V: AstVisitor<T>>(
    visitor: &mut V,
    function: &FunctionImplementation,
) {
    for p in &function.parameters {
        p.visit(visitor);
    }
    for stmt in function.get_statements() {
        stmt.visit(visitor);
    }
}

pub fn walk_procedure_implementation<T: Default, V: AstVisitor<T>>(
    visitor: &mut V,
    procedure: &ProcedureImplementation,
) {
    for p in &procedure.parameters {
        p.visit(visitor);
    }
    for stmt in procedure.get_statements() {
        stmt.visit(visitor);
    }
}

pub fn walk_let_stmt<T: Default, V: AstVisitor<T>>(visitor: &mut V, let_stmt: &LetStatement) {
    for arg in let_stmt.get_arguments() {
        arg.visit(visitor);
    }
    let_stmt.get_value_expression().visit(visitor);
}

pub fn walk_binary_expression<T: Default, V: AstVisitor<T>>(
    visitor: &mut V,
    binary: &BinaryExpression,
) {
    binary.get_left_expression().visit(visitor);
    binary.get_right_expression().visit(visitor);
}

pub fn walk_predefined_function_call_expression<T: Default, V: AstVisitor<T>>(
    visitor: &mut V,
    call: &PredefinedFunctionCallExpression,
) {
    for arg in call.get_arguments() {
        arg.visit(visitor);
    }
}

pub fn walk_function_call_expression<T: Default, V: AstVisitor<T>>(
    visitor: &mut V,
    call: &FunctionCallExpression,
) {
    for arg in call.get_arguments() {
        arg.visit(visitor);
    }
}

pub fn walk_predefined_call_statement<T: Default, V: AstVisitor<T>>(
    visitor: &mut V,
    call: &PredefinedCallStatement,
) {
    for arg in call.get_arguments() {
        arg.visit(visitor);
    }
}

pub fn walk_procedure_call_statement<T: Default, V: AstVisitor<T>>(
    visitor: &mut V,
    call: &ProcedureCallStatement,
) {
    for arg in call.get_arguments() {
        arg.visit(visitor);
    }
}

pub fn walk_if_then_stmt<T: Default, V: AstVisitor<T>>(visitor: &mut V, if_then: &IfThenStatement) {
    if_then.get_condition().visit(visitor);
    for stmt in if_then.get_statements() {
        stmt.visit(visitor);
    }
    for stmt in if_then.get_else_if_blocks() {
        stmt.get_condition().visit(visitor);
        for s in stmt.get_statements() {
            s.visit(visitor);
        }
    }
    if let Some(else_block) = if_then.get_else_block() {
        for stmt in else_block.get_statements() {
            stmt.visit(visitor);
        }
    }
}

pub fn walk_if_stmt<T: Default, V: AstVisitor<T>>(visitor: &mut V, if_stmt: &IfStatement) {
    if_stmt.get_condition().visit(visitor);
    if_stmt.get_statement().visit(visitor);
}

pub fn walk_block_stmt<T: Default, V: AstVisitor<T>>(visitor: &mut V, block: &BlockStatement) {
    for stmt in block.get_statements() {
        stmt.visit(visitor);
    }
}

#[allow(unused_variables)]
pub trait AstVisitorMut: Sized {
    fn visit_identifier(&mut self, id: &unicase::Ascii<String>) -> unicase::Ascii<String> {
        id.clone()
    }

    fn visit_condition(&mut self, condition: &Expression) -> Expression {
        condition.visit_mut(self)
    }

    // visit expressions
    fn visit_identifier_expression(&mut self, identifier: &IdentifierExpression) -> Expression {
        Expression::Identifier(IdentifierExpression::new(Spanned {
            span: identifier.get_identifier_token().span.clone(),
            token: Token::Identifier(self.visit_identifier(identifier.get_identifier())),
        }))
    }

    fn visit_constant_expression(&mut self, constant: &ConstantExpression) -> Expression {
        Expression::Const(constant.clone())
    }

    fn visit_binary_expression(&mut self, binary: &BinaryExpression) -> Expression {
        let left = binary.get_left_expression().visit_mut(self);
        let right = binary.get_right_expression().visit_mut(self);
        Expression::Binary(BinaryExpression::empty(left, binary.get_op(), right))
    }

    fn visit_unary_expression(&mut self, unary: &UnaryExpression) -> Expression {
        let expr = unary.get_expression().visit_mut(self);
        Expression::Unary(UnaryExpression::empty(unary.get_op(), expr))
    }

    fn visit_predefined_function_call_expression(
        &mut self,
        call: &PredefinedFunctionCallExpression,
    ) -> Expression {
        Expression::PredefinedFunctionCall(PredefinedFunctionCallExpression::empty(
            call.get_func(),
            call.get_arguments()
                .iter()
                .map(|arg| arg.visit_mut(self))
                .collect(),
        ))
    }

    fn visit_function_call_expression(&mut self, call: &FunctionCallExpression) -> Expression {
        Expression::FunctionCall(FunctionCallExpression::new(
            Spanned {
                span: call.get_identifier_token().span.clone(),
                token: Token::Identifier(self.visit_identifier(call.get_identifier())),
            },
            call.get_lpar_token_token().clone(),
            call.get_arguments()
                .iter()
                .map(|arg| arg.visit_mut(self))
                .collect(),
            call.get_rpar_token_token().clone(),
        ))
    }

    fn visit_parens_expression(&mut self, parens: &ParensExpression) -> Expression {
        Expression::Parens(ParensExpression::empty(
            parens.get_expression().visit_mut(self),
        ))
    }

    // visit statements

    fn visit_comment_statement(&mut self, comment: &CommentAstNode) -> Statement {
        Statement::Comment(comment.clone())
    }

    fn visit_block(&mut self, block: &BlockStatement) -> BlockStatement {
        BlockStatement::empty(
            block
                .get_statements()
                .iter()
                .map(|stmt| stmt.visit_mut(self))
                .collect(),
        )
    }
    fn visit_if_statement(&mut self, if_stmt: &IfStatement) -> Statement {
        Statement::If(IfStatement::empty(
            self.visit_condition(if_stmt.get_condition()),
            if_stmt.get_statement().visit_mut(self),
        ))
    }

    fn visit_if_then_statement(&mut self, if_then: &IfThenStatement) -> Statement {
        Statement::IfThen(IfThenStatement::empty(
            self.visit_condition(if_then.get_condition()),
            if_then
                .get_statements()
                .iter()
                .map(|stmt| stmt.visit_mut(self))
                .collect(),
            if_then
                .get_else_if_blocks()
                .iter()
                .map(|stmt| stmt.visit_mut(self))
                .collect(),
            if_then
                .get_else_block()
                .as_ref()
                .map(|else_block| else_block.visit_mut(self)),
        ))
    }

    fn visit_else_if_block(&mut self, else_if: &ElseIfBlock) -> ElseIfBlock {
        ElseIfBlock::empty(
            self.visit_condition(else_if.get_condition()),
            else_if
                .get_statements()
                .iter()
                .map(|stmt| stmt.visit_mut(self))
                .collect(),
        )
    }

    fn visit_else_block(&mut self, else_block: &ElseBlock) -> ElseBlock {
        ElseBlock::empty(
            else_block
                .get_statements()
                .iter()
                .map(|stmt| stmt.visit_mut(self))
                .collect(),
        )
    }

    fn visit_select_statement(&mut self, select_stmt: &SelectStatement) -> Statement {
        Statement::Select(SelectStatement::empty(
            select_stmt.get_expression().visit_mut(self),
            select_stmt
                .get_case_blocks()
                .iter()
                .map(|block| block.visit_mut(self))
                .collect(),
            select_stmt
                .get_default_statements()
                .iter()
                .map(|stmt| stmt.visit_mut(self))
                .collect(),
        ))
    }

    fn visit_case_block(&mut self, case_block: &CaseBlock) -> CaseBlock {
        CaseBlock::empty(
            case_block
                .get_case_specifiers()
                .iter()
                .map(|specifier| specifier.visit_mut(self))
                .collect(),
            case_block
                .get_statements()
                .iter()
                .map(|stmt| stmt.visit_mut(self))
                .collect(),
        )
    }

    fn visit_case_specifier(&mut self, case_specfier: &CaseSpecifier) -> CaseSpecifier {
        match case_specfier {
            CaseSpecifier::Expression(expr) => {
                CaseSpecifier::Expression(Box::new(expr.visit_mut(self)))
            }
            CaseSpecifier::FromTo(from, to) => {
                CaseSpecifier::FromTo(Box::new(from.visit_mut(self)), Box::new(to.visit_mut(self)))
            }
        }
    }

    fn visit_while_statement(&mut self, while_stmt: &WhileStatement) -> Statement {
        Statement::While(WhileStatement::empty(
            self.visit_condition(while_stmt.get_condition()),
            while_stmt.get_statement().visit_mut(self),
        ))
    }

    fn visit_while_do_statement(&mut self, while_do: &WhileDoStatement) -> Statement {
        Statement::WhileDo(WhileDoStatement::empty(
            self.visit_condition(while_do.get_condition()),
            while_do
                .get_statements()
                .iter()
                .map(|stmt| stmt.visit_mut(self))
                .collect(),
        ))
    }

    fn visit_for_statement(&mut self, for_stmt: &ForStatement) -> Statement {
        Statement::For(ForStatement::new(
            for_stmt.get_for_token().clone(),
            Spanned {
                span: for_stmt.get_identifier_token().span.clone(),
                token: Token::Identifier(self.visit_identifier(for_stmt.get_identifier())),
            },
            for_stmt.get_eq_token().clone(),
            for_stmt.get_start_expr().visit_mut(self),
            for_stmt.get_to_token().clone(),
            for_stmt.get_end_expr().visit_mut(self),
            for_stmt.get_step_token().clone(),
            for_stmt
                .get_step_expr()
                .as_ref()
                .map(|step_expr| Box::new(step_expr.visit_mut(self))),
            for_stmt
                .get_statements()
                .iter()
                .map(|stmt| stmt.visit_mut(self))
                .collect(),
            for_stmt.get_next_token().clone(),
            for_stmt.get_next_identifier_token().map(|ni| Spanned {
                span: ni.span.clone(),
                token: Token::Identifier(
                    self.visit_identifier(for_stmt.get_next_identifier().unwrap()),
                ),
            }),
        ))
    }

    fn visit_break_statement(&mut self, break_stmt: &BreakStatement) -> Statement {
        Statement::Break(break_stmt.clone())
    }
    fn visit_continue_statement(&mut self, continue_stmt: &ContinueStatement) -> Statement {
        Statement::Continue(continue_stmt.clone())
    }
    fn visit_gosub_statement(&mut self, gosub: &GosubStatement) -> Statement {
        Statement::Gosub(gosub.clone())
    }
    fn visit_return_statement(&mut self, return_stmt: &ReturnStatement) -> Statement {
        Statement::Return(return_stmt.clone())
    }

    fn visit_let_statement(&mut self, let_stmt: &LetStatement) -> Statement {
        Statement::Let(LetStatement::new(
            let_stmt.get_let_token().clone(),
            Spanned {
                span: let_stmt.get_identifier_token().span.clone(),
                token: Token::Identifier(self.visit_identifier(let_stmt.get_identifier())),
            },
            let_stmt.get_lpar_token().clone(),
            let_stmt
                .get_arguments()
                .iter()
                .map(|arg| arg.visit_mut(self))
                .collect(),
            let_stmt.get_rpar_token_token().clone(),
            let_stmt.get_eq_token().clone(),
            let_stmt.get_value_expression().visit_mut(self),
        ))
    }

    fn visit_goto_statement(&mut self, goto: &GotoStatement) -> Statement {
        Statement::Goto(goto.clone())
    }
    fn visit_label_statement(&mut self, label: &LabelStatement) -> Statement {
        Statement::Label(label.clone())
    }
    fn visit_procedure_call_statement(&mut self, call: &ProcedureCallStatement) -> Statement {
        Statement::Call(ProcedureCallStatement::new(
            Spanned {
                span: call.get_identifier_token().span.clone(),
                token: Token::Identifier(self.visit_identifier(call.get_identifier())),
            },
            call.get_leftpar_token().clone(),
            call.get_arguments()
                .iter()
                .map(|arg| arg.visit_mut(self))
                .collect(),
            call.get_rightpar_token().clone(),
        ))
    }
    fn visit_predefined_call_statement(&mut self, call: &PredefinedCallStatement) -> Statement {
        Statement::PredifinedCall(PredefinedCallStatement::empty(
            call.get_func(),
            call.get_arguments()
                .iter()
                .map(|arg| arg.visit_mut(self))
                .collect(),
        ))
    }

    // visit declarations
    fn visit_variable_declaration_statement(
        &mut self,
        var_decl: &VariableDeclarationStatement,
    ) -> Statement {
        Statement::VariableDeclaration(VariableDeclarationStatement::new(
            var_decl.get_type_token().clone(),
            var_decl.get_variable_type(),
            var_decl
                .get_variables()
                .iter()
                .map(|var| var.visit_mut(self))
                .collect(),
        ))
    }

    fn visit_variable_specifier(&mut self, var: &VariableSpecifier) -> VariableSpecifier {
        VariableSpecifier::new(
            Spanned {
                span: var.get_identifier_token().span.clone(),
                token: Token::Identifier(self.visit_identifier(var.get_identifier())),
            },
            var.get_leftpar_token().clone(),
            var.get_dimensions().clone(),
            var.get_rightpar_token().clone(),
        )
    }

    fn visit_procedure_declaration(&mut self, proc_decl: &ProcedureDeclarationAstNode) -> AstNode {
        AstNode::ProcedureDeclaration(ProcedureDeclarationAstNode::new(
            proc_decl.get_declare_token().clone(),
            proc_decl.get_procedure_token().clone(),
            Spanned {
                span: proc_decl.get_identifier_token().span.clone(),
                token: Token::Identifier(self.visit_identifier(proc_decl.get_identifier())),
            },
            proc_decl.get_leftpar_token().clone(),
            proc_decl
                .get_parameters()
                .iter()
                .map(|param| param.visit_mut(self))
                .collect(),
            proc_decl.get_rightpar_token().clone(),
        ))
    }
    fn visit_function_declaration(&mut self, func_decl: &FunctionDeclarationAstNode) -> AstNode {
        AstNode::FunctionDeclaration(FunctionDeclarationAstNode::new(
            func_decl.get_declare_token().clone(),
            func_decl.get_function_token().clone(),
            Spanned {
                span: func_decl.get_identifier_token().span.clone(),
                token: Token::Identifier(self.visit_identifier(func_decl.get_identifier())),
            },
            func_decl.get_leftpar_token().clone(),
            func_decl
                .get_parameters()
                .iter()
                .map(|param| param.visit_mut(self))
                .collect(),
            func_decl.get_rightpar_token().clone(),
            func_decl.get_return_type_token().clone(),
            func_decl.get_return_type(),
        ))
    }

    // visit implementations

    fn visit_comment_implementation(&mut self, comment: &Spanned<Token>) -> Spanned<Token> {
        comment.clone()
    }

    fn visit_function_implementation(&mut self, function: &FunctionImplementation) -> AstNode {
        AstNode::Function(FunctionImplementation::new(
            function.id,
            function.get_function_token().clone(),
            Spanned {
                span: function.get_identifier_token().span.clone(),
                token: Token::Identifier(self.visit_identifier(function.get_identifier())),
            },
            function.get_leftpar_token().clone(),
            function
                .get_parameters()
                .iter()
                .map(|arg| arg.visit_mut(self))
                .collect(),
            function.rightpar_token.clone(),
            function.get_return_type_token().clone(),
            function.get_return_type(),
            function
                .get_statements()
                .iter()
                .map(|stmt| stmt.visit_mut(self))
                .collect(),
            function.get_endfunc_token().clone(),
        ))
    }

    fn visit_parameter_specifier(&mut self, param: &ParameterSpecifier) -> ParameterSpecifier {
        param.clone()
    }

    fn visit_procedure_implementation(&mut self, procedure: &ProcedureImplementation) -> AstNode {
        AstNode::Procedure(ProcedureImplementation::new(
            procedure.id,
            procedure.get_procedure_token().clone(),
            Spanned {
                span: procedure.get_identifier_token().span.clone(),
                token: Token::Identifier(self.visit_identifier(procedure.get_identifier())),
            },
            procedure.get_leftpar_token().clone(),
            procedure
                .get_parameters()
                .iter()
                .map(|arg| arg.visit_mut(self))
                .collect(),
            procedure.get_rightpar_token().clone(),
            procedure
                .get_statements()
                .iter()
                .map(|stmt| stmt.visit_mut(self))
                .collect(),
            procedure.get_endproc_token().clone(),
        ))
    }

    fn visit_ast(&mut self, program: &Ast) -> Ast {
        let mut new_program = Ast::new();
        new_program.file_name.clone_from(&program.file_name);
        for node in &program.nodes {
            new_program.nodes.push(node.visit_mut(self));
        }
        new_program
    }
}
