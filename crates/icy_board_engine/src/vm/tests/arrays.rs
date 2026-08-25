use crate::vm::tests::{compile_errors_with_runtime, run_ppl};

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

/// IN keeps working as a name, so reserving the word costs nobody anything.
#[test]
fn in_is_still_available_as_a_variable_name() {
    assert_eq!("3", run_ppl("INTEGER in\nLET in = 3\nPRINT in"));
}
