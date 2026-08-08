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
            // An empty branch still has to keep its labels, or they fall through to the default.
            case_blocks.push(CaseBlock::empty(specifiers, if_then_stmt.get_statements().clone()));

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

#[cfg(test)]
mod tests {
    use super::{BinOp, CaseSpecifier, Expression, Statement, scan_select_statements};
    use crate::ast::{
        BinaryExpression, BreakStatement, Constant, ConstantExpression, ElseBlock, ElseIfBlock, IdentifierExpression, IfThenStatement, constant::NumberFormat,
    };

    fn var(name: &str) -> Expression {
        IdentifierExpression::create_empty_expression(unicase::Ascii::new(name.to_string()))
    }

    fn int(value: i32) -> Expression {
        ConstantExpression::create_empty_expression(Constant::Integer(value, NumberFormat::Default))
    }

    fn bin(op: BinOp, left: Expression, right: Expression) -> Expression {
        BinaryExpression::create_empty_expression(op, left, right)
    }

    /// The decompiler hands the reconstruction an IF chain whose nodes carry no spans,
    /// which the parser cannot produce, so the chain is built here instead.
    fn reconstruct(condition: Expression, then_statements: Vec<Statement>, else_if_condition: Expression) -> Statement {
        let mut statements = vec![IfThenStatement::create_empty_statement(
            condition,
            then_statements,
            vec![ElseIfBlock::empty(else_if_condition, vec![BreakStatement::create_empty_statement()])],
            Some(ElseBlock::empty(vec![BreakStatement::create_empty_statement()])),
        )];
        scan_select_statements(&mut statements);
        statements.remove(0)
    }

    fn labels(statement: &Statement) -> Vec<String> {
        let Statement::Select(select_stmt) = statement else {
            panic!("no select statement was reconstructed");
        };
        select_stmt
            .get_case_blocks()
            .iter()
            .map(|block| {
                block
                    .get_case_specifiers()
                    .iter()
                    .map(|spec| match spec {
                        CaseSpecifier::Expression(expr) => expr.to_string(),
                        CaseSpecifier::FromTo(from, to) => format!("{from}..{to}"),
                    })
                    .collect::<Vec<_>>()
                    .join(", ")
            })
            .collect()
    }

    #[test]
    fn test_a_branch_that_ors_its_comparisons_becomes_one_case() {
        let condition = bin(
            BinOp::Or,
            bin(BinOp::Or, bin(BinOp::Eq, var("I"), int(1)), bin(BinOp::Eq, var("I"), int(2))),
            bin(BinOp::Eq, var("I"), int(3)),
        );
        let stmt = reconstruct(condition, vec![BreakStatement::create_empty_statement()], bin(BinOp::Eq, var("I"), int(4)));
        assert_eq!(labels(&stmt), vec!["1, 2, 3", "4"]);
    }

    #[test]
    fn test_a_branch_that_tests_two_bounds_becomes_a_range() {
        let condition = bin(BinOp::And, bin(BinOp::LowerEq, int(4), var("I")), bin(BinOp::LowerEq, var("I"), int(6)));
        let stmt = reconstruct(condition, vec![BreakStatement::create_empty_statement()], bin(BinOp::Eq, var("I"), int(9)));
        assert_eq!(labels(&stmt), vec!["4..6", "9"]);
    }

    #[test]
    fn test_a_case_without_statements_keeps_its_labels() {
        let stmt = reconstruct(bin(BinOp::Eq, var("I"), int(1)), Vec::new(), bin(BinOp::Eq, var("I"), int(2)));
        assert_eq!(labels(&stmt), vec!["1", "2"]);
    }

    #[test]
    fn test_branches_that_test_different_expressions_stay_an_if() {
        let stmt = reconstruct(
            bin(BinOp::Eq, var("I"), int(1)),
            vec![BreakStatement::create_empty_statement()],
            bin(BinOp::Eq, var("J"), int(2)),
        );
        assert!(matches!(stmt, Statement::IfThen(_)));
    }
}
