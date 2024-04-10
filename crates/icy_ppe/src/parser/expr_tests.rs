use crate::{
    ast::{
        BinOp, BinaryExpression, Constant, ConstantExpression, Expression, ParensExpression,
        PredefinedFunctionCallExpression, UnaryExpression, UnaryOp,
    },
    executable::{FuncOpCode, LAST_PPLC},
    parser::{Encoding, Parser},
};
use std::path::PathBuf;

fn parse_expression(input: &str) -> Expression {
    let mut parser = Parser::new(PathBuf::from("."), input, Encoding::Utf8, LAST_PPLC);
    parser.next_token();
    let res = parser.parse_expression().unwrap();
    assert_eq!(parser.get_cur_token(), None);
    res
}

fn check_expression(input: &str, check: &Expression) {
    let expr = parse_expression(input);
    assert!(
        expr.is_similar(check),
        "Expression {expr:?} is not similar to {check:?}"
    );
}

fn _check_error(input: &str) {
    let mut parser = Parser::new(PathBuf::from("."), input, Encoding::Utf8, LAST_PPLC);
    parser.next_token();
    let expr = parser.parse_expression();
    assert!(
        !parser.errors.lock().unwrap().has_errors(),
        "No error found parsed expr {expr:?}"
    );
}

#[test]
fn test_parse_parens() {
    check_expression(
        "(5)",
        &ParensExpression::create_empty_expression(ConstantExpression::create_empty_expression(
            Constant::Integer(5),
        )),
    );
}

#[test]
fn test_unary_expressions() {
    check_expression(
        "!FALSE",
        &UnaryExpression::create_empty_expression(
            UnaryOp::Not,
            ConstantExpression::create_empty_expression(Constant::Builtin(
                &crate::ast::constant::BuiltinConst::FALSE,
            )),
        ),
    );

    check_expression(
        "-5",
        &UnaryExpression::create_empty_expression(
            UnaryOp::Minus,
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );

    check_expression(
        "+5",
        &UnaryExpression::create_empty_expression(
            UnaryOp::Plus,
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
}

#[test]
fn test_parse_expression() {
    check_expression(
        "$42.42",
        &ConstantExpression::create_empty_expression(Constant::Money(4242)),
    );
    check_expression(
        "ABORT()",
        &PredefinedFunctionCallExpression::create_empty_expression(
            FuncOpCode::ABORT.get_definition(),
            Vec::new(),
        ),
    );
    check_expression(
        "ABS(5)",
        &PredefinedFunctionCallExpression::create_empty_expression(
            FuncOpCode::ABS.get_definition(),
            vec![ConstantExpression::create_empty_expression(
                Constant::Integer(5),
            )],
        ),
    );
}

#[test]
fn test_binary_expressions() {
    check_expression(
        "2^5",
        &BinaryExpression::create_empty_expression(
            BinOp::PoW,
            ConstantExpression::create_empty_expression(Constant::Integer(2)),
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
    check_expression(
        "2**5",
        &BinaryExpression::create_empty_expression(
            BinOp::PoW,
            ConstantExpression::create_empty_expression(Constant::Integer(2)),
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
    check_expression(
        "2*5",
        &BinaryExpression::create_empty_expression(
            BinOp::Mul,
            ConstantExpression::create_empty_expression(Constant::Integer(2)),
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
    check_expression(
        "2/5",
        &BinaryExpression::create_empty_expression(
            BinOp::Div,
            ConstantExpression::create_empty_expression(Constant::Integer(2)),
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
    check_expression(
        "2%5",
        &BinaryExpression::create_empty_expression(
            BinOp::Mod,
            ConstantExpression::create_empty_expression(Constant::Integer(2)),
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
    check_expression(
        "2+5",
        &BinaryExpression::create_empty_expression(
            BinOp::Add,
            ConstantExpression::create_empty_expression(Constant::Integer(2)),
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
    check_expression(
        "2-5",
        &BinaryExpression::create_empty_expression(
            BinOp::Sub,
            ConstantExpression::create_empty_expression(Constant::Integer(2)),
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
    check_expression(
        "2=5",
        &BinaryExpression::create_empty_expression(
            BinOp::Eq,
            ConstantExpression::create_empty_expression(Constant::Integer(2)),
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
    check_expression(
        "2==5",
        &BinaryExpression::create_empty_expression(
            BinOp::Eq,
            ConstantExpression::create_empty_expression(Constant::Integer(2)),
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
    check_expression(
        "2<>5",
        &BinaryExpression::create_empty_expression(
            BinOp::NotEq,
            ConstantExpression::create_empty_expression(Constant::Integer(2)),
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
    check_expression(
        "2!=5",
        &BinaryExpression::create_empty_expression(
            BinOp::NotEq,
            ConstantExpression::create_empty_expression(Constant::Integer(2)),
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
    check_expression(
        "2<5",
        &BinaryExpression::create_empty_expression(
            BinOp::Lower,
            ConstantExpression::create_empty_expression(Constant::Integer(2)),
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
    check_expression(
        "2<=5",
        &BinaryExpression::create_empty_expression(
            BinOp::LowerEq,
            ConstantExpression::create_empty_expression(Constant::Integer(2)),
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
    check_expression(
        "2>5",
        &BinaryExpression::create_empty_expression(
            BinOp::Greater,
            ConstantExpression::create_empty_expression(Constant::Integer(2)),
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
    check_expression(
        "2>=5",
        &BinaryExpression::create_empty_expression(
            BinOp::GreaterEq,
            ConstantExpression::create_empty_expression(Constant::Integer(2)),
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
    check_expression(
        "2&5",
        &BinaryExpression::create_empty_expression(
            BinOp::And,
            ConstantExpression::create_empty_expression(Constant::Integer(2)),
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
    check_expression(
        "2|5",
        &BinaryExpression::create_empty_expression(
            BinOp::Or,
            ConstantExpression::create_empty_expression(Constant::Integer(2)),
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
}
