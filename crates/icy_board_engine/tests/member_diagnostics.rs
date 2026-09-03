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
    assert!(errors.iter().any(|e| e == "Record type UserData(100) has no member named b"), "{errors:?}");
}

#[test]
fn assigning_to_a_field_a_record_does_not_have_is_reported() {
    let errors = diagnostics("TYPE FooBar\n  INTEGER a\nENDTYPE\n\nFooBar foo\n\nfoo.b = 1\n");
    assert!(errors.iter().any(|e| e == "Record type UserData(100) has no member named b"), "{errors:?}");
}

#[test]
fn assigning_to_a_board_object_member_is_reported() {
    let errors = diagnostics("CONFERENCE c = CONFINFO(0)\nc.Name = \"x\"\n");
    assert!(!errors.is_empty(), "writing to a board object should be reported");
}

#[test]
fn records_of_different_declared_types_cannot_be_assigned() {
    let errors = diagnostics("TYPE Alpha\n  INTEGER v\nENDTYPE\nTYPE Beta\n  INTEGER v\nENDTYPE\nAlpha first\nBeta second\nfirst = second\n");
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
    let errors =
        diagnostics("TYPE Alpha\n  INTEGER v\nENDTYPE\nTYPE Beta\n  INTEGER v\nENDTYPE\nBeta value\nTake(value)\nPROCEDURE Take(Alpha argument)\nENDPROC\n");
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
fn a_procedure_call_with_a_missing_argument_is_reported_without_panicking() {
    let errors = diagnostics("PROCEDURE FooBar(INTEGER a)\n  PRINTLN a\nENDPROC\nBEGIN\n  FooBar()\nEND\n");
    assert!(errors.iter().any(|e| e == "Not enough arguments passed (FooBar:0:1)"), "{errors:?}");
}

#[test]
fn missing_callable_arguments_are_reported_without_panicking() {
    let sources = [
        "PROCEDURE Apply(PROCEDURE callback())\nENDPROC\nBEGIN\n  Apply()\nEND\n",
        "PROCEDURE Apply(FUNCTION callback() INTEGER)\nENDPROC\nBEGIN\n  Apply()\nEND\n",
        "FUNCTION AddOne(INTEGER value) INTEGER\n  RETURN value + 1\nENDFUNC\nBEGIN\n  PRINTLN AddOne()\nEND\n",
    ];

    for source in sources {
        let errors = diagnostics(source);
        assert!(errors.iter().any(|e| e.contains("Not enough arguments passed")), "{errors:?}");
    }
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
    let errors =
        diagnostics("TYPE Rec\n  INTEGER v\nENDTYPE\nRec first\nRec second\nIF first = second PRINT \"equal\"\nIF first <> second PRINT \"different\"\n");
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
fn a_board_object_cannot_be_a_record_field() {
    let errors = diagnostics("TYPE Holder\n  CONFERENCE Conf\nENDTYPE\nHolder item\n");
    assert!(errors.iter().any(|e| e == "Board object UserData(30) cannot be a record field"), "{errors:?}");
}

#[test]
fn whole_arrays_of_records_cannot_be_compared() {
    let errors = diagnostics("TYPE Rec\n  INTEGER v\nENDTYPE\nRec first(2)\nRec second(2)\nIF first = second PRINT \"equal\"\n");
    assert!(errors.iter().any(|e| e == "Whole arrays of custom types cannot be compared"), "{errors:?}");
}

#[test]
fn indexed_records_can_still_be_compared() {
    let errors = diagnostics("TYPE Rec\n  INTEGER v\nENDTYPE\nRec first(2)\nRec second(2)\nIF first(1) = second(1) PRINT \"equal\"\n");
    assert!(errors.is_empty(), "{errors:?}");
}

#[test]
fn a_member_on_something_that_is_not_an_object_is_reported() {
    let errors = diagnostics("INTEGER i\nPRINTLN i.Name\n");
    assert!(errors.iter().any(|e| e == "Member not found"), "{errors:?}");
}

/// Every string and bytes member is a function. A bare one used to type-check and then reach the
/// code generator with nothing to lower, which wrote an invalid expression into the executable.
#[test]
fn a_string_member_without_a_call_is_reported() {
    for source in ["STRING s\nPRINTLN s.ToLower\n", "STRING s\nPRINTLN s.Len\n", "STRING s\ns = s.Trim\n"] {
        let errors = diagnostics(source);
        assert!(
            errors.iter().any(|error| error.starts_with("Function used as variable")),
            "{source:?} -> {errors:?}"
        );
    }
}

#[test]
fn a_called_string_member_is_still_accepted() {
    for source in [
        "STRING s\nPRINTLN s.ToLower()\n",
        "STRING s\nPRINTLN s.Len()\n",
        "STRING s\nPRINTLN s.ToLower().Trim()\n",
        "STRING s\nPRINTLN s.Find(\"a\")\n",
    ] {
        assert!(diagnostics(source).is_empty(), "{source:?}");
    }
}

/// A board object member reads as a value only where it is a property. A function or a procedure
/// written without its call reached code generation as a plain member read, which left one that
/// takes arguments with none at all.
#[test]
fn a_board_object_routine_without_a_call_is_reported() {
    for source in ["PRINTLN Terminal.BeginUpdate\n", "PRINTLN Terminal.SetFont\n"] {
        let errors = diagnostics(source);
        assert!(
            errors.iter().any(|error| error.starts_with("Function used as variable")),
            "{source:?} -> {errors:?}"
        );
    }
}

#[test]
fn a_board_object_property_is_still_read_without_a_call() {
    assert!(diagnostics("PRINTLN Terminal.Info\n").is_empty());
    assert!(diagnostics("PRINTLN Terminal.BeginUpdate()\n").is_empty());
    assert!(diagnostics("PRINTLN Terminal.SetFont(1)\n").is_empty());
}
