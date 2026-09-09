use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    ast::{BinOp, BinaryExpression, Expression, IdentifierExpression, output_visitor::OutputVisitor},
    compiler::workspace::Workspace,
    parser::{Encoding, ErrorReporter, Parser, UserTypeRegistry},
};

use super::add_parens_if_required;

fn variable(name: &str) -> Expression {
    IdentifierExpression::create_empty_expression(unicase::Ascii::new(name.to_string()))
}

fn shape(expression: &Expression) -> String {
    match expression {
        Expression::Parens(expression) => shape(expression.get_expression()),
        Expression::Binary(expression) => format!(
            "({} {:?} {})",
            shape(expression.get_left_expression()),
            expression.get_op(),
            shape(expression.get_right_expression())
        ),
        Expression::Identifier(expression) => expression.get_identifier().to_string(),
        other => panic!("unexpected expression {other:?}"),
    }
}

#[test]
fn every_binary_operator_pair_preserves_its_tree_on_both_sides() {
    let operators = [
        BinOp::PoW,
        BinOp::Mul,
        BinOp::Div,
        BinOp::Mod,
        BinOp::Add,
        BinOp::Sub,
        BinOp::Eq,
        BinOp::NotEq,
        BinOp::Lower,
        BinOp::LowerEq,
        BinOp::Greater,
        BinOp::GreaterEq,
        BinOp::And,
        BinOp::Or,
        BinOp::ShortAnd,
        BinOp::ShortOr,
    ];
    for parent in operators {
        for child in operators {
            for right_operand in [false, true] {
                let nested = BinaryExpression::create_empty_expression(child, variable("B"), variable("C"));
                let nested = add_parens_if_required(parent, nested, right_operand);
                let expression = if right_operand {
                    BinaryExpression::create_empty_expression(parent, variable("A"), nested)
                } else {
                    BinaryExpression::create_empty_expression(parent, nested, variable("A"))
                };
                let mut output = OutputVisitor::default();
                expression.visit(&mut output);
                let registry = UserTypeRegistry::default();
                let errors = Arc::new(Mutex::new(ErrorReporter::default()));
                let mut workspace = Workspace::default();
                workspace.set_default_language_version(Some(400));
                let mut parser = Parser::new(
                    PathBuf::from("expression.pps"),
                    errors.clone(),
                    &registry,
                    &output.output,
                    Encoding::Utf8,
                    &workspace,
                );
                parser.next_token();
                let parsed = parser.parse_expression().unwrap();
                assert_eq!(parser.get_cur_token(), None, "{}", output.output);
                assert!(!errors.lock().unwrap().has_errors(), "{}", output.output);
                assert_eq!(shape(&expression), shape(&parsed), "{}", output.output);
            }
        }
    }
}
