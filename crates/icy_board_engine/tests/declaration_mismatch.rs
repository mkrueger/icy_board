use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};

fn diagnostics(source: &str) -> Vec<String> {
    let reg = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let workspace = Workspace::default();

    let ast = parse_ast(PathBuf::from("test.pps"), errors.clone(), source, &reg, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, reg, errors.clone());
    compiler.compile(&[&ast]);

    let reporter = errors.lock().unwrap();
    reporter.errors.iter().map(|e| e.error.to_string()).collect()
}

#[test]
fn a_return_type_that_disagrees_with_the_declaration_is_reported() {
    let errors = diagnostics("DECLARE FUNCTION Make() INTEGER\nPRINT Make()\nFUNCTION Make() STRING\n  RETURN \"x\"\nENDFUNC\n");
    assert!(
        errors.iter().any(|e| e == "FUNCTION return type does not match with declaration (Make)"),
        "{errors:?}"
    );
}

#[test]
fn a_parameter_count_that_disagrees_with_the_declaration_is_reported() {
    let errors = diagnostics("DECLARE FUNCTION F(INTEGER a) INTEGER\nPRINT F(1)\nFUNCTION F(INTEGER a, INTEGER b) INTEGER\n  RETURN 1\nENDFUNC\n");
    assert!(
        errors.iter().any(|e| e == "FUNCTION/PROCEDURE parameters not match with declaration (F)"),
        "{errors:?}"
    );
}

#[test]
fn a_declaration_that_agrees_with_the_implementation_is_accepted() {
    let errors = diagnostics("DECLARE FUNCTION Make() INTEGER\nPRINT Make()\nFUNCTION Make() INTEGER\n  RETURN 7\nENDFUNC\n");
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn a_routine_without_a_declaration_is_accepted() {
    let errors = diagnostics("PRINT Make()\nFUNCTION Make() INTEGER\n  RETURN 7\nENDFUNC\n");
    assert!(errors.is_empty(), "{errors:?}");
}
