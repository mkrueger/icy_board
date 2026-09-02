use crate::vm::tests::{compile_errors, compile_errors_with_runtime, run_ppl};

/// A routine reference stores its parameter count where an array keeps its bounds,
/// so it must not be mistaken for one.
#[test]
fn a_routine_parameter_is_not_an_array() {
    let errors = compile_errors("PROCEDURE Apply(FUNCTION callback(INTEGER value) INTEGER)\n    PRINT callback + 1\nENDPROC\n");
    assert_eq!(vec!["Function used as variable (callback)"], errors);
}

#[test]
fn a_function_can_be_passed_and_called() {
    assert_eq!(
        "42",
        run_ppl(
            r"
PrintWith(Twice)

PROCEDURE PrintWith(FUNCTION callback(INTEGER value) INTEGER)
    PRINT callback(21)
ENDPROC

FUNCTION Twice(INTEGER value) INTEGER
    RETURN value * 2
ENDFUNC
",
        )
    );
}

#[test]
fn a_procedure_parameter_keeps_var_semantics() {
    assert_eq!(
        "5",
        run_ppl(
            r"
Apply(Increment)

PROCEDURE Apply(PROCEDURE callback(VAR INTEGER value))
    INTEGER number = 4
    callback(number)
    PRINT number
ENDPROC

PROCEDURE Increment(VAR INTEGER value)
    value = value + 1
ENDPROC
",
        )
    );
}

#[test]
fn by_value_parameters_beyond_the_var_mask_do_not_overflow() {
    assert_eq!(
        "42",
        run_ppl(
            r"
INTEGER value = 1
Change(value, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16)
PRINT value

PROCEDURE Change(VAR INTEGER target, INTEGER p1, INTEGER p2, INTEGER p3, INTEGER p4, INTEGER p5, INTEGER p6, INTEGER p7, INTEGER p8, INTEGER p9, INTEGER p10, INTEGER p11, INTEGER p12, INTEGER p13, INTEGER p14, INTEGER p15, INTEGER p16)
    target = 42
ENDPROC
",
        )
    );
}

#[test]
fn a_routine_parameter_can_be_forwarded() {
    assert_eq!(
        "7",
        run_ppl(
            r"
Relay(PrintValue)

PROCEDURE Relay(PROCEDURE callback(INTEGER value))
    Invoke(callback)
ENDPROC

PROCEDURE Invoke(PROCEDURE callback(INTEGER value))
    callback(7)
ENDPROC

PROCEDURE PrintValue(INTEGER value)
    PRINT value
ENDPROC
",
        )
    );
}

#[test]
fn a_routine_argument_must_have_the_declared_signature() {
    let errors = compile_errors(
        r"
Apply(Wrong)

PROCEDURE Apply(PROCEDURE callback(INTEGER value))
ENDPROC

PROCEDURE Wrong(STRING value)
ENDPROC
",
    );
    assert!(errors.iter().any(|error| error.contains("parameters not match")), "{errors:?}");
}

#[test]
fn a_procedure_call_reports_excess_arguments() {
    let errors = compile_errors(
        r"
Show(1, 2)

PROCEDURE Show(INTEGER value)
ENDPROC
",
    );
    assert!(errors.iter().any(|error| error.contains("Too many arguments passed (Show:2:1)")), "{errors:?}");
}

#[test]
fn a_bare_routine_name_is_still_not_a_general_value() {
    let errors = compile_errors(
        r"
PRINT Work

PROCEDURE Work()
ENDPROC
",
    );
    assert!(errors.iter().any(|error| error == "Function used as variable (Work)"), "{errors:?}");
}

#[test]
fn a_function_name_is_not_its_return_value_outside_that_function() {
    let errors = compile_errors(
        r"
PRINT Work

FUNCTION Work() INTEGER
    Work = 1
ENDFUNC
EXIT
",
    );
    assert!(errors.iter().any(|error| error == "Function used as variable (Work)"), "{errors:?}");
}

#[test]
fn passing_a_routine_needs_runtime_400() {
    let errors = compile_errors_with_runtime(
        r"
Apply(Work)

PROCEDURE Apply(PROCEDURE callback())
ENDPROC

PROCEDURE Work()
ENDPROC
",
        340,
    );
    assert!(
        errors.iter().any(|error| error == "Passing a FUNCTION/PROCEDURE needs runtime 400"),
        "{errors:?}"
    );
}
