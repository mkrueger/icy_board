use crate::vm::tests::{compile_errors, run_ppl};

#[test]
fn test_a_record_field_keeps_what_was_assigned_to_it() {
    assert_eq!(
        "5Hello World",
        run_ppl(
            r#"
TYPE FooBar
   INTEGER a
   STRING b
ENDTYPE

FooBar foo

foo.a = 5
foo.b = "Hello World"

PRINT foo.a, foo.b
"#
        )
    );
}

#[test]
fn test_a_record_starts_out_with_the_empty_value_of_each_field() {
    assert_eq!(
        "[0][]",
        run_ppl(
            r#"
TYPE Rec
   INTEGER a
   STRING b
ENDTYPE
Rec r
PRINT "[", r.a, "][", r.b, "]"
"#
        )
    );
}

#[test]
fn test_a_record_field_can_be_a_vector() {
    assert_eq!(
        "0 42",
        run_ppl(
            r#"
TYPE Rec
   INTEGER Values(10)
ENDTYPE
Rec r
PRINT r.Values(3), " "
r.Values(3) = 42
PRINT r.Values(3)
"#
        )
    );
}

#[test]
fn test_an_indexed_record_field_takes_compound_assignment() {
    assert_eq!(
        "43",
        run_ppl(
            r#"
TYPE Rec
   INTEGER Values(10)
ENDTYPE
Rec r
r.Values(3) = 42
r.Values(3) += 1
PRINT r.Values(3)
"#
        )
    );
}

#[test]
fn test_a_record_field_can_be_a_matrix_or_cube() {
    assert_eq!(
        "23 123",
        run_ppl(
            r#"
TYPE Rec
   INTEGER Matrix(2, 3)
   INTEGER Cube(1, 2, 3)
ENDTYPE
Rec r
r.Matrix(2, 3) = 23
r.Cube(1, 2, 3) = 123
PRINT r.Matrix(2, 3), " ", r.Cube(1, 2, 3)
"#
        )
    );
}

#[test]
fn test_an_array_field_can_hold_records() {
    assert_eq!(
        "left/right",
        run_ppl(
            r#"
TYPE Item
   STRING Name
ENDTYPE
TYPE Rec
   Item Items(1)
ENDTYPE
Rec r
r.Items(0).Name = "left"
r.Items(1).Name = "right"
PRINT r.Items(0).Name, "/", r.Items(1).Name
"#
        )
    );
}

#[test]
fn test_an_array_field_inside_an_array_field_can_be_written() {
    assert_eq!(
        "42",
        run_ppl(
            r#"
TYPE Item
   INTEGER Values(2)
ENDTYPE
TYPE Rec
   Item Items(1)
ENDTYPE
Rec outer
outer.Items(0).Values(2) = 42
PRINT outer.Items(0).Values(2)
"#
        )
    );
}

#[test]
fn test_nested_array_fields_can_be_written_through_a_record_array_element() {
    assert_eq!(
        "Door",
        run_ppl(
            r#"
TYPE Item
   STRING Name
ENDTYPE
TYPE Rec
   Item Items(1)
ENDTYPE
Rec recs(1)
recs(1).Items(0).Name = "Door"
PRINT recs(1).Items(0).Name
"#
        )
    );
}

#[test]
fn test_record_array_field_sizes_are_fixed() {
    let statement_errors = compile_errors("TYPE Rec\n  INTEGER Values(1)\nENDTYPE\nRec r\nREDIM r.Values, 2\n");
    assert_eq!(
        vec!["Record array field 'Values' has a fixed size and cannot be redimensioned"],
        statement_errors
    );

    let member_errors = compile_errors("TYPE Rec\n  INTEGER Values(1)\nENDTYPE\nRec r\nr.Values.Redim(2)\n");
    assert_eq!(vec!["Record array field 'Values' has a fixed size and cannot be redimensioned"], member_errors);
}

#[test]
fn test_record_array_fields_answer_len() {
    assert_eq!(
        "2 4 3",
        run_ppl(
            r#"
TYPE Inner
   INTEGER Values(2, 3)
ENDTYPE
TYPE Outer
   Inner Items(1)
ENDTYPE
Outer outer
PRINT outer.Items.Len(), " ", outer.Items(0).Values.Len(1), " ", Len(outer.Items(0).Values, 0)
"#
        )
    );
}

#[test]
fn test_foreach_walks_a_scalar_record_array_field() {
    assert_eq!(
        "6",
        run_ppl(
            r#"
TYPE Rec
   INTEGER Values(2)
ENDTYPE
Rec rec
INTEGER value
INTEGER total
rec.Values(0) = 1
rec.Values(1) = 2
rec.Values(2) = 3
FOREACH value IN rec.Values
   total += value
ENDFOREACH
PRINT total
"#
        )
    );
}

#[test]
fn test_foreach_walks_a_record_valued_field() {
    assert_eq!(
        "left/right/",
        run_ppl(
            r#"
TYPE Item
   STRING Name
ENDTYPE
TYPE Rec
   Item Items(1)
ENDTYPE
Rec rec
Item item
rec.Items(0).Name = "left"
rec.Items(1).Name = "right"
FOREACH item IN rec.Items
   PRINT item.Name, "/"
ENDFOREACH
"#
        )
    );
}

#[test]
fn test_a_whole_array_field_can_be_copied_when_its_shape_matches() {
    assert_eq!(
        "7 9",
        run_ppl(
            r#"
TYPE Rec
   INTEGER Values(1)
ENDTYPE
Rec source
Rec target
source.Values(0) = 7
source.Values(1) = 9
target.Values = source.Values
PRINT target.Values(0), " ", target.Values(1)
"#
        )
    );
}

#[test]
fn test_whole_array_field_assignment_checks_its_shape() {
    let errors = compile_errors(
        "TYPE Rec\n  INTEGER Small(1)\n  INTEGER Large(2)\n  INTEGER Scalar\nENDTYPE\nRec r\nr.Small = r.Large\nr.Small = 1\nr.Scalar = r.Small\n",
    );
    assert_eq!(
        vec![
            "Record array field 'Small' expects Integer(1), got Integer(2)",
            "Record array field 'Small' requires an array value with the same shape",
            "Whole arrays cannot be used as scalar values; index an element first",
        ],
        errors
    );
}

/// The same layout rules have to hold when the field is reached through an index,
/// because that path assigns in place and would otherwise drop the fixed bounds.
#[test]
fn test_whole_array_field_assignment_through_an_index_checks_its_shape() {
    let scalar = compile_errors("TYPE Item\n  INTEGER Values(2)\nENDTYPE\nTYPE Rec\n  Item Items(1)\nENDTYPE\nRec r\nr.Items(0).Values = 5\n");
    assert_eq!(vec!["Record array field 'Values' requires an array value with the same shape"], scalar);

    let mismatch = compile_errors(
        "TYPE Item\n  INTEGER Values(2)\n  INTEGER Other(1)\nENDTYPE\nTYPE Rec\n  Item Items(1)\nENDTYPE\nRec r\nr.Items(0).Values = r.Items(0).Other\n",
    );
    assert_eq!(vec!["Record array field 'Values' expects Integer(2), got Integer(1)"], mismatch);

    let scalar_target = compile_errors(
        "TYPE Item\n  STRING Name\n  INTEGER Values(2)\nENDTYPE\nTYPE Rec\n  Item Items(1)\nENDTYPE\nRec r\nr.Items(0).Name = r.Items(0).Values\n",
    );
    assert_eq!(vec!["Whole arrays cannot be used as scalar values; index an element first"], scalar_target);
}

#[test]
fn test_a_whole_array_field_can_be_copied_through_an_index() {
    assert_eq!(
        "7 3",
        run_ppl(
            r#"
TYPE Item
   INTEGER Values(2)
ENDTYPE
TYPE Rec
   Item Items(1)
ENDTYPE
Rec r
r.Items(1).Values(2) = 7
r.Items(0).Values = r.Items(1).Values
PRINT r.Items(0).Values(2), " ", r.Items(0).Values.Len()
"#
        )
    );
}

#[test]
fn test_whole_array_fields_cannot_be_used_as_scalars() {
    let errors = compile_errors("TYPE Rec\n  INTEGER Values(1)\nENDTYPE\nRec r\nPRINT r.Values + 1\n");
    assert_eq!(vec!["Whole arrays cannot be used as scalar values; index an element first"], errors);
}

#[test]
fn test_record_array_field_index_diagnostic_names_the_rank() {
    let too_few = compile_errors("TYPE Rec\n  INTEGER Values(1, 2)\nENDTYPE\nRec r\nPRINT r.Values(1)\n");
    assert_eq!(vec!["Record array field 'Values' has rank 2, but 1 index was supplied"], too_few);

    let too_many = compile_errors("TYPE Rec\n  INTEGER Values(1)\nENDTYPE\nRec r\nPRINT r.Values(0, 1)\n");
    assert_eq!(vec!["Record array field 'Values' has rank 1, but 2 indices were supplied"], too_many);
}

#[test]
fn test_record_equality_includes_array_field_contents() {
    assert_eq!(
        "1 0 1",
        run_ppl(
            r#"
TYPE Rec
   INTEGER Values(1)
ENDTYPE
Rec first
Rec second
PRINT first = second, " "
second.Values(1) = 7
PRINT first = second, " "
first = second
PRINT first = second
"#
        )
    );
}

#[test]
fn test_a_field_takes_the_type_it_was_declared_with() {
    // The string is converted on the way in, the integer keeps its own type.
    assert_eq!(
        "42|7|",
        run_ppl(
            r#"
TYPE Rec
   INTEGER a
   STRING b
ENDTYPE
Rec r
r.a = "42"
r.b = 7
PRINT r.a, "|", r.b, "|"
"#
        )
    );
}

#[test]
fn test_two_records_of_the_same_type_do_not_share_their_fields() {
    assert_eq!(
        "1 2",
        run_ppl(
            r#"
TYPE Rec
   INTEGER a
ENDTYPE
Rec first
Rec second
first.a = 1
second.a = 2
PRINT first.a, " ", second.a
"#
        )
    );
}

#[test]
fn test_a_field_can_be_read_and_written_in_the_same_expression() {
    assert_eq!(
        "11",
        run_ppl(
            r"
TYPE Rec
   INTEGER a
ENDTYPE
Rec r
r.a = 5
r.a = r.a + 6
PRINT r.a
"
        )
    );
}

#[test]
fn test_a_field_takes_a_compound_assignment() {
    assert_eq!(
        "15",
        run_ppl(
            r"
TYPE Rec
   INTEGER a
ENDTYPE
Rec r
r.a = 5
r.a += 10
PRINT r.a
"
        )
    );
}

#[test]
fn test_a_record_survives_a_loop() {
    assert_eq!(
        "0 1 2 ",
        run_ppl(
            r#"
TYPE Rec
   INTEGER a
ENDTYPE
Rec r
INTEGER i
FOR i = 0 TO 2
    r.a = i
    PRINT r.a, " "
NEXT
"#
        )
    );
}

#[test]
fn test_a_program_can_declare_more_than_one_record() {
    assert_eq!(
        "ab",
        run_ppl(
            r#"
TYPE First
   STRING a
ENDTYPE
TYPE Second
   STRING b
ENDTYPE
First one
Second two
one.a = "a"
two.b = "b"
PRINT one.a, two.b
"#
        )
    );
}

#[test]
fn test_a_whole_record_is_copied_by_assignment() {
    // Records are values, so the copy does not write through to the original.
    assert_eq!(
        "3/9",
        run_ppl(
            r#"
TYPE Rec
   INTEGER a
ENDTYPE
Rec x
Rec y
x.a = 3
y = x
y.a = 9
PRINT x.a, "/", y.a
"#
        )
    );
}

#[test]
fn test_a_record_local_to_a_procedure_has_its_fields() {
    assert_eq!(
        "4",
        run_ppl(
            r"
TYPE Rec
   INTEGER a
ENDTYPE
Go()
PROCEDURE Go()
  Rec local
  local.a = 4
  PRINT local.a
ENDPROC
"
        )
    );
}

#[test]
fn test_a_record_local_starts_empty_on_every_call() {
    assert_eq!(
        "1 1 ",
        run_ppl(
            r#"
TYPE Rec
   INTEGER a
ENDTYPE
Go()
Go()
PROCEDURE Go()
  Rec local
  local.a = local.a + 1
  PRINT local.a, " "
ENDPROC
"#
        )
    );
}

#[test]
fn test_a_record_travels_into_a_procedure() {
    assert_eq!(
        "5",
        run_ppl(
            r"
TYPE Rec
   INTEGER a
ENDTYPE
Rec x
x.a = 5
Show(x)
PROCEDURE Show(Rec r)
  PRINT r.a
ENDPROC
"
        )
    );
}

#[test]
fn test_a_var_record_parameter_writes_back() {
    assert_eq!(
        "99",
        run_ppl(
            r"
TYPE Rec
   INTEGER a
ENDTYPE
Rec x
x.a = 5
Bump(x)
PRINT x.a
PROCEDURE Bump(VAR Rec r)
  r.a = 99
ENDPROC
"
        )
    );
}

#[test]
fn test_a_record_parameter_of_a_value_kind_does_not_write_back() {
    assert_eq!(
        "5",
        run_ppl(
            r"
TYPE Rec
   INTEGER a
ENDTYPE
Rec x
x.a = 5
Bump(x)
PRINT x.a
PROCEDURE Bump(Rec r)
  r.a = 99
ENDPROC
"
        )
    );
}

#[test]
fn test_a_record_travels_into_a_function_and_back_out() {
    assert_eq!(
        "8",
        run_ppl(
            r"
TYPE Rec
   INTEGER a
ENDTYPE
DECLARE FUNCTION Take(Rec r) INTEGER
Rec x
x.a = 8
PRINT Take(x)
FUNCTION Take(Rec r) INTEGER
  RETURN r.a
ENDFUNC
"
        )
    );
}

#[test]
fn test_a_function_can_answer_a_record() {
    assert_eq!(
        "7",
        run_ppl(
            r"
TYPE Rec
   INTEGER a
ENDTYPE
DECLARE FUNCTION Make() Rec
Rec x
x = Make()
PRINT x.a
FUNCTION Make() Rec
  Rec tmp
  tmp.a = 7
  RETURN tmp
ENDFUNC
"
        )
    );
}

#[test]
fn test_an_array_can_hold_records() {
    assert_eq!(
        "1/2",
        run_ppl(
            r#"
TYPE Rec
  INTEGER v
ENDTYPE
Rec items(2)
items[0].v = 1
items[1].v = 2
PRINT items[0].v, "/", items[1].v
"#
        )
    );
}

#[test]
fn test_record_array_elements_do_not_share_fields() {
    assert_eq!(
        "9/0",
        run_ppl(
            r#"
TYPE Rec
  INTEGER v
ENDTYPE
Rec items(2)
items[0].v = 9
PRINT items[0].v, "/", items[1].v
"#
        )
    );
}

#[test]
fn test_a_two_dimensional_array_can_hold_records() {
    assert_eq!(
        "7/8",
        run_ppl(
            r#"
TYPE Rec
  INTEGER v
ENDTYPE
Rec items(1, 1)
items[0, 1].v = 7
items[1, 0].v = 8
PRINT items[0, 1].v, "/", items[1, 0].v
"#
        )
    );
}

#[test]
fn test_parenthesis_indexing_can_reach_a_record_field() {
    assert_eq!(
        "5",
        run_ppl(
            r"
TYPE Rec
  INTEGER v
ENDTYPE
Rec items(1)
items(0).v = 5
PRINT items(0).v
"
        )
    );
}

#[test]
fn test_a_three_dimensional_array_can_hold_records() {
    assert_eq!(
        "9",
        run_ppl(
            r"
TYPE Rec
  INTEGER v
ENDTYPE
Rec items(1, 1, 1)
items[1, 0, 1].v = 9
PRINT items[1, 0, 1].v
"
        )
    );
}

#[test]
fn test_a_whole_record_can_be_assigned_to_an_array_element() {
    assert_eq!(
        "6",
        run_ppl(
            r"
TYPE Rec
  INTEGER v
ENDTYPE
Rec source
Rec items(1)
source.v = 6
items[0] = source
PRINT items[0].v
"
        )
    );
}

#[test]
fn test_a_local_record_array_starts_empty_on_every_call() {
    assert_eq!(
        "1 1 ",
        run_ppl(
            r#"
TYPE Rec
  INTEGER v
ENDTYPE
Go()
Go()
PROCEDURE Go()
  Rec items(1)
  items[0].v += 1
  PRINT items[0].v, " "
ENDPROC
"#
        )
    );
}

#[test]
fn test_a_record_variable_can_be_initialized_from_another_record() {
    assert_eq!(
        "4/9",
        run_ppl(
            r#"
TYPE Rec
    INTEGER v
ENDTYPE
Rec source
source.v = 4
Rec copy = source
copy.v = 9
PRINT source.v, "/", copy.v
"#
        )
    );
}

#[test]
fn test_a_var_parameter_can_write_back_to_a_record_array_element() {
    assert_eq!(
        "8",
        run_ppl(
            r"
TYPE Rec
    INTEGER v
ENDTYPE
Rec items(1)
items[0].v = 3
Change(items[0])
PRINT items[0].v
PROCEDURE Change(VAR Rec value)
    value.v = 8
ENDPROC
"
        )
    );
}

#[test]
fn test_records_compare_by_their_fields() {
    assert_eq!(
        "equal different",
        run_ppl(
            r#"
TYPE Rec
    INTEGER v
    STRING s
ENDTYPE
Rec first
Rec second
first.v = 1
first.s = "x"
second.v = 1
second.s = "x"
IF first = second PRINT "equal"
second.v = 2
IF first <> second PRINT " different"
"#
        )
    );
}

#[test]
fn test_nested_records_compare_by_their_fields() {
    assert_eq!(
        "equal different",
        run_ppl(
            r#"
TYPE Inner
    INTEGER v
ENDTYPE
TYPE Outer
    Inner value
ENDTYPE
Outer first
Outer second
first.value.v = 1
second.value.v = 1
IF first = second PRINT "equal"
second.value.v = 2
IF first <> second PRINT " different"
"#
        )
    );
}
