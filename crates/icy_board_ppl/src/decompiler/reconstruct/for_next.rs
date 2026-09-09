use crate::{
    ast::{BinOp, Constant, Expression, ForStatement, Statement, UnaryOp},
    semantic::SemanticVisitor,
};

pub fn scan_for_next(visitor: &SemanticVisitor, statements: &mut Vec<Statement>, lang_version: u16) {
    // FOR Header:
    // LET VAR001 = [START]
    // :LABEL002
    // IF (!(((1 < 0) + (VAR001 > [END])) & ((1 > 0) + (VAR001 <= [END])))) GOTO LABEL001
    // ...
    // LET VAR001 = VAR001 + [STEP]
    // GOTO LABEL002
    // :LABEL001
    //
    // Compilers before 3.40 invert the test and jump straight to the exit, which
    // leaves out the GOTO and the body label.
    if statements.len() < 4 {
        return;
    }
    for i in 0..statements.len() - 4 {
        if let Statement::Let(outer_let) = &statements[i] {
            let Statement::Label(for_loop_label) = &statements[i + 1] else { continue };
            let for_loop_label = for_loop_label.get_label().clone();
            let if_statement = &statements[i + 2];
            let m = match_for_header(if_statement);

            if let Some((index_variable, target_label, to_expr, comparison, exits_on_match)) = m {
                if outer_let.get_identifier() != &index_variable || !plain_assignment(outer_let) {
                    continue;
                }
                let (skip_label, body_start) = if exits_on_match {
                    (target_label.clone(), i + 3)
                } else {
                    let Statement::Goto(skip_label_stmt) = &statements[i + 3] else {
                        continue;
                    };
                    let Statement::Label(body_label_stmt) = &statements[i + 4] else {
                        continue;
                    };
                    if body_label_stmt.get_label() != &target_label {
                        continue;
                    }
                    (skip_label_stmt.get_label().clone(), i + 5)
                };

                let mut matching_goto = -1;
                for j in body_start..statements.len() - 1 {
                    let Statement::Goto(goto_label) = &statements[j] else {
                        continue;
                    };
                    let Statement::Label(label) = &statements[j + 1] else {
                        continue;
                    };
                    if goto_label.get_label() == &for_loop_label && label.get_label() == &skip_label {
                        matching_goto = j as i32;
                        break;
                    }
                }
                if matching_goto < 0 || (matching_goto as usize) <= body_start {
                    continue;
                }

                let Statement::Let(inner_let) = &statements[matching_goto as usize - 1] else {
                    continue;
                };
                let Expression::Binary(bin_expr) = inner_let.get_value_expression() else {
                    continue;
                };
                if inner_let.get_identifier() != &index_variable || !plain_assignment(inner_let) {
                    continue;
                }
                if bin_expr.get_op() != BinOp::Add {
                    continue;
                } // always add even if step is negative
                let Expression::Identifier(left) = bin_expr.get_left_expression() else {
                    continue;
                };
                if left.get_identifier() != &index_variable {
                    continue;
                }
                let step_expr = bin_expr.get_right_expression().clone();
                // A simplified comparison only proves a FOR when STEP has a
                // known, nonzero sign and agrees with the comparison direction.
                let Some(step) = literal_step(&step_expr) else { continue };
                if step == 0.0 || !step.is_finite() || (step > 0.0) != matches!(comparison, BinOp::LowerEq | BinOp::Greater) {
                    continue;
                }

                let back_edge = matching_goto as usize;
                let body = &statements[body_start..back_edge - 1];
                // Jumping to the condition skips the increment. It is NOT a FOR
                // CONTINUE; leave this shape for WHILE (which retains the head).
                if super::label_references(body, &for_loop_label) != 0
                    || (!exits_on_match && super::label_references(body, &target_label) != 0)
                    || super::has_external_entries(visitor, statements, i + 1..back_edge + 1)
                {
                    continue;
                }

                let from_expr: Expression = outer_let.get_value_expression().clone();
                let var_name = outer_let.get_identifier().clone();

                let mut for_block: Vec<Statement> = statements.drain(i..=(matching_goto as usize)).collect();
                // pop for header
                for_block.drain(0..body_start - i);

                // Keep an increment label in the body: nested loops may still
                // jump there, since their jumps must not become inner CONTINUEs.
                for_block.pop();
                for_block.pop();
                let continue_label = super::get_last_label(&for_block);
                super::optimize_block(visitor, &mut for_block, lang_version);
                super::handle_break_continue(skip_label, continue_label, &mut for_block);
                if matches!(&step_expr, Expression::Const(value) if matches!(value.get_constant_value(), Constant::Integer(1, _))) {
                    statements.insert(i, ForStatement::create_empty_statement(var_name, from_expr, to_expr, None, for_block));
                } else {
                    statements.insert(
                        i,
                        ForStatement::create_empty_statement(var_name, from_expr, to_expr, Some(Box::new(step_expr)), for_block),
                    );
                }
                break;
            }
        }
    }
}

fn match_for_header(
    if_statement: &Statement,
) -> Option<(
    unicase::Ascii<String>, // indexName
    unicase::Ascii<String>, // for_label
    Expression,             // to_expr
    BinOp,                  // comparison direction
    bool,                   // the goto leaves the loop instead of entering the body
)> {
    match if_statement {
        Statement::If(if_stmt) => {
            if let Expression::Binary(bin_op) = if_stmt.get_condition() {
                let exits_on_match = match bin_op.get_op() {
                    BinOp::LowerEq | BinOp::GreaterEq => false,
                    BinOp::Greater | BinOp::Lower => true,
                    _ => return None,
                };
                let Expression::Identifier(index_variable) = bin_op.get_left_expression() else {
                    return None;
                };
                let to_expr: Expression = bin_op.get_right_expression().clone();

                let Statement::Goto(for_label) = if_stmt.get_statement() else {
                    return None;
                };

                return Some((
                    index_variable.get_identifier().clone(),
                    for_label.get_label().clone(),
                    to_expr,
                    bin_op.get_op(),
                    exits_on_match,
                ));
            }
        }
        _ => return None,
    }

    None
}

fn plain_assignment(statement: &crate::ast::LetStatement) -> bool {
    statement.get_arguments().is_empty()
        && statement.get_members().is_empty()
        && statement.get_target_expression().is_none()
        && *statement.get_let_variant() == crate::parser::lexer::Token::Eq
}

/// Do not evaluate arbitrary expressions here: names/calls can change sign, and
/// constant arithmetic may overflow in the target runtime's numeric type.
fn literal_step(expression: &Expression) -> Option<f64> {
    match expression {
        Expression::Const(value) => match value.get_constant_value() {
            Constant::Integer(value, _) => Some(*value as f64),
            Constant::Unsigned(value, _) if *value <= i32::MAX as u64 => Some(*value as f64),
            // Decimal literals may be lowered to REAL; reject underflow and
            // overflow there as well as non-finite source values.
            Constant::Double(value) => Some(*value as f32 as f64),
            _ => None,
        },
        Expression::Parens(value) => literal_step(value.get_expression()),
        Expression::Unary(value) => match value.get_op() {
            UnaryOp::Minus => literal_step(value.get_expression())
                .filter(|value| *value != i32::MIN as f64)
                .map(|value| -value),
            UnaryOp::Plus => literal_step(value.get_expression()),
            _ => None,
        },
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ast::{ConstantExpression, IfStatement, LetStatement, UnaryExpression},
        decompiler::reconstruct::{label_references, tests::*, while_do::scan_do_while},
        parser::lexer::Token,
    };

    fn candidate(comparison: BinOp, step: Expression, body: Vec<Statement>) -> Vec<Statement> {
        let exits = matches!(comparison, BinOp::Greater | BinOp::Lower);
        let mut statements = vec![
            assign("i", int(1)),
            label("head"),
            IfStatement::create_empty_statement(bin(comparison, var("I"), int(10)), goto(if exits { "exit" } else { "body" })),
        ];
        if !exits {
            statements.extend([goto("exit"), label("body")]);
        }
        statements.extend(body);
        statements.extend([label("increment"), assign("i", bin(BinOp::Add, var("I"), step)), goto("head"), label("exit")]);
        statements
    }

    fn assert_rejected(mut statements: Vec<Statement>) {
        let original = statements.clone();
        scan_for_next(&visitor(), &mut statements, 400);
        assert_eq!(statements, original, "uncertain FOR must leave all statements intact");
    }

    #[test]
    fn comparison_direction_must_match_nonzero_literal_step() {
        for comparison in [BinOp::Greater, BinOp::Lower, BinOp::LowerEq, BinOp::GreaterEq] {
            for step in [-2, 0, 2] {
                let mut statements = candidate(comparison, int(step), Vec::new());
                if step != 0 && (step > 0) == matches!(comparison, BinOp::Greater | BinOp::LowerEq) {
                    scan_for_next(&visitor(), &mut statements, 400);
                    let Statement::For(statement) = &statements[0] else {
                        panic!("expected FOR: {statements:?}")
                    };
                    assert_eq!(statement.get_identifier(), &name("i"));
                    assert_eq!(statement.get_step_expr().as_deref(), Some(&int(step)));
                } else {
                    assert_rejected(statements);
                }
            }
        }
    }

    #[test]
    fn dynamic_or_unproven_steps_are_not_reconstructed() {
        for step in [var("step"), bin(BinOp::Add, int(i32::MAX), int(1)), bin(BinOp::Sub, var("i"), int(2))] {
            assert_rejected(candidate(BinOp::Greater, step, Vec::new()));
        }
        let negative = UnaryExpression::create_empty_expression(UnaryOp::Minus, int(2));
        let mut statements = candidate(BinOp::Lower, negative, Vec::new());
        scan_for_next(&visitor(), &mut statements, 400);
        assert!(matches!(statements[0], Statement::For(_)));
    }

    #[test]
    fn nonfinite_underflowing_and_overflowing_steps_are_rejected() {
        for step in [f64::NAN, f64::INFINITY, f64::NEG_INFINITY, f64::MIN_POSITIVE, f64::MAX] {
            let mut statements = candidate(BinOp::Greater, ConstantExpression::create_empty_expression(Constant::Double(step)), Vec::new());
            // NaN does not implement reflexive equality, so inspect the shape
            // instead of comparing cloned ASTs in this test.
            let length = statements.len();
            scan_for_next(&visitor(), &mut statements, 400);
            assert_eq!(statements.len(), length);
            assert!(matches!(statements[0], Statement::Let(_)));
        }
        assert_rejected(candidate(
            BinOp::Greater,
            UnaryExpression::create_empty_expression(UnaryOp::Minus, int(i32::MIN)),
            Vec::new(),
        ));
    }

    #[test]
    fn unlabeled_increment_does_not_invent_a_continue_target() {
        let mut statements = candidate(BinOp::Greater, int(1), vec![assign("x", int(1))]);
        statements.remove(statements.len() - 4);
        scan_for_next(&visitor(), &mut statements, 400);
        let Statement::For(statement) = &statements[0] else { panic!("expected FOR") };
        assert_eq!(statement.get_statements(), &vec![assign("x", int(1))]);
    }

    #[test]
    fn initialization_comparison_increment_and_increment_left_must_agree() {
        for (initial, comparison, increment, left) in [("j", "i", "j", "i"), ("i", "j", "i", "j"), ("i", "i", "j", "i"), ("i", "i", "i", "j")] {
            let mut statements = candidate(BinOp::Greater, int(1), Vec::new());
            statements[0] = assign(initial, int(1));
            statements[2] = IfStatement::create_empty_statement(bin(BinOp::Greater, var(comparison), int(10)), goto("exit"));
            let increment_index = statements.len() - 3;
            statements[increment_index] = assign(increment, bin(BinOp::Add, var(left), int(1)));
            assert_rejected(statements);
        }
    }

    #[test]
    fn increment_left_must_be_an_identifier_and_targets_must_be_scalar() {
        for left in [int(7), bin(BinOp::Add, var("i"), int(2))] {
            let mut statements = candidate(BinOp::Greater, int(1), Vec::new());
            let increment_index = statements.len() - 3;
            statements[increment_index] = assign("i", bin(BinOp::Add, left, int(1)));
            assert_rejected(statements);
        }
        for initialization in [false, true] {
            let mut statements = candidate(BinOp::Greater, int(1), Vec::new());
            let index = if initialization { 0 } else { statements.len() - 3 };
            statements[index] = LetStatement::create_empty_statement(name("i"), Token::Eq, vec![int(0)], bin(BinOp::Add, var("i"), int(1)));
            assert_rejected(statements);
        }
        let mut statements = candidate(BinOp::Greater, int(1), Vec::new());
        statements[0] = Statement::Let(LetStatement::empty(name("i"), Token::Eq, Vec::new(), int(1)).with_target_expression(var("other")));
        assert_rejected(statements);
    }

    #[test]
    fn for_continue_targets_increment_and_nested_jumps_keep_the_label() {
        let nested = foreach(vec![goto("increment"), goto("exit")]);
        let mut statements = candidate(BinOp::Greater, int(1), vec![goto("increment"), nested.clone(), goto("exit")]);
        scan_for_next(&visitor(), &mut statements, 400);
        let Statement::For(statement) = &statements[0] else { panic!("expected FOR") };
        assert!(statement.get_step_expr().is_none());
        assert!(matches!(statement.get_statements()[0], Statement::Continue(_)));
        assert_eq!(statement.get_statements()[1], nested);
        assert!(matches!(statement.get_statements()[2], Statement::Break(_)));
        assert_eq!(statement.get_statements().last(), Some(&label("increment")));
        assert_eq!(statements.last(), Some(&label("exit")));
    }

    #[test]
    fn body_head_jumps_prevent_for_but_allow_while_fallback() {
        for comparison in [BinOp::Greater, BinOp::LowerEq] {
            for nested in [false, true] {
                let jump = if nested { foreach(vec![goto("head")]) } else { goto("head") };
                let mut statements = candidate(comparison, int(1), vec![jump]);
                assert_rejected(statements.clone());
                scan_do_while(&visitor(), &mut statements, 400);
                assert_eq!(statements[1], label("head"));
                assert!(matches!(statements[2], Statement::WhileDo(_)));
                assert_eq!(label_references(&statements, &name("head")), usize::from(nested));
            }
        }
    }

    #[test]
    fn external_entries_and_body_label_jumps_prevent_for_label_removal() {
        for target in ["head", "body", "increment"] {
            for before in [false, true] {
                let mut statements = candidate(BinOp::LowerEq, int(1), Vec::new());
                if before {
                    statements.insert(0, goto(target));
                } else {
                    statements.push(goto(target));
                }
                assert_rejected(statements);
            }
        }
        assert_rejected(candidate(BinOp::LowerEq, int(1), vec![foreach(vec![goto("body")])]));
    }

    #[test]
    fn raw_inner_while_is_reconstructed_before_outer_for_jumps() {
        let inner = raw_while("inner", "inner_exit", vec![goto("exit"), goto("increment"), assign("x", int(1))], false);
        let mut statements = candidate(BinOp::Greater, int(1), inner);
        scan_for_next(&visitor(), &mut statements, 400);
        let Statement::For(statement) = &statements[0] else { panic!("expected FOR") };
        assert!(statement.get_statements().iter().any(|statement| matches!(statement, Statement::WhileDo(_))));
        assert_eq!(label_references(&statements, &name("exit")), 1);
        assert_eq!(label_references(&statements, &name("increment")), 1);
    }
}
