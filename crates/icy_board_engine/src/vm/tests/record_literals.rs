use crate::vm::tests::{compile_errors, compile_errors_with_runtime, run_ppl};

#[test]
fn a_named_record_literal_initializes_fields_in_any_order() {
    assert_eq!(
        "12,hello",
        run_ppl(
            r#"
TYPE Item
    INTEGER Number
    STRING Text
ENDTYPE

Item value = Item { Text = "hello", Number = 12 }
PRINT value.Number, ",", value.Text
"#,
        )
    );
}

#[test]
fn omitted_record_literal_fields_keep_their_empty_values() {
    assert_eq!(
        "7,",
        run_ppl(
            r#"
TYPE Item
    INTEGER Number
    STRING Text
ENDTYPE

Item value = Item { Number = 7 }
PRINT value.Number, ",", value.Text
"#,
        )
    );
}

#[test]
fn a_record_literal_can_be_assigned_passed_and_returned() {
    assert_eq!(
        "3,4",
        run_ppl(
            r#"
TYPE Point
    INTEGER X
    INTEGER Y
ENDTYPE

Point value
value = Make(Point { X = 1, Y = 2 })
PRINT value.X, ",", value.Y

FUNCTION Make(Point input) Point
    RETURN Point { X = input.X + 2, Y = input.Y + 2 }
ENDFUNC
"#,
        )
    );
}

#[test]
fn record_literal_fields_are_checked() {
    let duplicate = compile_errors(
        "TYPE Point\n INTEGER X\nENDTYPE\nPoint value = Point { X = 1, X = 2 }\n",
    );
    assert!(duplicate.iter().any(|error| error == "Record literal field 'X' is listed more than once"), "{duplicate:?}");

    let unknown = compile_errors(
        "TYPE Point\n INTEGER X\nENDTYPE\nPoint value = Point { Y = 1 }\n",
    );
    assert!(unknown.iter().any(|error| error == "Record type UserData(100) has no field 'Y'"), "{unknown:?}");
}

#[test]
fn a_record_literal_field_rejects_the_wrong_record_type() {
    let errors = compile_errors(
        r#"
TYPE First
    INTEGER Value
ENDTYPE
TYPE Second
    INTEGER Value
ENDTYPE
TYPE Holder
    First Value
ENDTYPE
Holder holder = Holder { Value = Second { Value = 1 } }
"#,
    );
    assert!(
        errors.iter().any(|error| error == "Record field 'Value' expects UserData(100), got UserData(101)"),
        "{errors:?}"
    );
}

#[test]
fn a_record_literal_needs_runtime_401() {
    let errors = compile_errors_with_runtime(
        "TYPE Point\n INTEGER X\nENDTYPE\nPoint value = Point { X = 1 }\n",
        400,
    );
    assert!(errors.iter().any(|error| error == "Record literals need runtime 401"), "{errors:?}");
}
