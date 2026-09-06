//! PPLC 3.40 splits implementation formals at raw commas, but ignores DECLARE
//! dimensions. The unused-formal fixtures isolate syntax from body checking.
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    ast::{Ast, AstNode, ParameterSpecifier},
    compiler::{PPECompiler, workspace::Workspace},
    parser::{Encoding, ErrorReporter, ParserErrorType, UserTypeRegistry, lexer::Token, parse_ast},
};

const LEGACY_LANGUAGES: [u16; 6] = [300, 310, 320, 330, 340, 350];

fn parse(source: &str, language: u16, runtime: u16) -> (Ast, Arc<Mutex<ErrorReporter>>) {
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut workspace = Workspace::default();
    workspace.set_default_language_version(Some(language));
    workspace.package.runtime = Some(runtime);
    let ast = parse_ast(
        PathBuf::from("legacy_parameter_syntax.pps"),
        errors.clone(),
        source,
        &UserTypeRegistry::icy_board_registry(),
        Encoding::Utf8,
        &workspace,
    );
    (ast, errors)
}

fn accepted(source: &str, language: u16, runtime: u16) -> Ast {
    let (ast, errors) = parse(source, language, runtime);
    let messages = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>();
    assert!(messages.is_empty(), "language {language}, runtime {runtime}: {source}\n{messages:?}");
    ast
}

fn rejected_at_comma(source: &str, language: u16, runtime: u16) {
    let (_, errors) = parse(source, language, runtime);
    let reporter = errors.lock().unwrap();
    let messages = reporter.errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>();
    assert_eq!(reporter.errors.len(), 1, "language {language}, runtime {runtime}: {source}\n{messages:?}");
    let error = &reporter.errors[0];
    assert_eq!(
        error.error.downcast_ref::<ParserErrorType>(),
        Some(&ParserErrorType::MissingCloseParens(Token::Comma))
    );
    assert_eq!(&source[error.span.clone()], ",", "the diagnostic must identify the dimension separator");
}

fn routine(function: bool, declaration: Option<&str>, parameters: &str, body: &str) -> String {
    let (kind, result, end) = if function {
        ("FUNCTION", " INTEGER", "ENDFUNC")
    } else {
        ("PROCEDURE", "", "ENDPROC")
    };
    let declaration = declaration.map_or_else(String::new, |parameters| format!("DECLARE {kind} Work({parameters}){result}\n"));
    format!("{declaration}{kind} Work({parameters}){result}\n{body}\n{end}\n")
}

fn ranks(ast: &Ast) -> Vec<Vec<usize>> {
    ast.nodes
        .iter()
        .filter_map(|node| match node {
            AstNode::Function(node) => Some(node.get_parameters()),
            AstNode::Procedure(node) => Some(node.get_parameters()),
            AstNode::FunctionDeclaration(node) => Some(node.get_parameters()),
            AstNode::ProcedureDeclaration(node) => Some(node.get_parameters()),
            _ => None,
        })
        .map(|parameters| {
            parameters
                .iter()
                .map(|parameter| match parameter {
                    ParameterSpecifier::Variable(parameter) => parameter.get_variable().as_ref().unwrap().get_dimensions().len(),
                    _ => panic!("expected an ordinary parameter"),
                })
                .collect()
        })
        .collect()
}

#[test]
fn original_unused_formal_probes_are_rejected_by_the_parser() {
    // Actual PPLC.EXE 3.40 rejects both at line 4 with:
    // "Closing parenthesis not found (INTEGER VALUE(3)".
    for source in [
        include_str!("../../../compat/declare/dim_unused.pps"),
        include_str!("../../../compat/declare/dim_function_unused.pps"),
    ] {
        for language in LEGACY_LANGUAGES {
            for runtime in [340, 400] {
                rejected_at_comma(source, language, runtime);
            }
        }
    }
}

#[test]
fn legacy_multidimensional_formals_fail_with_unused_or_indexed_bodies() {
    for language in LEGACY_LANGUAGES {
        for runtime in [340, 400] {
            for function in [false, true] {
                for (bounds, index) in [("3, 4", "1, 2"), ("3, 4, 5", "1, 2, 3")] {
                    for body in [String::new(), format!("PRINTLN value({index})")] {
                        let parameters = format!("INTEGER first, INTEGER value({bounds}), INTEGER last");
                        rejected_at_comma(&routine(function, None, &parameters, &body), language, runtime);
                    }
                }
            }
            rejected_at_comma(&routine(false, None, "VAR INTEGER value(3, 4)", ""), language, runtime);
        }
    }
}

#[test]
fn legacy_scalar_and_one_dimensional_formals_keep_parameter_boundaries() {
    for language in LEGACY_LANGUAGES {
        for function in [false, true] {
            let source = routine(function, None, "INTEGER first, INTEGER value(3), INTEGER last", "PRINTLN value(1)");
            let ast = accepted(&source, language, 400);
            assert_eq!(ranks(&ast), [vec![0, 1, 0]]);
        }
        let ast = accepted(&routine(false, None, "VAR INTEGER value(3), INTEGER last", ""), language, 400);
        assert_eq!(ranks(&ast), [vec![1, 0]]);
    }
}

#[test]
fn declare_dimensions_do_not_restrict_legacy_scalar_implementations() {
    for language in [340, 350] {
        for runtime in [340, 400] {
            for function in [false, true] {
                for (bounds, rank) in [("3, 4", 2), ("3, 4, 5", 3)] {
                    let declaration = format!("INTEGER value({bounds})");
                    let source = routine(
                        function,
                        Some(&declaration),
                        "INTEGER value",
                        if function { "Work = value" } else { "PRINTLN value" },
                    );
                    let ast = accepted(&source, language, runtime);
                    assert_eq!(ranks(&ast), [vec![rank], vec![0]]);

                    let mut workspace = Workspace::default();
                    workspace.set_default_language_version(Some(language));
                    workspace.package.runtime = Some(runtime);
                    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
                    let mut compiler = PPECompiler::new(&workspace, UserTypeRegistry::icy_board_registry(), errors.clone());
                    compiler.compile(&[&ast]);
                    let messages = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>();
                    assert!(messages.is_empty(), "{source}\n{messages:?}");
                    assert!(compiler.create_executable().is_ok());
                }
            }
        }
    }
}

#[test]
fn modern_formals_preserve_all_fixed_and_dynamic_ranks() {
    for runtime in [340, 400] {
        for function in [false, true] {
            for (suffix, rank) in [
                ("(3)", 1),
                ("(3, 4)", 2),
                ("(3, 4, 5)", 3),
                ("[3]", 1),
                ("[3, 4]", 2),
                ("[3, 4, 5]", 3),
                ("[]", 1),
                ("[,]", 2),
                ("[,,]", 3),
            ] {
                let parameters = format!("INTEGER first, INTEGER value{suffix}, INTEGER last");
                let source = routine(function, Some(&parameters), &parameters, "");
                let ast = accepted(&source, 400, runtime);
                assert_eq!(ranks(&ast), [vec![0, rank, 0], vec![0, rank, 0]], "{source}");
            }
        }
    }
}

#[test]
fn legacy_global_and_local_multidimensional_arrays_are_unaffected() {
    for language in LEGACY_LANGUAGES {
        let source = "INTEGER global(3, 4, 5)\nPROCEDURE Work(INTEGER value)\nINTEGER local(3, 4)\nPRINTLN local(1, 2)\nENDPROC\n";
        accepted(source, language, 400);
    }
}

#[test]
fn nested_callback_signatures_keep_the_existing_language_gate() {
    let parameters = "FUNCTION outer(PROCEDURE inner(INTEGER value(3, 4)), INTEGER other) INTEGER, INTEGER last";
    for function in [false, true] {
        let source = routine(function, Some(parameters), parameters, "");
        accepted(&source, 400, 400);
        for language in [340, 350] {
            assert!(
                !parse(&source, language, 400).1.lock().unwrap().errors.is_empty(),
                "callbacks still require language 400"
            );
        }
    }
}

#[test]
fn modern_nested_callbacks_keep_dynamic_array_parameters_and_results() {
    for rank in ["[]", "[,]", "[,,]"] {
        for function in [false, true] {
            let parameters = format!(
                "FUNCTION outer(PROCEDURE inner(VAR INTEGER value{rank}), FUNCTION leaf(INTEGER item{rank}) INTEGER{rank}) INTEGER{rank}, INTEGER last"
            );
            let source = routine(function, Some(&parameters), &parameters, "");
            accepted(&source, 400, 400);
        }
    }
}
