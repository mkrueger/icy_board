use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};

/// Compiles a snippet and answers the diagnostics rather than the executable.
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
fn a_member_that_does_not_exist_is_reported_and_does_not_stop_the_compiler() {
    let errors = diagnostics("CONFERENCE c = CONFINFO(0)\nPRINTLN c.Nmae\n");
    assert!(errors.iter().any(|e| e == "Member not found"), "{errors:?}");
}

#[test]
fn a_member_call_that_does_not_exist_is_reported_and_does_not_stop_the_compiler() {
    let errors = diagnostics("CONFERENCE c = CONFINFO(0)\nPRINTLN c.HasAcces()\n");
    assert!(!errors.is_empty(), "an unknown member function should be reported");
}

#[test]
fn a_field_of_a_declared_record_compiles() {
    let errors = diagnostics("TYPE FooBar\n  INTEGER a\nENDTYPE\n\nFooBar foo\n\nfoo.a = 1\nPRINTLN foo.a\n");
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn a_field_a_record_does_not_have_is_reported() {
    let errors = diagnostics("TYPE FooBar\n  INTEGER a\nENDTYPE\n\nFooBar foo\n\nPRINTLN foo.b\n");
    assert!(errors.iter().any(|e| e == "Member not found"), "{errors:?}");
}

#[test]
fn assigning_to_a_field_a_record_does_not_have_is_reported() {
    let errors = diagnostics("TYPE FooBar\n  INTEGER a\nENDTYPE\n\nFooBar foo\n\nfoo.b = 1\n");
    assert!(errors.iter().any(|e| e == "Member not found"), "{errors:?}");
}

#[test]
fn assigning_to_a_board_object_member_is_reported() {
    let errors = diagnostics("CONFERENCE c = CONFINFO(0)\nc.Name = \"x\"\n");
    assert!(!errors.is_empty(), "writing to a board object should be reported");
}

#[test]
fn a_member_on_something_that_is_not_an_object_is_reported() {
    let errors = diagnostics("INTEGER i\nPRINTLN i.Name\n");
    assert!(errors.iter().any(|e| e == "Member not found"), "{errors:?}");
}
