use core::panic;

use crate::{
    ast::{
        AstNode, AstVisitorMut, BinaryExpression, BlockStatement, CommentAstNode, Constant, ConstantExpression, DimensionSpecifier, Expression, ForStatement,
        FunctionImplementation, GotoStatement, IdentifierExpression, IfStatement, LabelStatement, LetStatement, ReturnStatement, SelectStatement, Statement,
        VariableDeclarationStatement, VariableSpecifier,
    },
    decompiler::evaluation_visitor::OptimizationVisitor,
    parser::lexer::{Spanned, Token},
};

pub struct AstTransformationVisitor {
    labels: usize,
    continue_break_labels: Vec<(unicase::Ascii<String>, unicase::Ascii<String>)>,
    cur_function: Option<unicase::Ascii<String>>,
    optimize_output: bool,
}

impl AstTransformationVisitor {
    pub fn new(optimize_output: bool) -> Self {
        Self {
            labels: 0,
            continue_break_labels: Vec::new(),
            cur_function: None,
            optimize_output,
        }
    }
    pub fn next_label(&mut self) -> unicase::Ascii<String> {
        let label = unicase::Ascii::new(format!("*(label{}", self.labels));
        self.labels += 1;
        label
    }
}

impl AstVisitorMut for AstTransformationVisitor {
    fn visit_continue_statement(&mut self, _continue_stmt: &crate::ast::ContinueStatement) -> Statement {
        if self.continue_break_labels.is_empty() {
            return CommentAstNode::create_empty_statement("no continue block");
        }
        let (continue_label, _) = self.continue_break_labels.last().unwrap();
        GotoStatement::create_empty_statement(continue_label.clone())
    }
    fn visit_break_statement(&mut self, _break_stmt: &crate::ast::BreakStatement) -> Statement {
        if self.continue_break_labels.is_empty() {
            return CommentAstNode::create_empty_statement("no break block");
        }
        let (_, break_label) = self.continue_break_labels.last().unwrap();
        GotoStatement::create_empty_statement(break_label.clone())
    }

    fn visit_if_statement(&mut self, if_stmt: &IfStatement) -> Statement {
        if matches!(if_stmt.get_statement(), Statement::Goto(_)) {
            return Statement::If(IfStatement::empty(
                if_stmt.get_condition().visit_mut(self),
                if_stmt.get_statement().visit_mut(self),
            ));
        }
        let mut statements = Vec::new();
        let if_exit_label = self.next_label();
        statements.push(IfStatement::create_empty_statement(
            if_stmt.get_condition().negate_expression(),
            GotoStatement::create_empty_statement(if_exit_label.clone()),
        ));
        statements.push(if_stmt.get_statement().visit_mut(self));
        statements.push(LabelStatement::create_empty_statement(if_exit_label.clone()));
        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_if_then_statement(&mut self, if_then: &crate::ast::IfThenStatement) -> Statement {
        let mut statements = Vec::new();

        let last_exit_label = self.next_label();
        let mut if_exit_label = self.next_label();

        statements.push(IfStatement::create_empty_statement(
            if_then.get_condition().negate_expression(),
            GotoStatement::create_empty_statement(if_exit_label.clone()),
        ));
        statements.extend(if_then.get_statements().iter().map(|s| s.visit_mut(self)));

        if !if_then.get_else_if_blocks().is_empty() || if_then.get_else_block().is_some() {
            statements.push(GotoStatement::create_empty_statement(last_exit_label.clone()));
        }

        for else_if in if_then.get_else_if_blocks() {
            statements.push(LabelStatement::create_empty_statement(if_exit_label.clone()));

            if_exit_label = self.next_label();
            statements.push(IfStatement::create_empty_statement(
                else_if.get_condition().negate_expression(),
                GotoStatement::create_empty_statement(if_exit_label.clone()),
            ));
            statements.extend(else_if.get_statements().iter().map(|s| s.visit_mut(self)));
            statements.push(GotoStatement::create_empty_statement(last_exit_label.clone()));
        }

        if let Some(else_block) = if_then.get_else_block() {
            statements.push(LabelStatement::create_empty_statement(if_exit_label.clone()));
            if_exit_label = self.next_label();

            statements.extend(else_block.get_statements().iter().map(|s| s.visit_mut(self)));
        }

        statements.push(LabelStatement::create_empty_statement(if_exit_label.clone()));
        statements.push(LabelStatement::create_empty_statement(last_exit_label.clone()));

        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_while_statement(&mut self, while_stmt: &crate::ast::WhileStatement) -> Statement {
        let mut statements = Vec::new();
        let break_label = self.next_label();

        statements.push(IfStatement::create_empty_statement(
            while_stmt.get_condition().negate_expression(),
            GotoStatement::create_empty_statement(break_label.clone()),
        ));
        statements.push(while_stmt.get_statement().visit_mut(self));
        statements.push(LabelStatement::create_empty_statement(break_label.clone()));
        self.continue_break_labels.pop();
        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_while_do_statement(&mut self, while_do: &crate::ast::WhileDoStatement) -> Statement {
        let mut statements = Vec::new();

        let continue_label = self.next_label();
        let break_label = self.next_label();

        self.continue_break_labels.push((continue_label.clone(), break_label.clone()));

        statements.push(LabelStatement::create_empty_statement(continue_label.clone()));
        statements.push(IfStatement::create_empty_statement(
            while_do.get_condition().negate_expression(),
            GotoStatement::create_empty_statement(break_label.clone()),
        ));
        statements.extend(while_do.get_statements().iter().map(|s| s.visit_mut(self)));
        statements.push(GotoStatement::create_empty_statement(continue_label.clone()));
        statements.push(LabelStatement::create_empty_statement(break_label.clone()));
        self.continue_break_labels.pop();
        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_repeat_until_statement(&mut self, repeat_until: &crate::ast::RepeatUntilStatement) -> Statement {
        let mut statements = Vec::new();

        let loop_label = self.next_label();
        let continue_label = self.next_label();
        let break_label = self.next_label();

        self.continue_break_labels.push((continue_label.clone(), break_label.clone()));

        statements.push(LabelStatement::create_empty_statement(loop_label.clone()));
        statements.extend(repeat_until.get_statements().iter().map(|s| s.visit_mut(self)));

        statements.push(LabelStatement::create_empty_statement(continue_label.clone()));

        statements.push(IfStatement::create_empty_statement(
            repeat_until.get_condition().negate_expression(),
            GotoStatement::create_empty_statement(loop_label.clone()),
        ));
        statements.push(LabelStatement::create_empty_statement(break_label.clone()));
        self.continue_break_labels.pop();
        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_loop_statement(&mut self, loop_stmt: &crate::ast::LoopStatement) -> Statement {
        let mut statements = Vec::new();

        let continue_label = self.next_label();
        let break_label = self.next_label();

        self.continue_break_labels.push((continue_label.clone(), break_label.clone()));

        statements.push(LabelStatement::create_empty_statement(continue_label.clone()));
        statements.extend(loop_stmt.get_statements().iter().map(|s| s.visit_mut(self)));
        statements.push(GotoStatement::create_empty_statement(continue_label.clone()));
        statements.push(LabelStatement::create_empty_statement(break_label.clone()));
        self.continue_break_labels.pop();
        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_select_statement(&mut self, select_stmt: &SelectStatement) -> Statement {
        let mut statements = Vec::new();

        let expr = select_stmt.get_expression().clone();
        let case_exit_label = self.next_label();

        for case_block in select_stmt.get_case_blocks() {
            let next_case_label = self.next_label();

            for spec in case_block.get_case_specifiers() {
                match spec {
                    crate::ast::CaseSpecifier::Expression(spec_expr) => {
                        statements.push(IfStatement::create_empty_statement(
                            BinaryExpression::create_empty_expression(crate::ast::BinOp::Eq, expr.clone(), *spec_expr.clone()),
                            GotoStatement::create_empty_statement(next_case_label.clone()),
                        ));
                    }
                    crate::ast::CaseSpecifier::FromTo(from_expr, to_expr) => {
                        statements.push(IfStatement::create_empty_statement(
                            BinaryExpression::create_empty_expression(
                                crate::ast::BinOp::And,
                                BinaryExpression::create_empty_expression(crate::ast::BinOp::LowerEq, *from_expr.clone(), expr.clone()),
                                BinaryExpression::create_empty_expression(crate::ast::BinOp::LowerEq, expr.clone(), *to_expr.clone()),
                            ),
                            GotoStatement::create_empty_statement(next_case_label.clone()),
                        ));
                    }
                }
            }

            statements.extend(case_block.get_statements().iter().map(|s| s.visit_mut(self)));
            statements.push(GotoStatement::create_empty_statement(case_exit_label.clone()));
            statements.push(LabelStatement::create_empty_statement(next_case_label.clone()));
        }

        statements.extend(select_stmt.get_default_statements().iter().map(|s| s.visit_mut(self)));

        statements.push(LabelStatement::create_empty_statement(case_exit_label.clone()));

        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_for_statement(&mut self, for_stmt: &ForStatement) -> Statement {
        let mut statements = Vec::new();

        let loop_label = self.next_label();
        let continue_label = self.next_label();
        let break_label = self.next_label();

        let id_expr = Expression::Identifier(IdentifierExpression::new(for_stmt.get_identifier_token().clone()));

        // init variable
        statements.push(LetStatement::create_empty_statement(
            for_stmt.get_identifier().clone(),
            Token::Eq,
            Vec::new(),
            for_stmt.get_start_expr().visit_mut(self),
        ));

        // create loop
        self.continue_break_labels.push((continue_label.clone(), break_label.clone()));
        statements.push(LabelStatement::create_empty_statement(loop_label.clone()));

        let increment = if let Some(increment) = for_stmt.get_step_expr() {
            increment.visit_mut(self)
        } else {
            Expression::Const(ConstantExpression::empty(Constant::Integer(1)))
        };

        let end_expr = for_stmt.get_end_expr().visit_mut(self);

        let lower_bound = BinaryExpression::create_empty_expression(
            crate::ast::BinOp::Or,
            BinaryExpression::create_empty_expression(
                crate::ast::BinOp::Lower,
                ConstantExpression::create_empty_expression(Constant::Integer(0)),
                increment.clone(),
            ),
            BinaryExpression::create_empty_expression(crate::ast::BinOp::Lower, id_expr.clone(), end_expr.clone()),
        );

        let upper_bound = BinaryExpression::create_empty_expression(
            crate::ast::BinOp::Or,
            BinaryExpression::create_empty_expression(
                crate::ast::BinOp::Greater,
                ConstantExpression::create_empty_expression(Constant::Integer(0)),
                increment.clone(),
            ),
            BinaryExpression::create_empty_expression(crate::ast::BinOp::Greater, id_expr.clone(), end_expr.clone()),
        );

        let condition = BinaryExpression::create_empty_expression(crate::ast::BinOp::And, lower_bound, upper_bound);
        statements.push(IfStatement::create_empty_statement(
            if self.optimize_output {
                condition.visit_mut(&mut OptimizationVisitor::default())
            } else {
                condition
            },
            GotoStatement::create_empty_statement(break_label.clone()),
        ));

        statements.extend(for_stmt.get_statements().iter().map(|s| s.visit_mut(self)));

        // create step & increment

        statements.push(LabelStatement::create_empty_statement(continue_label.clone()));
        statements.push(LetStatement::create_empty_statement(
            for_stmt.get_identifier().clone(),
            Token::Eq,
            Vec::new(),
            BinaryExpression::create_empty_expression(crate::ast::BinOp::Add, id_expr, increment),
        ));

        // loop & exit;
        statements.push(GotoStatement::create_empty_statement(loop_label.clone()));
        statements.push(LabelStatement::create_empty_statement(break_label.clone()));
        self.continue_break_labels.pop();
        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_let_statement(&mut self, let_stmt: &LetStatement) -> Statement {
        let mut val_expr = let_stmt.get_value_expression().visit_mut(self);

        match let_stmt.get_let_variant() {
            Token::MulAssign => {
                val_expr = BinaryExpression::create_empty_expression(
                    crate::ast::BinOp::Mul,
                    Expression::Identifier(IdentifierExpression::new(let_stmt.get_identifier_token().clone())),
                    val_expr,
                );
            }
            Token::DivAssign => {
                val_expr = BinaryExpression::create_empty_expression(
                    crate::ast::BinOp::Div,
                    Expression::Identifier(IdentifierExpression::new(let_stmt.get_identifier_token().clone())),
                    val_expr,
                );
            }
            Token::ModAssign => {
                val_expr = BinaryExpression::create_empty_expression(
                    crate::ast::BinOp::Mod,
                    Expression::Identifier(IdentifierExpression::new(let_stmt.get_identifier_token().clone())),
                    val_expr,
                );
            }
            Token::AddAssign => {
                val_expr = BinaryExpression::create_empty_expression(
                    crate::ast::BinOp::Add,
                    Expression::Identifier(IdentifierExpression::new(let_stmt.get_identifier_token().clone())),
                    val_expr,
                );
            }
            Token::SubAssign => {
                val_expr = BinaryExpression::create_empty_expression(
                    crate::ast::BinOp::Sub,
                    Expression::Identifier(IdentifierExpression::new(let_stmt.get_identifier_token().clone())),
                    val_expr,
                );
            }
            Token::AndAssign => {
                val_expr = BinaryExpression::create_empty_expression(
                    crate::ast::BinOp::And,
                    Expression::Identifier(IdentifierExpression::new(let_stmt.get_identifier_token().clone())),
                    val_expr,
                );
            }
            Token::OrAssign => {
                val_expr = BinaryExpression::create_empty_expression(
                    crate::ast::BinOp::Or,
                    Expression::Identifier(IdentifierExpression::new(let_stmt.get_identifier_token().clone())),
                    val_expr,
                );
            }
            _ => {}
        }

        Statement::Let(LetStatement::new(
            let_stmt.get_let_token().clone(),
            Spanned {
                span: let_stmt.get_identifier_token().span.clone(),
                token: Token::Identifier(self.visit_identifier(let_stmt.get_identifier())),
            },
            let_stmt.get_lpar_token().clone(),
            let_stmt.get_arguments().iter().map(|arg| arg.visit_mut(self)).collect(),
            let_stmt.get_rpar_token().clone(),
            Spanned::create_empty(Token::Eq),
            val_expr,
        ))
    }

    fn visit_function_implementation(&mut self, function: &FunctionImplementation) -> AstNode {
        self.cur_function = Some(function.get_identifier().clone());
        let res = AstNode::Function(FunctionImplementation::new(
            function.id,
            function.get_function_token().clone(),
            Spanned {
                span: function.get_identifier_token().span.clone(),
                token: Token::Identifier(self.visit_identifier(function.get_identifier())),
            },
            function.get_leftpar_token().clone(),
            function.get_parameters().iter().map(|arg| arg.visit_mut(self)).collect(),
            function.get_rightpar_token().clone(),
            function.get_return_type_token().clone(),
            function.get_return_type(),
            function.get_statements().iter().map(|stmt| stmt.visit_mut(self)).collect(),
            function.get_endfunc_token().clone(),
        ));

        self.cur_function = None;
        res
    }

    fn visit_return_statement(&mut self, return_stmt: &ReturnStatement) -> Statement {
        let mut statements = Vec::new();
        if let Some(expr) = return_stmt.get_expression() {
            if self.cur_function.is_none() {
                panic!("Return statement outside of function");
            }
            statements.push(Statement::Let(LetStatement::new(
                None,
                Spanned {
                    span: return_stmt.get_return_token().span.clone(),
                    token: Token::Identifier(self.cur_function.clone().unwrap()), // Parser doesn't allow return expression outside of function
                },
                None,
                Vec::new(),
                None,
                Spanned::create_empty(Token::Eq),
                expr.visit_mut(self),
            )));
        }
        statements.push(ReturnStatement::create_empty_statement(None));
        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_variable_declaration_statement(&mut self, var_decl: &VariableDeclarationStatement) -> Statement {
        let mut statements = Vec::new();
        for var in var_decl.get_variables() {
            if let Some(init) = var.get_initalizer() {
                match init {
                    Expression::ArrayInitializer(array) => {
                        let stmt = Statement::VariableDeclaration(VariableDeclarationStatement::new(
                            var_decl.get_type_token().clone(),
                            var_decl.get_variable_type(),
                            vec![VariableSpecifier::new(
                                var.get_identifier_token().clone(),
                                None,
                                vec![DimensionSpecifier::empty(array.get_expressions().len())],
                                None,
                                None,
                                None,
                            )],
                        ));
                        statements.push(stmt);

                        for (idx, expr) in array.get_expressions().iter().enumerate() {
                            statements.push(Statement::Let(LetStatement::new(
                                None,
                                var.get_identifier_token().clone(),
                                None,
                                vec![Expression::Const(ConstantExpression::empty(Constant::Integer(idx as i32)))],
                                None,
                                Spanned::create_empty(Token::Eq),
                                expr.visit_mut(self),
                            )));
                        }
                    }
                    _ => {
                        let stmt = Statement::VariableDeclaration(VariableDeclarationStatement::new(
                            var_decl.get_type_token().clone(),
                            var_decl.get_variable_type(),
                            vec![VariableSpecifier::new(
                                var.get_identifier_token().clone(),
                                None,
                                var.get_dimensions().clone(),
                                None,
                                None,
                                None,
                            )],
                        ));
                        statements.push(stmt);

                        statements.push(Statement::Let(LetStatement::new(
                            None,
                            var.get_identifier_token().clone(),
                            None,
                            Vec::new(),
                            None,
                            Spanned::create_empty(Token::Eq),
                            init.visit_mut(self),
                        )));
                    }
                }
            } else {
                statements.push(Statement::VariableDeclaration(VariableDeclarationStatement::new(
                    var_decl.get_type_token().clone(),
                    var_decl.get_variable_type(),
                    vec![VariableSpecifier::new(
                        var.get_identifier_token().clone(),
                        None,
                        var.get_dimensions().clone(),
                        None,
                        None,
                        None,
                    )],
                )));
            }
        }
        Statement::Block(BlockStatement::empty(statements))
    }
}
