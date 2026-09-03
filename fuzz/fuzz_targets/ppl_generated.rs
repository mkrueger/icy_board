#![no_main]

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};
use icy_board_fuzz::{Program, check_diagnostic_spans};
use libfuzzer_sys::fuzz_target;

// A source built to look like PPL, so the mutator spends its time on the compiler rather than on
// getting past the lexer.
fuzz_target!(|program: Program| {
    let source = program.render();
    let file_name = PathBuf::from("/nonexistent-fuzz-root/fuzz.pps");

    let mut workspace = Workspace::default();
    workspace.set_default_language_version(Some(program.language_version()));
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
