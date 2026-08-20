use crate::vm::tests::run_ppl;

#[test]
fn test_a_function_can_be_called_before_the_file_defines_it() {
    assert_eq!(
        "7",
        run_ppl(
            r"
INTEGER v
v = Make()
PRINT v
FUNCTION Make() INTEGER
  RETURN 7
ENDFUNC
"
        )
    );
}

#[test]
fn test_a_procedure_can_be_called_before_the_file_defines_it() {
    assert_eq!(
        "ran",
        run_ppl(
            r#"
Go()
PROCEDURE Go()
  PRINT "ran"
ENDPROC
"#
        )
    );
}

#[test]
fn test_a_function_used_in_an_expression_before_its_definition() {
    assert_eq!(
        "9",
        run_ppl(
            r"
PRINT Twice(3) + 3
FUNCTION Twice(INTEGER n) INTEGER
  RETURN n * 2
ENDFUNC
"
        )
    );
}

#[test]
fn test_a_forward_declaration_is_still_accepted() {
    assert_eq!(
        "7",
        run_ppl(
            r"
DECLARE FUNCTION Make() INTEGER
INTEGER v
v = Make()
PRINT v
FUNCTION Make() INTEGER
  RETURN 7
ENDFUNC
"
        )
    );
}

#[test]
fn test_routines_can_call_each_other_in_either_order() {
    assert_eq!(
        "10",
        run_ppl(
            r"
PRINT Outer(5)
FUNCTION Outer(INTEGER n) INTEGER
  RETURN Inner(n)
ENDFUNC
FUNCTION Inner(INTEGER n) INTEGER
  RETURN n * 2
ENDFUNC
"
        )
    );
}

#[test]
fn repeated_nested_function_calls_release_the_call_stack() {
    assert_eq!(
        "200",
        run_ppl(
            r"
INTEGER count = 0
INTEGER i = 0
WHILE i < 200 DO
    IF Outer(i) count += 1
    i += 1
ENDWHILE
PRINT count
FUNCTION Outer(INTEGER n) BOOLEAN
    RETURN Inner(n)
ENDFUNC
FUNCTION Inner(INTEGER n) BOOLEAN
    RETURN n >= 0
ENDFUNC
"
        )
    );
}

#[test]
fn routine_loops_resolve_global_constants_before_folding() {
    assert_eq!(
        "20",
        run_ppl(
            r"
CONST INTEGER LIMIT = 20
INTEGER result
Count(result)
PRINT result
PROCEDURE Count(VAR INTEGER value)
    INTEGER i = 0
    value = 0
    WHILE i < LIMIT DO
        value += 1
        i += 1
    ENDWHILE
ENDPROC
"
        )
    );
}

#[test]
fn test_a_function_defined_before_its_use_still_works() {
    assert_eq!(
        "4",
        run_ppl(
            r"
DECLARE FUNCTION Half(INTEGER n) INTEGER
PRINT Half(8)
FUNCTION Half(INTEGER n) INTEGER
  RETURN n / 2
ENDFUNC
"
        )
    );
}
