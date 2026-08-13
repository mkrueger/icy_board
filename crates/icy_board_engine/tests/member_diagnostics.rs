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
fn records_of_different_declared_types_cannot_be_assigned() {
    let errors = diagnostics(
        "TYPE Alpha\n  INTEGER v\nENDTYPE\nTYPE Beta\n  INTEGER v\nENDTYPE\nAlpha first\nBeta second\nfirst = second\n",
    );
    assert!(errors.iter().any(|e| e == "Can't assign UserData(101) to UserData(100)"), "{errors:?}");
}

#[test]
fn a_scalar_cannot_replace_a_record() {
    let errors = diagnostics("TYPE Rec\n  INTEGER v\nENDTYPE\nRec item\nitem = 12\n");
    assert!(errors.iter().any(|e| e == "Can't assign Integer to UserData(100)"), "{errors:?}");
}

#[test]
fn a_record_cannot_be_stored_in_a_scalar() {
    let errors = diagnostics("TYPE Rec\n  INTEGER v\nENDTYPE\nRec item\nINTEGER n\nn = item\n");
    assert!(errors.iter().any(|e| e == "Can't assign UserData(100) to Integer"), "{errors:?}");
}

#[test]
fn a_nested_field_rejects_the_wrong_record_type() {
    let errors = diagnostics(
        "TYPE Inner\n  INTEGER v\nENDTYPE\nTYPE Other\n  INTEGER v\nENDTYPE\nTYPE Outer\n  Inner value\nENDTYPE\nOuter wrapper\nOther source\nwrapper.value = source\n",
    );
    assert!(errors.iter().any(|e| e == "Can't assign UserData(101) to UserData(100)"), "{errors:?}");
}

#[test]
fn type_and_field_names_ignore_case() {
    let errors = diagnostics("TYPE Mixed\n  INTEGER Value\nENDTYPE\nmIxEd item\nitem.vAlUe = 1\nPRINT item.VALUE\n");
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn a_routine_rejects_a_different_record_type() {
    let errors = diagnostics(
        "TYPE Alpha\n  INTEGER v\nENDTYPE\nTYPE Beta\n  INTEGER v\nENDTYPE\nBeta value\nTake(value)\nPROCEDURE Take(Alpha argument)\nENDPROC\n",
    );
    assert!(errors.iter().any(|e| e == "Argument 1 expects UserData(100), got UserData(101)"), "{errors:?}");
}

#[test]
fn a_routine_rejects_a_scalar_where_it_needs_a_record() {
    let errors = diagnostics("TYPE Rec\n  INTEGER v\nENDTYPE\nTake(12)\nPROCEDURE Take(Rec argument)\nENDPROC\n");
    assert!(errors.iter().any(|e| e == "Argument 1 expects UserData(100), got Integer"), "{errors:?}");
}

#[test]
fn a_routine_rejects_a_record_where_it_needs_a_scalar() {
    let errors = diagnostics("TYPE Rec\n  INTEGER v\nENDTYPE\nRec value\nTake(value)\nPROCEDURE Take(INTEGER argument)\nENDPROC\n");
    assert!(errors.iter().any(|e| e == "Argument 1 expects Integer, got UserData(100)"), "{errors:?}");
}

#[test]
fn arithmetic_on_records_is_rejected() {
    let errors = diagnostics("TYPE Rec\n  INTEGER v\nENDTYPE\nRec first\nRec second\nINTEGER n\nn = first + second\n");
    assert!(errors.iter().any(|e| e == "Operator + is not defined for custom types"), "{errors:?}");
}

#[test]
fn ordering_records_is_rejected() {
    let errors = diagnostics("TYPE Rec\n  INTEGER v\nENDTYPE\nRec first\nRec second\nIF first < second PRINT \"less\"\n");
    assert!(errors.iter().any(|e| e.contains("is not defined for custom types")), "{errors:?}");
}

#[test]
fn equality_and_inequality_are_defined_for_records() {
    let errors = diagnostics("TYPE Rec\n  INTEGER v\nENDTYPE\nRec first\nRec second\nIF first = second PRINT \"equal\"\nIF first <> second PRINT \"different\"\n");
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn a_record_cannot_be_compared_with_a_scalar() {
    let errors = diagnostics("TYPE Rec\n  INTEGER v\nENDTYPE\nRec item\nIF item = 1 PRINT \"equal\"\n");
    assert!(errors.iter().any(|e| e == "Can't compare UserData(100) with Integer"), "{errors:?}");
}

#[test]
fn records_of_different_types_cannot_be_compared() {
    let errors = diagnostics(
        "TYPE First\n  INTEGER v\nENDTYPE\nTYPE Second\n  INTEGER v\nENDTYPE\nFirst firstRecord\nSecond secondRecord\nIF firstRecord = secondRecord PRINT \"equal\"\n",
    );
    assert!(errors.iter().any(|e| e == "Can't compare UserData(100) with UserData(101)"), "{errors:?}");
}

#[test]
fn a_member_on_something_that_is_not_an_object_is_reported() {
    let errors = diagnostics("INTEGER i\nPRINTLN i.Name\n");
    assert!(errors.iter().any(|e| e == "Member not found"), "{errors:?}");
}
