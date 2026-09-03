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
use libfuzzer_sys::fuzz_target;

// What the language server does to every keystroke: read the text and look at what
// it means. The path does not exist, so `;$INCLUDE:` stays off the real disk.
fuzz_target!(|data: &[u8]| {
    if data.len() > 64 * 1024 {
        return;
    }
    let source = String::from_utf8_lossy(data);
    let workspace = Workspace::default();
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));

    let ast = parse_ast(
        PathBuf::from("/nonexistent-fuzz-root/fuzz.pps"),
        errors.clone(),
        &source,
        &registry,
        Encoding::Utf8,
        &workspace,
    );

    let mut visitor = SemanticVisitor::new(&workspace, errors, registry);
    ast.visit(&mut visitor);
    visitor.finish();
});
