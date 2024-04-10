use crate::{
    ast::{
        AstVisitorMut, BinaryExpression, BlockStatement, CommentAstNode, Constant,
        ConstantExpression, Expression, ForStatement, GotoStatement, IdentifierExpression,
        IfStatement, LabelStatement, LetStatement, SelectStatement, Statement,
    },
    decompiler::evaluation_visitor::OptimizationVisitor,
};

#[derive(Default)]
pub struct AstTransformationVisitor {
    labels: usize,
    continue_break_labels: Vec<(unicase::Ascii<String>, unicase::Ascii<String>)>,
}

impl AstTransformationVisitor {
    pub fn next_label(&mut self) -> unicase::Ascii<String> {
        let label = unicase::Ascii::new(format!("*(label{}", self.labels));
        self.labels += 1;
        label
    }
}

impl AstVisitorMut for AstTransformationVisitor {
    fn visit_continue_statement(
        &mut self,
        _continue_stmt: &crate::ast::ContinueStatement,
    ) -> Statement {
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
        statements.push(LabelStatement::create_empty_statement(
            if_exit_label.clone(),
        ));
        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_if_then_statement(&mut self, if_then: &crate::ast::IfThenStatement) -> Statement {
        let mut statements = Vec::new();

        let last_exit_label = self.next_label();
        let mut if_exit_label = self.next_label();

        statements.push(IfStatement::create_empty_statement(
            if_then.get_condition().negate_expression().clone(),
            GotoStatement::create_empty_statement(if_exit_label.clone()),
        ));
        statements.extend(if_then.get_statements().iter().map(|s| s.visit_mut(self)));

        if !if_then.get_else_if_blocks().is_empty() || if_then.get_else_block().is_some() {
            statements.push(GotoStatement::create_empty_statement(
                last_exit_label.clone(),
            ));
        }

        for else_if in if_then.get_else_if_blocks() {
            statements.push(LabelStatement::create_empty_statement(
                if_exit_label.clone(),
            ));

            if_exit_label = self.next_label();
            statements.push(IfStatement::create_empty_statement(
                else_if.get_condition().negate_expression(),
                GotoStatement::create_empty_statement(if_exit_label.clone()),
            ));
            statements.extend(else_if.get_statements().iter().map(|s| s.visit_mut(self)));
            statements.push(GotoStatement::create_empty_statement(
                last_exit_label.clone(),
            ));
        }

        if let Some(else_block) = if_then.get_else_block() {
            statements.push(LabelStatement::create_empty_statement(
                if_exit_label.clone(),
            ));
            if_exit_label = self.next_label();

            statements.extend(
                else_block
                    .get_statements()
                    .iter()
                    .map(|s| s.visit_mut(self)),
            );
        }

        statements.push(LabelStatement::create_empty_statement(
            if_exit_label.clone(),
        ));
        statements.push(LabelStatement::create_empty_statement(
            last_exit_label.clone(),
        ));

        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_while_do_statement(&mut self, while_do: &crate::ast::WhileDoStatement) -> Statement {
        let mut statements = Vec::new();

        let continue_label = self.next_label();
        let break_label = self.next_label();

        self.continue_break_labels
            .push((continue_label.clone(), break_label.clone()));

        statements.push(LabelStatement::create_empty_statement(
            continue_label.clone(),
        ));
        statements.push(IfStatement::create_empty_statement(
            while_do.get_condition().negate_expression(),
            GotoStatement::create_empty_statement(break_label.clone()),
        ));
        statements.extend(while_do.get_statements().iter().map(|s| s.visit_mut(self)));
        statements.push(GotoStatement::create_empty_statement(
            continue_label.clone(),
        ));
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
                            BinaryExpression::create_empty_expression(
                                crate::ast::BinOp::Eq,
                                expr.clone(),
                                *spec_expr.clone(),
                            ),
                            GotoStatement::create_empty_statement(next_case_label.clone()),
                        ));
                    }
                    crate::ast::CaseSpecifier::FromTo(from_expr, to_expr) => {
                        statements.push(IfStatement::create_empty_statement(
                            BinaryExpression::create_empty_expression(
                                crate::ast::BinOp::And,
                                BinaryExpression::create_empty_expression(
                                    crate::ast::BinOp::LowerEq,
                                    *from_expr.clone(),
                                    expr.clone(),
                                ),
                                BinaryExpression::create_empty_expression(
                                    crate::ast::BinOp::LowerEq,
                                    expr.clone(),
                                    *to_expr.clone(),
                                ),
                            ),
                            GotoStatement::create_empty_statement(next_case_label.clone()),
                        ));
                    }
                }
            }

            statements.extend(
                case_block
                    .get_statements()
                    .iter()
                    .map(|s| s.visit_mut(self)),
            );
            statements.push(GotoStatement::create_empty_statement(
                case_exit_label.clone(),
            ));
            statements.push(LabelStatement::create_empty_statement(
                next_case_label.clone(),
            ));
        }

        statements.extend(
            select_stmt
                .get_default_statements()
                .iter()
                .map(|s| s.visit_mut(self)),
        );

        statements.push(LabelStatement::create_empty_statement(
            case_exit_label.clone(),
        ));

        Statement::Block(BlockStatement::empty(statements))
    }

    fn visit_for_statement(&mut self, for_stmt: &ForStatement) -> Statement {
        let mut statements = Vec::new();

        let continue_label = self.next_label();
        let break_label = self.next_label();

        let id_expr =
            IdentifierExpression::create_empty_expression(for_stmt.get_identifier().clone());

        // init variable
        statements.push(LetStatement::create_empty_statement(
            for_stmt.get_identifier().clone(),
            Vec::new(),
            for_stmt.get_start_expr().visit_mut(self),
        ));

        // create loop
        self.continue_break_labels
            .push((continue_label.clone(), break_label.clone()));
        statements.push(LabelStatement::create_empty_statement(
            continue_label.clone(),
        ));

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
            BinaryExpression::create_empty_expression(
                crate::ast::BinOp::Lower,
                id_expr.clone(),
                end_expr.clone(),
            ),
        );

        let upper_bound = BinaryExpression::create_empty_expression(
            crate::ast::BinOp::Or,
            BinaryExpression::create_empty_expression(
                crate::ast::BinOp::Greater,
                ConstantExpression::create_empty_expression(Constant::Integer(0)),
                increment.clone(),
            ),
            BinaryExpression::create_empty_expression(
                crate::ast::BinOp::Greater,
                id_expr.clone(),
                end_expr.clone(),
            ),
        );

        let condition = BinaryExpression::create_empty_expression(
            crate::ast::BinOp::And,
            lower_bound,
            upper_bound,
        );

        statements.push(IfStatement::create_empty_statement(
            condition.visit_mut(&mut OptimizationVisitor::default()),
            GotoStatement::create_empty_statement(break_label.clone()),
        ));

        statements.extend(for_stmt.get_statements().iter().map(|s| s.visit_mut(self)));

        // create step & increment

        statements.push(LetStatement::create_empty_statement(
            for_stmt.get_identifier().clone(),
            Vec::new(),
            BinaryExpression::create_empty_expression(crate::ast::BinOp::Add, id_expr, increment),
        ));

        // loop & exit;
        statements.push(GotoStatement::create_empty_statement(
            continue_label.clone(),
        ));
        statements.push(LabelStatement::create_empty_statement(break_label.clone()));
        self.continue_break_labels.pop();
        Statement::Block(BlockStatement::empty(statements))
    }
}
