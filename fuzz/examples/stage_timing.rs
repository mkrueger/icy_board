//! Times each stage of the compiler on a source file, so a slow input can be pinned to a stage.

use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    time::Instant,
};

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    semantic::SemanticVisitor,
};

fn main() {
    let path = std::env::args().nth(1).expect("usage: stage_timing <source.pps>");
    let source = std::fs::read_to_string(&path).expect("source");
    let workspace = Workspace::default();
    let file = PathBuf::from("stage.pps");

    let start = Instant::now();
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(file.clone(), errors.clone(), &source, &registry, Encoding::Utf8, &workspace);
    println!("parse      {:>8.3?}  diagnostics {}", start.elapsed(), errors.lock().unwrap().errors.len());

    let start = Instant::now();
    let mut visitor = SemanticVisitor::new(&workspace, errors.clone(), registry);
    ast.visit(&mut visitor);
    visitor.finish();
    println!("semantic   {:>8.3?}  diagnostics {}", start.elapsed(), errors.lock().unwrap().errors.len());

    let start = Instant::now();
    let registry = UserTypeRegistry::icy_board_registry();
    let compile_errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut compiler = PPECompiler::new(&workspace, registry, compile_errors.clone());
    compiler.compile(&[&ast]);
    println!(
        "compile    {:>8.3?}  diagnostics {}",
        start.elapsed(),
        compile_errors.lock().unwrap().errors.len()
    );
}
