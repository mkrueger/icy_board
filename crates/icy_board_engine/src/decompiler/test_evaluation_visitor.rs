use std::path::PathBuf;

use crate::{
    ast::Expression,
    executable::LAST_PPLC,
    parser::{Encoding, Parser},
};

use super::evaluation_visitor::OptimizationVisitor;

fn parse_expression(input: &str) -> Expression {
    let mut parser = Parser::new(PathBuf::from("."), input, Encoding::Utf8, LAST_PPLC);
    parser.next_token();
    let res = parser.parse_expression().unwrap();
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
