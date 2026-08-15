use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};

/// Compiles a snippet the way a tool without a manifest does, so only the source
/// itself can say which language it is written in.
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
fn the_declared_version_reaches_the_builtin_checks() {
    let errors = diagnostics("PRINTLN ISNONSTOP()\n");
    assert!(errors.is_empty(), "{errors:?}");

    let errors = diagnostics(";$LANGVERSION 100\nPRINTLN ISNONSTOP()\n");
    assert!(
        errors.iter().any(|e| e.contains("ISNONSTOP")),
        "a 2.00 function should not pass for 1.00: {errors:?}"
    );
}

#[test]
fn the_declared_version_reaches_the_keywords() {
    let errors = diagnostics(";$LANGVERSION 400\nBEGIN\n  EXIT\nEND\n");
    assert!(errors.is_empty(), "{errors:?}");

    // EXIT is a plain identifier before 400, so it is nothing the compiler knows.
    let errors = diagnostics(";$LANGVERSION 350\nEXIT\n");
    assert!(!errors.is_empty(), "EXIT should not be a statement for 350");
}
