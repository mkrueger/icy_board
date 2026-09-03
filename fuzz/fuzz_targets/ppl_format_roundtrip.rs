#![no_main]

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::workspace::Workspace,
    formatting::{FormattingOptions, FormattingVisitor, StringFormattingBackend},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};
use icy_board_fuzz::Program;
use libfuzzer_sys::fuzz_target;

fn parse(source: &str, workspace: &Workspace) -> (icy_board_engine::ast::Ast, bool) {
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(
        PathBuf::from("/nonexistent-fuzz-root/fuzz.pps"),
        errors.clone(),
        source,
        &registry,
        Encoding::Utf8,
        workspace,
    );
    let has_errors = errors.lock().unwrap().has_errors();
    (ast, has_errors)
}

// Formatting a source the parser accepted has to produce a source the parser still accepts, and
// formatting that result again has to change nothing.
fuzz_target!(|program: Program| {
    let source = program.render();
    let mut workspace = Workspace::default();
    workspace.set_default_language_version(Some(program.language_version()));

    let (ast, has_errors) = parse(&source, &workspace);
    if has_errors {
        return;
    }

    let mut backend = StringFormattingBackend::new(&source);
    let options = FormattingOptions::default();
    FormattingVisitor::new(&mut backend, &options).format(&ast);
    let formatted = backend.apply();

    let (formatted_ast, formatted_has_errors) = parse(&formatted, &workspace);
    assert!(!formatted_has_errors, "formatting broke a clean source:\n{source}\n--- became ---\n{formatted}");

    let mut backend = StringFormattingBackend::new(&formatted);
    FormattingVisitor::new(&mut backend, &options).format(&formatted_ast);
    let twice = backend.apply();
    assert!(twice == formatted, "formatting is not stable:\n{formatted}\n--- became ---\n{twice}");
});
