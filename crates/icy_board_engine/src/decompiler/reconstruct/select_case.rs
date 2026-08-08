use crate::ast::{BinOp, CaseBlock, CaseSpecifier, Expression, SelectStatement, Statement};

/// Reads the condition of one branch as the case labels it stands for, together with the
/// expression they all test. A CASE with several labels arrives as a chain of ORed
/// comparisons and a range as a pair of bounds, so both collapse back here.
fn match_case(condition: &Expression) -> Option<(Expression, Vec<CaseSpecifier>)> {
    let Expression::Binary(bin_expr) = Statement::try_boolean_conversion(condition) else {
        return None;
    };

    match bin_expr.get_op() {
        BinOp::Eq => Some((
            bin_expr.get_left_expression().clone(),
            vec![CaseSpecifier::Expression(Box::new(bin_expr.get_right_expression().clone()))],
        )),
        BinOp::Or => {
            let (subject, mut specifiers) = match_case(bin_expr.get_left_expression())?;
            let (other, rest) = match_case(bin_expr.get_right_expression())?;
            if subject != other {
                return None;
            }
            specifiers.extend(rest);
            Some((subject, specifiers))
        }
        BinOp::And => {
            let Expression::Binary(lower) = Statement::try_boolean_conversion(bin_expr.get_left_expression()) else {
                return None;
            };
            let Expression::Binary(upper) = Statement::try_boolean_conversion(bin_expr.get_right_expression()) else {
                return None;
            };
            if lower.get_op() != BinOp::LowerEq || upper.get_op() != BinOp::LowerEq {
                return None;
            }
            if lower.get_right_expression() != upper.get_left_expression() {
                return None;
            }
            Some((
                lower.get_right_expression().clone(),
                vec![CaseSpecifier::FromTo(
                    Box::new(lower.get_left_expression().clone()),
                    Box::new(upper.get_right_expression().clone()),
                )],
            ))
        }
        _ => None,
    }
}

pub fn scan_select_statements(statements: &mut [Statement]) {
    let mut i = 0;
    while i < statements.len() {
        if let Statement::IfThen(if_then_stmt) = statements[i].clone() {
            if if_then_stmt.get_else_block().is_none() {
                i += 1;
                continue;
            }
            let Some((subject, specifiers)) = match_case(if_then_stmt.get_condition()) else {
                i += 1;
                continue;
            };

            let mut case_blocks = Vec::new();
            if !if_then_stmt.get_statements().is_empty() {
                case_blocks.push(CaseBlock::empty(specifiers, if_then_stmt.get_statements().clone()));
            }

            let mut skip = false;
            for if_else_block in if_then_stmt.get_else_if_blocks() {
                let Some((other, specifiers)) = match_case(if_else_block.get_condition()) else {
                    skip = true;
                    break;
                };
                if other != subject {
                    skip = true;
                    break;
                }
                case_blocks.push(CaseBlock::empty(specifiers, if_else_block.get_statements().clone()));
            }

            if skip {
                i += 1;
                continue;
            }

            let default_statements = if let Some(smts) = if_then_stmt.get_else_block() {
                smts.get_statements().clone()
            } else {
                Vec::new()
            };
            if case_blocks.len() > 1 {
                statements[i] = SelectStatement::create_empty_statement(subject, case_blocks, default_statements);
            }
        }
        i += 1;
    }
}
