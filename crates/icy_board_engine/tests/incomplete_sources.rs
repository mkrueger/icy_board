//! An editor asks the parser to read half written lines all the time, so no
//! source may leave it spinning - not even one that stops in the middle of a
//! token at the end of the file.

use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        mpsc::{RecvTimeoutError, channel},
    },
    time::Duration,
};

use icy_board_engine::{
    compiler::workspace::Workspace,
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    semantic::SemanticVisitor,
};

fn analyze(source: &str) {
    let workspace = Workspace::default();
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(PathBuf::from("test.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut visitor = SemanticVisitor::new(&workspace, errors, registry);
    ast.visit(&mut visitor);
    visitor.finish();
}

#[test]
fn a_source_that_stops_in_the_middle_still_finishes() {
    // None of these end with a newline, which is what a file looks like while
    // it is being typed.
    let sources = [
        "conf.",
        "conf.Name",
        "TYPE Point\n  INTEGER X\nENDTYPE\nPoint p\np.",
        "x = ",
        "x = 1 +",
        "PRINTLN \"abc",
        "PRINTLN 1,",
        "IF (",
        "IF x THEN",
        "WHILE x DO",
        "FOR i = 0 TO",
        "SELECT CASE x",
        "Point p = Point {",
        "Point p = Point { X =",
        "DECLARE PROCEDURE p(",
        "PROCEDURE p()",
        "FUNCTION f() INTEGER",
        "@X",
        "0FF",
        "values[",
        ";$IF",
        "_",
        ".",
    ];

    for source in sources {
        let (sender, receiver) = channel();
        let text = source.to_string();
        let worker = std::thread::spawn(move || {
            analyze(&text);
            let _ = sender.send(());
        });
        match receiver.recv_timeout(Duration::from_secs(10)) {
            Ok(()) => {
                worker.join().unwrap();
            }
            Err(RecvTimeoutError::Timeout) => panic!("parsing {source:?} does not finish"),
            Err(RecvTimeoutError::Disconnected) => panic!("parsing {source:?} panicked"),
        }
    }
}

#[test]
fn deeply_nested_expressions_report_an_error_instead_of_overflowing_the_stack() {
    let depth = 5_000;
    let source = format!("PRINTLN {}1{}", "(".repeat(depth), ")".repeat(depth));
    let workspace = Workspace::default();
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));

    parse_ast(PathBuf::from("test.pps"), errors.clone(), &source, &registry, Encoding::Utf8, &workspace);

    assert!(errors.lock().unwrap().errors.iter().any(|error| {
        matches!(
            error.error.downcast_ref::<icy_board_engine::parser::ParserErrorType>(),
            Some(icy_board_engine::parser::ParserErrorType::ExpressionNestingTooDeep(64))
        )
    }));
}

#[test]
fn a_long_unary_chain_reports_an_error_instead_of_overflowing_the_stack() {
    let source = format!("PRINTLN {}1", "!".repeat(5_000));
    let workspace = Workspace::default();
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));

    parse_ast(PathBuf::from("test.pps"), errors.clone(), &source, &registry, Encoding::Utf8, &workspace);

    assert!(errors.lock().unwrap().errors.iter().any(|error| {
        matches!(
            error.error.downcast_ref::<icy_board_engine::parser::ParserErrorType>(),
            Some(icy_board_engine::parser::ParserErrorType::ExpressionNestingTooDeep(64))
        )
    }));
}
