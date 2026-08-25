use crate::vm::tests::{compile_errors, compile_errors_with_runtime, run_ppl};

/// An array declared with an upper bound of 10 holds 11 elements, index 0 through 10.
#[test]
fn element_count_counts_every_slot_of_a_vector() {
    assert_eq!("11", run_ppl("INTEGER a(10)\nPRINT ElementCount(a)"));
}

#[test]
fn element_count_multiplies_the_dimensions_out() {
    assert_eq!(
        "12 24",
        run_ppl("INTEGER a(2, 3)\nINTEGER b(1, 2, 3)\nPRINT ElementCount(a), \" \", ElementCount(b)")
    );
}

/// A value that is not an array is one element, so a caller does not have to ask first.
#[test]
fn a_plain_value_counts_as_one_element() {
    assert_eq!("1", run_ppl("INTEGER a\nPRINT ElementCount(a)"));
}

#[test]
fn element_at_walks_a_vector_in_order() {
    assert_eq!(
        "0 1 2 3",
        run_ppl(
            r#"
INTEGER a(3)
INTEGER i
FOR i = 0 TO 3
  LET a(i) = i
NEXT
PRINT ElementAt(a, 0), " ", ElementAt(a, 1), " ", ElementAt(a, 2), " ", ElementAt(a, 3)
"#
        )
    );
}

/// Row-major: the last index moves fastest.
#[test]
fn element_at_walks_a_matrix_row_by_row() {
    assert_eq!(
        "11 12 21 22",
        run_ppl(
            r#"
INTEGER a(1, 1)
LET a(0, 0) = 11
LET a(0, 1) = 12
LET a(1, 0) = 21
LET a(1, 1) = 22
PRINT ElementAt(a, 0), " ", ElementAt(a, 1), " ", ElementAt(a, 2), " ", ElementAt(a, 3)
"#
        )
    );
}

#[test]
fn an_index_no_element_has_answers_empty() {
    assert_eq!("0 0", run_ppl("INTEGER a(1)\nPRINT ElementAt(a, 99), \" \", ElementAt(a, -1)"));
}

#[test]
fn foreach_walks_a_vector() {
    assert_eq!(
        "0 10 20 30 ",
        run_ppl(
            r#"
INTEGER a(3)
INTEGER i
FOR i = 0 TO 3
  LET a(i) = i * 10
NEXT
INTEGER v
FOREACH v IN a
  PRINT v, " "
ENDFOREACH
"#
        )
    );
}

/// The whole point: a matrix walks the same way a vector does, no rank in sight.
#[test]
fn foreach_walks_a_matrix_flat() {
    assert_eq!(
        "11 12 21 22 ",
        run_ppl(
            r#"
INTEGER a(1, 1)
LET a(0, 0) = 11
LET a(0, 1) = 12
LET a(1, 0) = 21
LET a(1, 1) = 22
INTEGER v
FOREACH v IN a
  PRINT v, " "
ENDFOREACH
"#
        )
    );
}

#[test]
fn foreach_walks_a_cube_flat() {
    assert_eq!(
        "24",
        run_ppl("STRING a(1, 2, 3)\nSTRING v\nINTEGER n\nFOREACH v IN a\n  LET n = n + 1\nENDFOREACH\nPRINT n")
    );
}

/// A loop that never runs is a loop that never runs; the body is skipped, not entered once.
#[test]
fn breaking_out_of_a_foreach_stops_it() {
    assert_eq!(
        "0 1 ",
        run_ppl(
            r#"
INTEGER a(9)
INTEGER i
FOR i = 0 TO 9
  LET a(i) = i
NEXT
INTEGER v
FOREACH v IN a
  IF (v > 1) BREAK
  PRINT v, " "
ENDFOREACH
"#
        )
    );
}

#[test]
fn continue_skips_to_the_next_element() {
    assert_eq!(
        "0 2 4 ",
        run_ppl(
            r#"
INTEGER a(4)
INTEGER i
FOR i = 0 TO 4
  LET a(i) = i
NEXT
INTEGER v
FOREACH v IN a
  IF (v % 2 <> 0) CONTINUE
  PRINT v, " "
ENDFOREACH
"#
        )
    );
}

#[test]
fn a_foreach_can_sit_inside_another_one() {
    assert_eq!(
        "1a 1b 2a 2b ",
        run_ppl(
            r#"
STRING outer(1)
STRING inner(1)
LET outer(0) = "1"
LET outer(1) = "2"
LET inner(0) = "a"
LET inner(1) = "b"
STRING o
STRING i
FOREACH o IN outer
  FOREACH i IN inner
    PRINT o, i, " "
  ENDFOREACH
ENDFOREACH
"#
        )
    );
}

/// The loop variable is a copy, so writing it leaves the array alone.
#[test]
fn writing_the_loop_variable_leaves_the_array_alone() {
    assert_eq!(
        "7 7 0 0",
        run_ppl(
            r#"
INTEGER a(1)
INTEGER v
FOREACH v IN a
  LET v = 7
  PRINT v, " "
ENDFOREACH
PRINT a(0), " ", a(1)
"#
        )
    );
}

/// NEXT closes a FOREACH the way ENDFOR closes a FOR.
#[test]
fn next_also_closes_a_foreach() {
    assert_eq!("0 0 ", run_ppl("INTEGER a(1)\nINTEGER v\nFOREACH v IN a\n  PRINT v, \" \"\nNEXT"));
}

#[test]
fn foreach_needs_runtime_400() {
    let errors = compile_errors_with_runtime("INTEGER a(1)\nINTEGER v\nFOREACH v IN a\nENDFOREACH", 340);

    assert!(!errors.is_empty(), "FOREACH should not compile for an older runtime");
}

/// The hidden counter the loop is built from has to land in the local scope.
#[test]
fn foreach_works_inside_a_procedure() {
    assert_eq!(
        "1 2 3 ",
        run_ppl(
            r#"
DECLARE PROCEDURE Walk()
Walk()
PROCEDURE Walk()
  STRING a(2)
  LET a(0) = "1"
  LET a(1) = "2"
  LET a(2) = "3"
  STRING v
  FOREACH v IN a
    PRINT v, " "
  ENDFOREACH
ENDPROC
"#
        )
    );
}

/// A value that is not an array is one element, so the body runs once rather than
/// not at all. Nothing has to ask what it was handed.
#[test]
fn foreach_over_a_plain_value_runs_once() {
    assert_eq!("42 ", run_ppl("INTEGER a\nLET a = 42\nINTEGER v\nFOREACH v IN a\n  PRINT v, \" \"\nENDFOREACH"));
}

/// IN keeps working as a name, so reserving the word costs nobody anything.
#[test]
fn in_is_still_available_as_a_variable_name() {
    assert_eq!("3", run_ppl("INTEGER in\nLET in = 3\nPRINT in"));
}

/// Every built-in array function may also be written as a member of the array.
#[test]
fn an_array_answers_len_as_a_member() {
    assert_eq!(
        "10 2 3",
        run_ppl("INTEGER a(10)\nINTEGER b(2, 3)\nPRINT a.Len(), \" \", b.Len(0), \" \", b.Len(1)")
    );
}

#[test]
fn an_array_answers_the_element_functions_as_members() {
    assert_eq!(
        "12 42",
        run_ppl("INTEGER a(2, 3)\nLET a(0, 1) = 42\nPRINT a.ElementCount(), \" \", a.ElementAt(1)")
    );
}

/// The member is the same call as the function, so both may name the same array.
#[test]
fn a_member_and_a_function_agree() {
    assert_eq!("1", run_ppl("INTEGER a(4, 5)\nPRINT a.Len(1) = Len(a, 1)"));
}

#[test]
fn a_value_that_is_not_an_array_has_no_members() {
    let errors = compile_errors("INTEGER a\nPRINT a.Len()");

    assert!(!errors.is_empty(), "a plain value should not answer array members");
}

#[test]
fn an_unknown_array_member_is_rejected() {
    let errors = compile_errors("INTEGER a(1)\nPRINT a.Sort()");

    assert!(!errors.is_empty(), "an array should not answer a member it does not have");
}

#[test]
fn an_array_member_checks_its_argument_count() {
    let errors = compile_errors("INTEGER a(1)\nPRINT a.Len(0, 1)");

    assert!(!errors.is_empty(), "Len takes at most one argument");
}

/// REDIM is a statement, so its member is one too.
#[test]
fn an_array_can_be_redimmed_through_its_member() {
    assert_eq!("1 20", run_ppl("INTEGER a(1)\nPRINT a.Len(), \" \"\na.Redim(20)\nPRINT a.Len()"));
}

#[test]
fn redim_as_a_member_takes_one_bound_per_dimension() {
    assert_eq!("2 3", run_ppl("INTEGER a(1)\na.Redim(2, 3)\nPRINT a.Len(0), \" \", a.Len(1)"));
}

/// The member is the same statement, so it agrees with the written out form.
#[test]
fn a_redim_member_and_the_statement_agree() {
    assert_eq!(
        "7 7",
        run_ppl("INTEGER a(1)\nINTEGER b(1)\na.Redim(7)\nREDIM b, 7\nPRINT a.Len(), \" \", b.Len()")
    );
}
