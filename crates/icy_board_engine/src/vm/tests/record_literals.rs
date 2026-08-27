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
    let duplicate = compile_errors("TYPE Point\n INTEGER X\nENDTYPE\nPoint value = Point { X = 1, X = 2 }\n");
    assert!(
        duplicate.iter().any(|error| error == "Record literal field 'X' is listed more than once"),
        "{duplicate:?}"
    );

    let unknown = compile_errors("TYPE Point\n INTEGER X\nENDTYPE\nPoint value = Point { Y = 1 }\n");
    assert!(unknown.iter().any(|error| error == "Record type UserData(100) has no field 'Y'"), "{unknown:?}");
}

#[test]
fn a_record_literal_field_rejects_the_wrong_record_type() {
    let errors = compile_errors(
        r"
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
",
    );
    assert!(
        errors
            .iter()
            .any(|error| error == "Record field 'Value' expects UserData(100), got UserData(101)"),
        "{errors:?}"
    );
}

#[test]
fn record_literal_array_fields_require_the_declared_shape() {
    let errors = compile_errors("TYPE Rec\n  INTEGER Small(1)\n  INTEGER Large(2)\nENDTYPE\nRec r\nr = Rec { Small = r.Large }\n");
    assert_eq!(vec!["Record array field 'Small' expects Integer(1), got Integer(2)"], errors);

    let scalar = compile_errors("TYPE Rec\n  INTEGER Values(1)\nENDTYPE\nRec r\nr = Rec { Values = 5 }\n");
    assert_eq!(vec!["Record array field 'Values' requires an array value with the same shape"], scalar);
}

#[test]
fn record_literal_scalar_fields_reject_whole_arrays() {
    let errors = compile_errors("TYPE Rec\n  INTEGER Scalar\n  INTEGER Values(1)\nENDTYPE\nRec r\nr = Rec { Scalar = r.Values }\n");
    assert_eq!(vec!["Whole arrays cannot be used as scalar values; index an element first"], errors);
}

#[test]
fn record_literal_array_fields_accept_the_same_shape() {
    assert_eq!(
        "7 1",
        run_ppl(
            "TYPE Rec\n  INTEGER Values(1)\nENDTYPE\nRec source\nRec target\nsource.Values(1) = 7\ntarget = Rec { Values = source.Values }\nPRINT target.Values(1), \" \", target.Values.Len()\n"
        )
    );
}

#[test]
fn a_record_literal_needs_runtime_400() {
    let errors = compile_errors_with_runtime("TYPE Point\n INTEGER X\nENDTYPE\nPoint value = Point { X = 1 }\n", 340);
    assert!(errors.iter().any(|error| error == "Record literals need runtime 400"), "{errors:?}");
}
