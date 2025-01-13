use crate::ast::{BinOp, CaseBlock, CaseSpecifier, Expression, SelectStatement, Statement};

pub fn scan_select_statements(statements: &mut [Statement]) {
    let mut i = 0;
    while i < statements.len() {
        if let Statement::IfThen(if_then_stmt) = statements[i].clone() {
            if if_then_stmt.get_else_block().is_none() {
                i += 1;
                continue;
            }
            let Expression::Binary(bin_expr) = Statement::try_boolean_conversion(if_then_stmt.get_condition()) else {
                i += 1;
                continue;
            };
            if bin_expr.get_op() != BinOp::Eq {
                i += 1;
                continue;
            }

            let mut skip = false;
            for if_else_block in if_then_stmt.get_else_if_blocks() {
                let Expression::Binary(bin_expr2) = Statement::try_boolean_conversion(if_else_block.get_condition()) else {
                    skip = true;
                    break;
                };
                if bin_expr2.get_op() != BinOp::Eq {
                    skip = true;
                    break;
                }
                if bin_expr.get_left_expression() != bin_expr2.get_left_expression() {
                    skip = true;
                    break;
                }
            }

            if skip {
                i += 1;
                continue;
            }

            let mut case_blocks = Vec::new();

            if !if_then_stmt.get_statements().is_empty() {
                case_blocks.push(CaseBlock::empty(
                    vec![CaseSpecifier::Expression(Box::new(bin_expr.get_right_expression().clone()))],
                    if_then_stmt.get_statements().clone(),
                ));
            }

            for if_else_block in if_then_stmt.get_else_if_blocks() {
                let Expression::Binary(bin_expr2) = Statement::try_boolean_conversion(if_else_block.get_condition()) else {
                    i += 1;
                    continue;
                };

                if bin_expr2.get_op() != BinOp::Eq {
                    i += 1;
                    continue;
                }
                case_blocks.push(CaseBlock::empty(
                    vec![CaseSpecifier::Expression(Box::new(bin_expr2.get_right_expression().clone()))],
                    if_else_block.get_statements().clone(),
                ));
            }
            let default_statements = if let Some(smts) = if_then_stmt.get_else_block() {
                smts.get_statements().clone()
            } else {
                Vec::new()
            };
            statements[i] = SelectStatement::create_empty_statement(bin_expr.get_left_expression().clone(), case_blocks, default_statements);
        }
        i += 1;
    }
}
