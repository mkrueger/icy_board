use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    ast::Expression,
    compiler::workspace::Workspace,
    parser::{Encoding, ErrorReporter, Parser, UserTypeRegistry},
};

use super::evaluation_visitor::OptimizationVisitor;

fn parse_expression(input: &str) -> Expression {
    let reg = UserTypeRegistry::default();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut parser = Parser::new(PathBuf::from("."), errors, &reg, input, Encoding::Utf8, &Workspace::default());
    parser.next_token();
    let res: Expression = parser.parse_expression().unwrap();
    assert_eq!(parser.get_cur_token(), None);
    res
}

fn test_expr(input: &str, expected: &str) {
    let expr = parse_expression(input);

    let out_expr = expr.visit_mut(&mut OptimizationVisitor::default());

    assert_eq!(expected, out_expr.to_string());
}

#[test]
fn test_unary() {
    test_expr("!FALSE", "TRUE");
    test_expr("!TRUE", "FALSE");
    test_expr("!!FALSE", "FALSE");
    test_expr("!!TRUE", "TRUE");
    test_expr("+5", "5");
}

#[test]
fn test_binary() {
    test_expr("FALSE & A", "FALSE");
    test_expr("TRUE | A", "TRUE");

    test_expr("TRUE & A", "A");
    test_expr("FALSE | A", "A");

    test_expr("0 < 1", "TRUE");
    test_expr("0 > 1", "FALSE");

    test_expr("(0 > 1) & (A < B | B > C)", "FALSE");
}

#[test]
fn decompiler_folds_only_complete_constant_subtrees() {
    use super::evaluation_visitor::ConstantFolder;
    for input in ["0.5 * A", "0 * Probe()", "FALSE & Probe()", "TRUE | Probe()", "0 / A", "!(0 * Probe())"] {
        let expression = parse_expression(input);
        assert_eq!(
            expression.to_string(),
            expression.visit_mut(&mut ConstantFolder::default()).to_string(),
            "{input}"
        );
    }
    assert_eq!("2", parse_expression("0.5 * 4").visit_mut(&mut ConstantFolder::default()).to_string());
}

#[test]
fn condition_simplification_keeps_effects_and_faulting_operands() {
    use super::evaluation_visitor::simplify_condition;
    for input in [
        "FALSE & Probe()",
        "TRUE | Probe()",
        "FALSE & A[1]",
        "TRUE | A.Value",
        "FALSE & 1 / A > 0",
        "0.5 * A",
    ] {
        let expression = parse_expression(input);
        assert_eq!(expression.to_string(), simplify_condition(&expression).to_string(), "{input}");
    }
    for (input, expected) in [
        ("TRUE & A", "A"),
        ("FALSE | A", "A"),
        ("FALSE & A", "FALSE"),
        ("TRUE | A", "TRUE"),
        ("(TRUE & A >= 1) | (FALSE & A <= 1)", "A >= 1"),
    ] {
        assert_eq!(expected, simplify_condition(&parse_expression(input)).to_string(), "{input}");
    }
}
