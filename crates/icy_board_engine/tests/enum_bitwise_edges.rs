//! Independent compiler/source-semantic checks for nominal enum expressions.
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    semantic::SemanticVisitor,
};

const SPARSE: &str = "ENUM Bits\n One = 1\n Two = 2\nENDENUM\n";

#[test]
fn constant_bitwise_values_preserve_the_enum_default_marker() {
    use icy_board_engine::ast::{BinOp, BinaryExpression, Expression, IdentifierExpression, MemberReferenceExpression, const_enum_value};
    let registry = UserTypeRegistry::icy_board_registry();
    let member = |name: &str| {
        MemberReferenceExpression::create_empty_expression(
            Expression::Identifier(IdentifierExpression::empty(unicase::Ascii::new("RegexOptions".to_string()))),
            unicase::Ascii::new(name.to_string()),
        )
    };
    for (operator, expected) in [(BinOp::Or, 3), (BinOp::And, 0)] {
        let expression = Expression::Binary(BinaryExpression::empty(member("IgnoreCase"), operator, member("MultiLine")));
        let value = const_enum_value(&expression, &|_| None, &registry.enums()).unwrap();
        assert_eq!(expected, value.as_int());
        assert_eq!(
            0,
            value.emptied().as_int(),
            "a constant value must retain the enum's default, not its left operand"
        );
    }
}

fn diagnostics(body: &str, language: u16, source_semantics: bool) -> Vec<String> {
    let mut workspace = Workspace::default();
    workspace.package.runtime = Some(400);
    workspace.set_default_language_version(Some(language));
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let source = format!("{SPARSE}{body}\n");
    let ast = parse_ast(PathBuf::from("edges.pps"), errors.clone(), &source, &registry, Encoding::Utf8, &workspace);
    assert!(!errors.lock().unwrap().has_errors(), "test source must parse: {body}");
    if source_semantics {
        let mut visitor = SemanticVisitor::new(&workspace, errors.clone(), registry);
        ast.visit(&mut visitor);
        visitor.finish();
    } else {
        PPECompiler::new(&workspace, registry, errors.clone()).compile(&[&ast]);
    }
    let result = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect();
    result
}

#[test]
fn constant_cast_operands_with_enum_comparisons_accept_unnamed_results() {
    for language in [350, 400] {
        for source_semantics in [false, true] {
            for expression in [
                "Bits(2 + (Bits.One = Bits.One))",
                "Bits(2 + ((Bits.One | Bits.One) = Bits.One)) & Bits.One",
                "Bits(2 + (Bits(1) = Bits(1))) & Bits.One",
            ] {
                let body = format!("PRINT {expression}");
                let errors = diagnostics(&body, language, source_semantics);
                assert!(
                    errors.is_empty(),
                    "language={language}, source_semantics={source_semantics}, {body}: {errors:?}"
                );
            }
        }
    }
}

#[test]
fn const_casts_reject_dynamic_or_wrong_type_values_but_accept_unnamed_values() {
    for language in [350, 400] {
        for source_semantics in [false, true] {
            for body in [
                "INTEGER number = 1\nCONST Bits Value = Bits(number)",
                "CONST Bits Value = Bits(Bits.One)",
                "CONST Bits Value = Bits(1, 2)",
            ] {
                let errors = diagnostics(body, language, source_semantics);
                assert!(!errors.is_empty(), "language={language}, source_semantics={source_semantics}, accepted {body}");
            }
            for body in [
                "CONST Bits Value = Bits(3) & Bits.One",
                "CONST BOOLEAN Value = (Bits(3) & Bits.One) = Bits.One",
                "CONST Bits Value = Bits.One | Bits.Two",
                "CONST Bits Value = Bits.One & Bits.Two",
                "CONST Bits Value = (Bits.One | Bits.Two) & Bits.One",
                "CONST Bits Value = Bits(1) | Bits.One\nPRINT Value",
                "CONST Bits Value = Bits(2) & Bits.Two\nPRINT Value",
                "CONST BOOLEAN Value = (Bits(1) | Bits.One) = Bits.One\nPRINT Value",
            ] {
                assert!(diagnostics(body, language, source_semantics).is_empty(), "rejected {body}");
            }
        }
    }
}
