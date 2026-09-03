#![no_main]

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};
use libfuzzer_sys::fuzz_target;

// The whole way a source becomes a PPE, which is what pplc does to a file it is given.
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

    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);

    // pplc only builds an executable once the diagnostics are clean, so the fuzzer must not
    // hand the code generator an AST the front end already rejected.
    if errors.lock().unwrap().has_errors() {
        return;
    }
    if let Ok(executable) = compiler.create_executable() {
        let _ = executable.to_buffer();
    }
});
