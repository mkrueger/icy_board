use crate::{
    ast::{BinOp, BinaryExpression, Constant, ConstantExpression, Expression, ParensExpression, UnaryExpression, UnaryOp, constant::NumberFormat},
    compiler::workspace::Workspace,
    parser::{Encoding, ErrorReporter, Parser, UserTypeRegistry},
};
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

fn parse_expression(input: &str) -> Expression {
    parse_expression_version(input, 400)
}

fn parse_expression_version(input: &str, language: u16) -> Expression {
    let reg = UserTypeRegistry::default();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut workspace = Workspace::default();
    workspace.set_default_language_version(Some(language));
    let mut parser = Parser::new(PathBuf::from("."), errors, &reg, input, Encoding::Utf8, &workspace);
    parser.next_token();
    let res = parser.parse_expression().unwrap();
    assert_eq!(parser.get_cur_token(), None);
    res
}

fn check_expression(input: &str, check: &Expression) {
    let expr = parse_expression(input);
    assert!(expr.is_similar(check), "Expression {expr} is not similar to {check}");
}

fn _check_error(input: &str) {
    let reg = UserTypeRegistry::default();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut parser = Parser::new(PathBuf::from("."), errors, &reg, input, Encoding::Utf8, &Workspace::default());
    parser.next_token();
    let expr = parser.parse_expression();
    assert!(!parser.error_reporter.lock().unwrap().has_errors(), "No error found parsed expr {expr:?}");
}

#[test]
fn test_parse_parens() {
    check_expression(
        "(5)",
        &ParensExpression::create_empty_expression(ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default))),
    );
}

#[test]
fn s2_original_logical_precedence() {
    for (source, or_op, and_op) in [
        ("TRUE | FALSE & FALSE", BinOp::Or, BinOp::And),
        ("TRUE || FALSE && FALSE", BinOp::ShortOr, BinOp::ShortAnd),
    ] {
        let Expression::Binary(outer) = parse_expression(source) else {
            panic!("expected OR")
        };
        assert_eq!(outer.get_op(), or_op);
        let Expression::Binary(inner) = outer.get_right_expression() else {
            panic!("expected AND")
        };
        assert_eq!(inner.get_op(), and_op);
    }
    let Expression::Unary(outer) = parse_expression("!1 = 2") else {
        panic!("expected NOT")
    };
    assert_eq!(outer.get_op(), UnaryOp::Not);
    let Expression::Binary(inner) = outer.get_expression() else {
        panic!("expected comparison")
    };
    assert_eq!(inner.get_op(), BinOp::Eq);
    let Expression::Binary(outer) = parse_expression("!1 = 2 & TRUE") else {
        panic!("expected AND")
    };
    assert_eq!(outer.get_op(), BinOp::And);
    assert!(matches!(outer.get_left_expression(), Expression::Unary(_)));
    let Expression::Binary(outer) = parse_expression_version("TRUE || FALSE && FALSE", 340) else {
        panic!("expected OR")
    };
    assert_eq!(outer.get_op(), BinOp::Or);
    let Expression::Binary(inner) = outer.get_right_expression() else {
        panic!("expected AND")
    };
    assert_eq!(inner.get_op(), BinOp::And);
}

#[test]
fn test_unary_expressions() {
    check_expression(
        "!FALSE",
        &UnaryExpression::create_empty_expression(
            UnaryOp::Not,
            ConstantExpression::create_empty_expression(Constant::Builtin(&crate::ast::constant::BuiltinConst::FALSE)),
        ),
    );

    check_expression(
        "-5",
        &UnaryExpression::create_empty_expression(
            UnaryOp::Minus,
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );

    check_expression(
        "+5",
        &UnaryExpression::create_empty_expression(
            UnaryOp::Plus,
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
}

#[test]
fn test_parse_expression() {
    check_expression("$42.42", &ConstantExpression::create_empty_expression(Constant::Money(4242)));
    /*
    check_expression(
        "ABORT()",
        &PredefinedFunctionCallExpression::create_empty_expression(FuncOpCode::ABORT.get_definition(), Vec::new()),
    );
    check_expression(
        "ABS(5)",
        &PredefinedFunctionCallExpression::create_empty_expression(
            FuncOpCode::ABS.get_definition(),
            vec![ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default))],
        ),
    );*/
}

#[test]
fn test_binary_expressions() {
    check_expression(
        "2^5",
        &BinaryExpression::create_empty_expression(
            BinOp::PoW,
            ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
    check_expression(
        "2**5",
        &BinaryExpression::create_empty_expression(
            BinOp::PoW,
            ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
    check_expression(
        "2*5",
        &BinaryExpression::create_empty_expression(
            BinOp::Mul,
            ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
    check_expression(
        "2/5",
        &BinaryExpression::create_empty_expression(
            BinOp::Div,
            ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
    check_expression(
        "2%5",
        &BinaryExpression::create_empty_expression(
            BinOp::Mod,
            ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
    check_expression(
        "2+5",
        &BinaryExpression::create_empty_expression(
            BinOp::Add,
            ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
    check_expression(
        "2-5",
        &BinaryExpression::create_empty_expression(
            BinOp::Sub,
            ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
    check_expression(
        "2=5",
        &BinaryExpression::create_empty_expression(
            BinOp::Eq,
            ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
    check_expression(
        "2==5",
        &BinaryExpression::create_empty_expression(
            BinOp::Eq,
            ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
    check_expression(
        "2<>5",
        &BinaryExpression::create_empty_expression(
            BinOp::NotEq,
            ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
    check_expression(
        "2!=5",
        &BinaryExpression::create_empty_expression(
            BinOp::NotEq,
            ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
    check_expression(
        "2<5",
        &BinaryExpression::create_empty_expression(
            BinOp::Lower,
            ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
    check_expression(
        "2<=5",
        &BinaryExpression::create_empty_expression(
            BinOp::LowerEq,
            ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
    check_expression(
        "2>5",
        &BinaryExpression::create_empty_expression(
            BinOp::Greater,
            ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
    check_expression(
        "2>=5",
        &BinaryExpression::create_empty_expression(
            BinOp::GreaterEq,
            ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
    check_expression(
        "2&5",
        &BinaryExpression::create_empty_expression(
            BinOp::And,
            ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
    check_expression(
        "2|5",
        &BinaryExpression::create_empty_expression(
            BinOp::Or,
            ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
}
