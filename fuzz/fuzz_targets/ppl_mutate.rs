#![no_main]

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};
use icy_board_fuzz::{MutatedSource, check_diagnostic_spans};
use libfuzzer_sys::fuzz_target;

// Real sources damaged in ways that keep them looking like PPL, so the mutator reaches the parts
// of the compiler that only a nearly valid file gets to.
fuzz_target!(|mutation: MutatedSource| {
    let source = mutation.render();
    let file_name = PathBuf::from("/nonexistent-fuzz-root/fuzz.pps");

    let mut workspace = Workspace::default();
    workspace.set_default_language_version(Some(mutation.language_version()));
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));

    let ast = parse_ast(file_name.clone(), errors.clone(), &source, &registry, Encoding::Utf8, &workspace);

    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);

    let reporter = errors.lock().unwrap();
    check_diagnostic_spans(&reporter, &source, &file_name);
    if reporter.has_errors() {
        return;
    }
    drop(reporter);

    if let Ok(executable) = compiler.create_executable() {
        let _ = executable.to_buffer();
    }
});
