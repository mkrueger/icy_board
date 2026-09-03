#![no_main]

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::workspace::Workspace,
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    semantic::SemanticVisitor,
};
use icy_board_fuzz::{Preprocessed, check_diagnostic_spans};
use libfuzzer_sys::fuzz_target;

// The lexer acts on directives before the parser sees a token, and it carries the conditional
// stack across them, so an arrangement a source would not write still has to end.
fuzz_target!(|program: Preprocessed| {
    let source = program.render();
    let file_name = PathBuf::from("/nonexistent-fuzz-root/fuzz.pps");

    let mut workspace = Workspace::default();
    workspace.set_default_language_version(Some(program.language_version()));
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));

    let ast = parse_ast(file_name.clone(), errors.clone(), &source, &registry, Encoding::Utf8, &workspace);

    let mut visitor = SemanticVisitor::new(&workspace, errors.clone(), registry);
    ast.visit(&mut visitor);
    visitor.finish();

    check_diagnostic_spans(&errors.lock().unwrap(), &source, &file_name);
});
