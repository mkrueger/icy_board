use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    ast::{Ast, AstNode, ParameterSpecifier, VariableParameterSpecifier},
    compiler::{PPECompiler, workspace::Workspace},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};

fn diagnostics(source: &str, language: u16, runtime: u16) -> Vec<String> {
    diagnostics_with_ast_edit(source, language, runtime, |_| {})
}

fn diagnostics_with_ast_edit(source: &str, language: u16, runtime: u16, edit: impl FnOnce(&mut Ast)) -> Vec<String> {
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut workspace = Workspace::default();
    workspace.set_default_language_version(Some(language));
    workspace.package.runtime = Some(runtime);
    let mut ast = parse_ast(
        PathBuf::from("strict_declare.pps"),
        errors.clone(),
        source,
        &registry,
        Encoding::Utf8,
        &workspace,
    );
    let parser_errors: Vec<_> = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect();
    assert!(parser_errors.is_empty(), "parser errors: {parser_errors:?}\n{source}");
    edit(&mut ast);

    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect()
}

fn routine(function: bool, expected: &str, actual: &str) -> String {
    if function {
        format!("DECLARE FUNCTION Work({expected}) INTEGER\nFUNCTION Work({actual}) INTEGER\nRETURN 1\nENDFUNC\n")
    } else {
        format!("DECLARE PROCEDURE Work({expected})\nPROCEDURE Work({actual})\nENDPROC\n")
    }
}

fn assert_parameter_mismatch(source: &str) {
    assert_eq!(
        diagnostics(source, 400, 400),
        ["FUNCTION/PROCEDURE parameters not match with declaration (Work)"],
        "{source}"
    );
}

fn assert_accepted(source: &str, language: u16, runtime: u16) {
    let errors = diagnostics(source, language, runtime);
    assert!(errors.is_empty(), "language {language}, runtime {runtime}: {errors:?}\n{source}");
}

#[test]
fn ordinary_parameter_types_must_match_for_functions_and_procedures() {
    for function in [false, true] {
        for (expected, actual) in [("INTEGER value", "STRING value"), ("STRING value", "INTEGER value")] {
            assert_parameter_mismatch(&routine(function, expected, actual));
        }
    }
}

#[test]
fn var_modes_must_match_for_functions_and_procedures() {
    for (expected, actual) in [("VAR INTEGER value", "INTEGER value"), ("INTEGER value", "VAR INTEGER value")] {
        assert_parameter_mismatch(&routine(false, expected, actual));
    }
    // The parser intentionally disallows direct VAR function parameters. Exercise
    // the semantic signature comparison without changing that header policy.
    let source = routine(true, "INTEGER original", "INTEGER renamed");
    for declaration_is_var in [false, true] {
        let errors = diagnostics_with_ast_edit(&source, 400, 400, |ast| {
            for node in &mut ast.nodes {
                let parameters = match node {
                    AstNode::FunctionDeclaration(declaration) if declaration_is_var => declaration.get_parameters_mut(),
                    AstNode::Function(function) if !declaration_is_var => function.get_parameters_mut(),
                    _ => continue,
                };
                let ParameterSpecifier::Variable(parameter) = &parameters[0] else {
                    panic!("expected an ordinary parameter");
                };
                parameters[0] = ParameterSpecifier::Variable(VariableParameterSpecifier::empty(
                    true,
                    parameter.get_variable_type(),
                    parameter.get_variable().clone(),
                ));
            }
        });
        assert_eq!(errors, ["FUNCTION/PROCEDURE parameters not match with declaration (Work)"]);
    }
}

#[test]
fn ranks_and_exact_bounds_must_match_for_functions_and_procedures() {
    for function in [false, true] {
        for (expected, actual) in [
            ("INTEGER value", "INTEGER value[0]"),
            ("INTEGER value[2]", "INTEGER value[2, 2]"),
            ("INTEGER value[]", "INTEGER value[, ]"),
            ("INTEGER value[2]", "INTEGER value[3]"),
            ("INTEGER value[2, 3]", "INTEGER value[2, 4]"),
            ("INTEGER value[2, 3, 4]", "INTEGER value[2, 3, 5]"),
        ] {
            assert_parameter_mismatch(&routine(function, expected, actual));
            assert_parameter_mismatch(&routine(function, actual, expected));
        }
    }
}

#[test]
fn dynamic_dimensions_are_not_bound_zero_for_functions_and_procedures() {
    for function in [false, true] {
        for (expected, actual) in [
            ("INTEGER value[]", "INTEGER value[0]"),
            ("INTEGER value[,]", "INTEGER value[0, 0]"),
            ("INTEGER value[,,]", "INTEGER value[0, 0, 0]"),
        ] {
            assert_parameter_mismatch(&routine(function, expected, actual));
            assert_parameter_mismatch(&routine(function, actual, expected));
        }
    }
}

#[test]
fn parameter_names_are_not_part_of_the_signature() {
    for function in [false, true] {
        for (expected, actual) in [
            ("INTEGER original", "INTEGER renamed"),
            ("VAR STRING original", "VAR STRING renamed"),
            ("INTEGER original[0]", "INTEGER renamed[0]"),
            ("INTEGER original[2, 3, 4]", "INTEGER renamed[2, 3, 4]"),
            ("INTEGER original[]", "INTEGER renamed[]"),
            ("INTEGER original[,]", "INTEGER renamed[,]"),
            ("INTEGER original[,,]", "INTEGER renamed[,,]"),
            (
                "FUNCTION original(PROCEDURE inner(VAR INTEGER value[])) INTEGER[]",
                "FUNCTION renamed(PROCEDURE other(VAR INTEGER changed[])) INTEGER[]",
            ),
        ] {
            if function && expected.starts_with("VAR ") {
                continue;
            }
            assert_accepted(&routine(function, expected, actual), 400, 400);
        }
    }
}

#[test]
fn callback_signatures_are_checked_recursively() {
    for function in [false, true] {
        for (expected, actual) in [
            ("INTEGER value", "STRING value"),
            ("VAR INTEGER value", "INTEGER value"),
            ("INTEGER value[2]", "INTEGER value[2, 2]"),
            ("INTEGER value[2, 3]", "INTEGER value[2, 4]"),
            ("INTEGER value[]", "INTEGER value[0]"),
        ] {
            for (expected, actual) in [(expected, actual), (actual, expected)] {
                assert_parameter_mismatch(&routine(
                    function,
                    &format!("FUNCTION outer(PROCEDURE inner({expected})) INTEGER"),
                    &format!("FUNCTION outer(PROCEDURE inner({actual})) INTEGER"),
                ));
            }
        }
    }
}

#[test]
fn callback_return_types_ranks_and_kinds_must_match() {
    for function in [false, true] {
        for (expected, actual) in [
            ("FUNCTION cb() INTEGER", "FUNCTION cb() STRING"),
            ("FUNCTION cb() INTEGER", "FUNCTION cb() INTEGER[]"),
            ("FUNCTION cb() INTEGER[]", "FUNCTION cb() INTEGER[,]"),
            ("FUNCTION cb() INTEGER", "PROCEDURE cb()"),
            ("PROCEDURE outer(FUNCTION cb() INTEGER[])", "PROCEDURE outer(FUNCTION cb() INTEGER)"),
        ] {
            assert_parameter_mismatch(&routine(function, expected, actual));
            assert_parameter_mismatch(&routine(function, actual, expected));
        }
    }
}

#[test]
fn structurally_identical_enums_and_records_are_distinct_parameter_types() {
    for definitions in [
        "ENUM First\nItem\nENDENUM\nENUM Second\nItem\nENDENUM\n",
        "TYPE First\nINTEGER value\nENDTYPE\nTYPE Second\nINTEGER value\nENDTYPE\n",
    ] {
        for function in [false, true] {
            for (expected, actual) in [
                ("First original", "Second renamed"),
                ("PROCEDURE cb(First value)", "PROCEDURE cb(Second value)"),
            ] {
                assert_parameter_mismatch(&format!("{definitions}{}", routine(function, expected, actual)));
            }
            assert_accepted(&format!("{definitions}{}", routine(function, "First original", "First renamed")), 400, 400);
        }
    }
}

#[test]
fn return_types_and_ranks_must_match() {
    for (expected, actual) in [("INTEGER", "STRING"), ("INTEGER", "INTEGER[]"), ("INTEGER[]", "INTEGER[,]")] {
        for (expected, actual) in [(expected, actual), (actual, expected)] {
            let source = format!("DECLARE FUNCTION Work() {expected}\nFUNCTION Work() {actual}\nENDFUNC\n");
            assert_eq!(
                diagnostics(&source, 400, 400),
                ["FUNCTION return type does not match with declaration (Work)"],
                "{source}"
            );
        }
    }
}

#[test]
fn nominal_return_types_must_match_even_with_identical_structure() {
    for definitions in [
        "ENUM First\nItem\nENDENUM\nENUM Second\nItem\nENDENUM\n",
        "TYPE First\nINTEGER value\nENDTYPE\nTYPE Second\nINTEGER value\nENDTYPE\n",
    ] {
        let source = format!("{definitions}DECLARE FUNCTION Work() First\nFUNCTION Work() Second\nENDFUNC\n");
        assert_eq!(
            diagnostics(&source, 400, 400),
            ["FUNCTION return type does not match with declaration (Work)"],
            "{source}"
        );
        assert_parameter_mismatch(&format!("{definitions}{}", routine(false, "FUNCTION cb() First", "FUNCTION cb() Second")));
    }
}

#[test]
fn routine_kinds_must_match_in_both_directions() {
    for (source, expected) in [
        (
            "DECLARE PROCEDURE Work()\nFUNCTION Work() INTEGER\nRETURN 1\nENDFUNC\n",
            vec!["Procedure used as function", "Missing FUNCTION/PROCEDURE definition. (Work)"],
        ),
        (
            "DECLARE FUNCTION Work() INTEGER\nPROCEDURE Work()\nENDPROC\n",
            vec!["Procedure used as function"],
        ),
    ] {
        assert_eq!(diagnostics(source, 400, 400), expected, "{source}");
    }
}

#[test]
fn implicit_signatures_without_declare_are_accepted() {
    assert_accepted(
        "PRINT Work(1)\nShow(2)\nFUNCTION Work(INTEGER value) INTEGER\nRETURN value\nENDFUNC\nPROCEDURE Show(INTEGER value)\nPRINT value\nENDPROC\n",
        400,
        350,
    );
    for function in [false, true] {
        let source = routine(
            function,
            "INTEGER unused",
            "INTEGER value[], FUNCTION cb(PROCEDURE inner(VAR INTEGER item)) INTEGER[]",
        );
        let (_, implementation) = source.split_once('\n').unwrap();
        assert_accepted(implementation, 400, 400);
    }
}

#[test]
fn source_language_not_runtime_controls_strict_scalar_signatures() {
    for function in [false, true] {
        let source = routine(function, "INTEGER value", "STRING value");
        assert_eq!(
            diagnostics(&source, 400, 350),
            ["FUNCTION/PROCEDURE parameters not match with declaration (Work)"]
        );
        assert_accepted(&source, 350, 400);
    }
    let source = "DECLARE FUNCTION Work() INTEGER\nFUNCTION Work() STRING\nRETURN \"ok\"\nENDFUNC\n";
    assert_eq!(diagnostics(source, 400, 350), ["FUNCTION return type does not match with declaration (Work)"]);
    assert_accepted(source, 350, 400);
    let source = "DECLARE PROCEDURE Work()\nFUNCTION Work() INTEGER\nRETURN 1\nENDFUNC\n";
    assert_eq!(
        diagnostics(source, 400, 350),
        ["Procedure used as function", "Missing FUNCTION/PROCEDURE definition. (Work)"]
    );
    assert_accepted(source, 350, 400);
}

#[test]
fn legacy_ordinary_parameters_remain_permissive() {
    for language in [340, 350] {
        for function in [false, true] {
            for (expected, actual) in [
                ("INTEGER value", "STRING value"),
                ("INTEGER value", "INTEGER value(2)"),
                ("INTEGER value(2, 3)", "INTEGER value(2)"),
                ("INTEGER value(2)", "INTEGER value(3)"),
            ] {
                assert_accepted(&routine(function, expected, actual), language, 400);
            }
        }
        for (expected, actual) in [("VAR INTEGER value", "INTEGER value"), ("INTEGER value", "VAR INTEGER value")] {
            assert_accepted(&routine(false, expected, actual), language, 400);
        }
        assert_accepted(
            "DECLARE FUNCTION Work() INTEGER\nFUNCTION Work() STRING\nWork = \"ok\"\nENDFUNC\n",
            language,
            400,
        );
    }
}
