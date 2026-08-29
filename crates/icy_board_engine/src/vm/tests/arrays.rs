use crate::vm::tests::{compile, compile_errors, compile_errors_with_runtime, run_ppl};

#[test]
fn ppl_400_dynamic_arrays_use_square_bracket_rank_syntax() {
    assert_eq!(
        "0 0 0 3 7",
        run_ppl(
            r#"
;$LANGVERSION 400
INTEGER vector[]
INTEGER matrix[,]
INTEGER cube[,,]
PRINT vector.Len(), " ", matrix.Len(), " ", cube.Len(), " "
vector.Redim(2)
vector[2] = 7
PRINT vector.Len(), " ", vector[2]
"#
        )
    );
}

#[test]
fn ppl_400_accepts_legacy_array_declarations_during_migration() {
    assert!(compile_errors(";$LANGVERSION 400\nINTEGER values(10)").is_empty());
    assert!(compile_errors(";$LANGVERSION 340\nINTEGER values(10)").is_empty());
}

#[test]
fn ppl_400_dynamic_array_declarations_accept_initializers() {
    assert_eq!(
        "3 a|b|",
        run_ppl(
            r#";$LANGVERSION 400
STRING parts[] = "a,b,".Split(",")
PRINT parts.Len(), " ", STRING.Join(parts, "|")
"#
        )
    );

    assert_eq!(
        "2 first",
        run_ppl(
            r#";$LANGVERSION 400
STRING lines[] = { "first", "second" }
PRINT lines.Len(), " ", lines[0]
"#
        )
    );
}

#[test]
fn ppl_400_functions_return_dynamic_arrays() {
    assert_eq!(
        "4 20",
        run_ppl(
            r#";$LANGVERSION 400
DECLARE FUNCTION MakeValues() INTEGER[]
INTEGER values[]
values = MakeValues()
PRINT values.Len(), " ", values[2]

FUNCTION MakeValues() INTEGER[]
  INTEGER result[3]
  result[2] = 20
  RETURN result
ENDFUNC
"#
        )
    );
}

/// An array declared with an upper bound of 10 holds 11 elements, index 0 through 10,
/// and `FOREACH` visits every one of them.
#[test]
fn a_walk_visits_every_slot_of_a_vector() {
    assert_eq!(
        "11",
        run_ppl("INTEGER a(10)\nINTEGER v\nINTEGER n\nFOREACH v IN a\n  LET n = n + 1\nENDFOREACH\nPRINT n")
    );
}

#[test]
fn a_walk_multiplies_the_dimensions_out() {
    assert_eq!(
        "12 24",
        run_ppl(
            r#"
INTEGER a(2, 3)
INTEGER b(1, 2, 3)
INTEGER v
INTEGER n
INTEGER m
FOREACH v IN a
  LET n = n + 1
ENDFOREACH
FOREACH v IN b
  LET m = m + 1
ENDFOREACH
PRINT n, " ", m
"#
        )
    );
}

/// Row-major: the last index moves fastest.
#[test]
fn a_walk_crosses_a_matrix_row_by_row() {
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

/// The flat walk is the compiler's own; source says which index it means per dimension.
#[test]
fn indexing_takes_one_index_per_dimension() {
    assert!(
        !compile_errors("INTEGER a(1, 1)\nPRINT a[2]").is_empty(),
        "one index into a matrix should not compile"
    );
    assert_eq!("12", run_ppl("INTEGER a(1, 1)\nLET a(0, 1) = 12\nPRINT a[0, 1]"));
}

#[test]
fn foreach_compiles_to_its_statement_bytecode() {
    let executable = compile("INTEGER values(1)\nINTEGER value\nFOREACH value IN values\n  PRINT value\nENDFOREACH");
    assert!(executable.script_buffer.contains(&(crate::executable::OpCode::ForEach as i16)));
    assert!(executable.script_buffer.contains(&(crate::executable::OpCode::NextForEach as i16)));
    assert!(!executable.script_buffer.contains(&-306));
    assert!(!executable.script_buffer.contains(&-307));
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

/// How many elements there are is settled when the loop starts, so resizing the array
/// inside it changes neither how many steps the walk takes nor where it stops.
/// `Redim` clears what the array held, which is why only the count is looked at here.
#[test]
fn resizing_the_array_does_not_change_how_far_the_walk_goes() {
    assert_eq!(
        "2 4",
        run_ppl(
            r#"
INTEGER grown(1)
INTEGER shrunk(3)
INTEGER v
INTEGER steps
FOREACH v IN grown
  LET steps = steps + 1
  grown.Redim(9)
ENDFOREACH
PRINT steps, " "
LET steps = 0
FOREACH v IN shrunk
  LET steps = steps + 1
  shrunk.Redim(1)
ENDFOREACH
PRINT steps
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

#[test]
fn returning_from_foreach_cleans_up_its_iterator() {
    assert_eq!(
        "7 7",
        run_ppl(
            r#"
DECLARE FUNCTION First() INTEGER
PRINT First(), " ", First()

FUNCTION First() INTEGER
    INTEGER values(1)
    INTEGER value
    values(0) = 7
    FOREACH value IN values
        RETURN value
    ENDFOREACH
    RETURN 0
ENDFUNC
"#
        )
    );
}

#[test]
fn goto_out_of_foreach_cleans_up_its_iterator() {
    assert_eq!(
        "2",
        run_ppl(
            r#"
INTEGER values(1)
INTEGER value
INTEGER count
FOREACH value IN values
    GOTO done
ENDFOREACH
:done
FOREACH value IN values
    count = count + 1
ENDFOREACH
PRINT count
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
        "11 3 4",
        run_ppl("INTEGER a(10)\nINTEGER b(2, 3)\nPRINT a.Len(), \" \", b.Len(0), \" \", b.Len(1)")
    );
}

#[test]
fn array_len_reports_element_counts() {
    assert_eq!(
        "11 12 3 4 0",
        run_ppl(
            ";$LANGVERSION 400\nINTEGER vector[10]\nINTEGER matrix[2, 3]\nINTEGER empty[]\nPRINT vector.Len(), \" \", matrix.Len(), \" \", matrix.Len(0), \" \", matrix.Len(1), \" \", empty.Len()"
        )
    );
}

#[test]
fn arrays_expose_len_but_not_count() {
    assert!(!compile_errors(";$LANGVERSION 400\nINTEGER values[10]\nPRINT values.Count()").is_empty());
}

#[test]
fn array_initializer_len_is_its_element_count() {
    assert_eq!(
        "3 30",
        run_ppl(";$LANGVERSION 400\nINTEGER values = { 10, 20, 30 }\nPRINT values.Len(), \" \", values[2]")
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
    assert_eq!("2 21", run_ppl("INTEGER a(1)\nPRINT a.Len(), \" \"\na.Redim(20)\nPRINT a.Len()"));
}

#[test]
fn redim_as_a_member_takes_one_bound_per_dimension() {
    assert_eq!("3 4", run_ppl("INTEGER a(1)\na.Redim(2, 3)\nPRINT a.Len(0), \" \", a.Len(1)"));
}

/// The member is the same statement, so it agrees with the written out form.
#[test]
fn a_redim_member_and_the_statement_agree() {
    assert_eq!(
        "8 8",
        run_ppl("INTEGER a(1)\nINTEGER b(1)\na.Redim(7)\nREDIM b, 7\nPRINT a.Len(), \" \", b.Len()")
    );
}

/// PCBoard wanted one subscript per dimension everywhere a variable was read, so a
/// bare array is not a value here either, whatever its rank.
#[test]
fn a_bare_array_is_not_a_value() {
    for source in [
        "INTEGER a(5)\nPRINT a + 1",
        "INTEGER a(5)\nPRINT a",
        "INTEGER a(5)\nPRINT (a) + 1",
        "INTEGER a(5)\nPRINT -a",
        "INTEGER a(1), b(1)\nPRINT b(a)",
        "INTEGER a(5)\nIF (a) PRINT a(0)",
        "INTEGER a(5)\nWHILE (a) PRINT a(0)",
        "INTEGER a(1), i\nFOR i = a TO 1\nNEXT",
        "INTEGER a(1), i\nFOR i = 0 TO a\nNEXT",
        "INTEGER a(1), i\nFOR i = 0 TO 1 STEP a\nNEXT",
        "INTEGER a(1)\nSELECT CASE a\nCASE 0\nENDSELECT",
        "INTEGER a(1), x\nSELECT CASE x\nCASE a\nENDSELECT",
        "INTEGER a(5)\nINTEGER x\nx = a",
        "INTEGER a(5)\nPRINT Upper(a)",
        "INTEGER a(5)\nPRINT Len(a)",
        "INTEGER a(1)\nPUSH a",
        "INTEGER a(1)\nPOP a",
        "STRING a(1)\nINPUT \"\", a",
        "STRING a(1)\nFGET 1, a",
        "INTEGER a(1), b(1)\nVARSEG a, b",
        "DECLARE PROCEDURE P(INTEGER x)\nINTEGER a(1)\nP(a)\nPROCEDURE P(INTEGER x)\nENDPROC",
        "DECLARE FUNCTION F(INTEGER x) INTEGER\nINTEGER a(1), x\nx = F(a)\nFUNCTION F(INTEGER x) INTEGER\nF = x\nENDFUNC",
        "DECLARE FUNCTION F() INTEGER\nINTEGER a(1)\nPRINT F()\nFUNCTION F() INTEGER\nF = a\nENDFUNC",
        "INTEGER a(1)\nPRINT F()\nFUNCTION F() INTEGER\nRETURN a\nENDFUNC",
        "INTEGER a(2, 2)\nPRINT a",
        "INTEGER a(1, 1, 1)\nPRINT a",
    ] {
        let errors = compile_errors(source);
        assert!(
            errors.iter().any(|error| error.starts_with("Not enough arguments passed (a:0:")),
            "{source}: {errors:?}"
        );
    }
}

/// The two original PPLC constructs that take a whole array still do, as do the
/// new array members and `FOREACH`.
#[test]
fn the_array_builtins_still_see_the_whole_array() {
    assert_eq!("6 3", run_ppl("INTEGER a(5)\nINTEGER b(2, 3)\nPRINT a.Len(), \" \", Len(b, 0)"));
    assert_eq!(
        "6",
        run_ppl("INTEGER a(5)\nINTEGER v\nINTEGER n\nFOREACH v IN a\n  n = n + 1\nENDFOREACH\nPRINT n")
    );
    assert!(compile_errors("INTEGER a(5)\nREDIM a, 9\nPRINT a.Len()").is_empty());
    assert!(compile_errors("INTEGER a(5)\nINTEGER idx(5)\nSORT a, idx").is_empty());
}

/// Our compiler refuses to emit a bare array read, but a PPE built by another tool
/// can still hold one, and PCBoard answered it with the first element.
#[test]
fn a_bare_array_value_decays_to_its_first_element() {
    use crate::executable::{GenericVariableData, VariableType, VariableValue};

    let mut array = VariableValue {
        vtype: VariableType::Integer,
        generic_data: GenericVariableData::create_array(VariableValue::new_int(0), 1, 2, 0, 0).unwrap(),
        ..Default::default()
    };
    array.set_array_value(0, 0, 0, VariableValue::new_int(7)).unwrap();

    assert_eq!(7, crate::vm::decay_array(array).as_int());
}
