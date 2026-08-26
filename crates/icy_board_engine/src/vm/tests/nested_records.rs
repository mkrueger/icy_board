use crate::vm::tests::run_ppl;

#[test]
fn a_member_call_can_stand_on_its_own_as_a_statement() {
    assert_eq!(
        "ok",
        run_ppl(
            r#"
CONFERENCE conf = Board.GetConference(0)
conf.HasAccess()
PRINT "ok"
"#
        )
    );
}

#[test]
fn test_a_field_of_a_field_keeps_what_was_assigned_to_it() {
    assert_eq!(
        "5",
        run_ppl(
            r"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  Inner i
ENDTYPE
Outer o
o.i.v = 5
PRINT o.i.v
"
        )
    );
}

#[test]
fn test_a_nested_record_starts_out_empty() {
    assert_eq!(
        "[0][]",
        run_ppl(
            r#"
TYPE Inner
  INTEGER v
  STRING s
ENDTYPE
TYPE Outer
  Inner i
ENDTYPE
Outer o
PRINT "[", o.i.v, "][", o.i.s, "]"
"#
        )
    );
}

#[test]
fn test_three_levels_deep() {
    assert_eq!(
        "7",
        run_ppl(
            r"
TYPE Level1
  INTEGER v
ENDTYPE
TYPE Level2
  Level1 one
ENDTYPE
TYPE Level3
  Level2 two
ENDTYPE
Level3 deep
deep.two.one.v = 7
PRINT deep.two.one.v
"
        )
    );
}

#[test]
fn test_a_variable_may_take_the_name_of_its_type() {
    // Names are compared without regard to case, so `C c` leaves `c` reading like the
    // type it was declared from. A member cannot start a declaration, so it is the
    // variable that is meant.
    let source = r"
TYPE C
  INTEGER v
ENDTYPE
C c
c.v = 1
PRINTLN c.v
";
    assert!(crate::vm::tests::compile_errors(source).is_empty());
    assert_eq!(crate::vm::tests::run_ppl(source), "1\n");
}

#[test]
fn test_what_a_board_object_answers_can_be_asked_again() {
    let output = crate::vm::tests::run_ppl_on(
        r"
CONFERENCE conf = Board.GetConference(0)
PRINT conf.Areas[0].Name
",
        |board| {
            board.conferences.clear();
            board.conferences.push(crate::icy_board::conferences::Conference {
                name: "Main".to_string(),
                areas: Some(std::sync::Arc::new(crate::icy_board::message_area::AreaList::new(vec![
                    crate::icy_board::message_area::MessageArea {
                        name: "General".to_string(),
                        ..Default::default()
                    },
                ]))),
                ..Default::default()
            });
        },
    );
    assert_eq!("General", output);
}

#[test]
fn test_the_outer_fields_stay_beside_the_nested_one() {
    assert_eq!(
        "1/2/x",
        run_ppl(
            r#"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  INTEGER before
  Inner i
  STRING after
ENDTYPE
Outer o
o.before = 1
o.i.v = 2
o.after = "x"
PRINT o.before, "/", o.i.v, "/", o.after
"#
        )
    );
}

#[test]
fn test_a_nested_field_takes_a_compound_assignment() {
    assert_eq!(
        "12",
        run_ppl(
            r"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  Inner i
ENDTYPE
Outer o
o.i.v = 2
o.i.v += 10
PRINT o.i.v
"
        )
    );
}

#[test]
fn test_a_whole_nested_record_is_copied() {
    assert_eq!(
        "3/9",
        run_ppl(
            r#"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  Inner i
ENDTYPE
Outer a
Outer b
a.i.v = 3
b = a
b.i.v = 9
PRINT a.i.v, "/", b.i.v
"#
        )
    );
}

#[test]
fn test_a_nested_record_survives_a_routine() {
    assert_eq!(
        "4",
        run_ppl(
            r"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  Inner i
ENDTYPE
Go()
PROCEDURE Go()
  Outer local
  local.i.v = 4
  PRINT local.i.v
ENDPROC
"
        )
    );
}

#[test]
fn test_an_inner_record_can_be_assigned_on_its_own() {
    assert_eq!(
        "6",
        run_ppl(
            r"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  Inner i
ENDTYPE
Outer o
Inner free
free.v = 6
o.i = free
PRINT o.i.v
"
        )
    );
}

#[test]
fn test_a_nested_var_parameter_writes_back() {
    assert_eq!(
        "11",
        run_ppl(
            r"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  Inner value
ENDTYPE
Outer wrapper
wrapper.value.v = 3
Change(wrapper)
PRINT wrapper.value.v
PROCEDURE Change(VAR Outer item)
  item.value.v = 11
ENDPROC
"
        )
    );
}

#[test]
fn test_a_nested_value_parameter_does_not_write_back() {
    assert_eq!(
        "3",
        run_ppl(
            r"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  Inner value
ENDTYPE
Outer wrapper
wrapper.value.v = 3
Change(wrapper)
PRINT wrapper.value.v
PROCEDURE Change(Outer item)
  item.value.v = 11
ENDPROC
"
        )
    );
}

#[test]
fn test_a_function_can_answer_a_nested_record() {
    assert_eq!(
        "13",
        run_ppl(
            r"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  Inner value
ENDTYPE
Outer wrapper
wrapper = Make()
PRINT wrapper.value.v
FUNCTION Make() Outer
  Outer made
  made.value.v = 13
  RETURN made
ENDFUNC
"
        )
    );
}

#[test]
fn test_assigning_an_inner_record_copies_it() {
    assert_eq!(
        "4/9",
        run_ppl(
            r#"
TYPE Inner
  INTEGER v
ENDTYPE
TYPE Outer
  Inner value
ENDTYPE
Inner source
Outer wrapper
source.v = 4
wrapper.value = source
wrapper.value.v = 9
PRINT source.v, "/", wrapper.value.v
"#
        )
    );
}
