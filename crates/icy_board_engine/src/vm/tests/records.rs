use crate::vm::tests::run_ppl;

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
            r#"
TYPE Rec
   INTEGER a
ENDTYPE
Rec r
r.a = 5
r.a = r.a + 6
PRINT r.a
"#
        )
    );
}

#[test]
fn test_a_field_takes_a_compound_assignment() {
    assert_eq!(
        "15",
        run_ppl(
            r#"
TYPE Rec
   INTEGER a
ENDTYPE
Rec r
r.a = 5
r.a += 10
PRINT r.a
"#
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
            r#"
TYPE Rec
   INTEGER a
ENDTYPE
Go()
PROCEDURE Go()
  Rec local
  local.a = 4
  PRINT local.a
ENDPROC
"#
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
            r#"
TYPE Rec
   INTEGER a
ENDTYPE
Rec x
x.a = 5
Show(x)
PROCEDURE Show(Rec r)
  PRINT r.a
ENDPROC
"#
        )
    );
}

#[test]
fn test_a_var_record_parameter_writes_back() {
    assert_eq!(
        "99",
        run_ppl(
            r#"
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
"#
        )
    );
}

#[test]
fn test_a_record_parameter_of_a_value_kind_does_not_write_back() {
    assert_eq!(
        "5",
        run_ppl(
            r#"
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
"#
        )
    );
}

#[test]
fn test_a_record_travels_into_a_function_and_back_out() {
    assert_eq!(
        "8",
        run_ppl(
            r#"
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
"#
        )
    );
}

#[test]
fn test_a_function_can_answer_a_record() {
    assert_eq!(
        "7",
        run_ppl(
            r#"
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
"#
        )
    );
}
