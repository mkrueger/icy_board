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
use icy_board_fuzz::{Program, check_diagnostic_spans};
use libfuzzer_sys::fuzz_target;

// An editor parses a file that stops mid token on nearly every keystroke, so every prefix of a
// source has to terminate and stay describable.
fuzz_target!(|program: Program| {
    let source = program.render();
    if source.len() > 32 * 1024 {
        return;
    }
    let mut workspace = Workspace::default();
    workspace.set_default_language_version(Some(program.language_version()));
    let file_name = PathBuf::from("/nonexistent-fuzz-root/fuzz.pps");

    // Whole file plus a handful of cut points, since checking every byte would only repeat work.
    let mut cuts = vec![0, source.len()];
    for divisor in [8, 4, 3, 2] {
        cuts.push(source.len() / divisor);
    }
    cuts.push(source.len().saturating_sub(1));

    for cut in cuts {
        let mut cut = cut.min(source.len());
        while cut > 0 && !source.is_char_boundary(cut) {
            cut -= 1;
        }
        let prefix = &source[..cut];

        let registry = UserTypeRegistry::icy_board_registry();
        let errors = Arc::new(Mutex::new(ErrorReporter::default()));
        let ast = parse_ast(file_name.clone(), errors.clone(), prefix, &registry, Encoding::Utf8, &workspace);

        let mut visitor = SemanticVisitor::new(&workspace, errors.clone(), registry);
        ast.visit(&mut visitor);
        visitor.finish();

        let reporter = errors.lock().unwrap();
        check_diagnostic_spans(&reporter, prefix, &file_name);
    }
});
