//! N2: square brackets and legacy parentheses address the same record storage.
//! Positive cases pass through compilation, PPE serialization/reload and the VM.

use super::{compile_errors, run_ppl};

#[test]
fn record_array_field_writes_and_compound_operators_accept_both_delimiters() {
    for (open, close) in [("[", "]"), ("(", ")")] {
        let source = format!(
            r#"
;$LANGVERSION 400
TYPE Item
    STRING names[1]
    INTEGER numbers[1]
ENDTYPE
Item item
item.names{open}0{close} = "new"
LET item.names{open}0{close} += "!"
item.numbers{open}0{close} = 10
item.numbers{open}0{close} += 2
item.numbers{open}0{close} -= 1
item.numbers{open}0{close} *= 4
item.numbers{open}0{close} /= 2
item.numbers{open}0{close} %= 8
item.numbers{open}0{close} &= 7
item.numbers{open}0{close} |= 8
PRINT item.names[0], " ", item.numbers[0], " ", item.numbers[1]
"#
        );
        assert_eq!(run_ppl(&source), "new! 1 0", "{open}{close}");
    }
}

#[test]
fn nested_record_array_writes_accept_mixed_delimiters_and_record_values() {
    for (open, close) in [("[", "]"), ("(", ")")] {
        let source = format!(
            r#"
;$LANGVERSION 400
TYPE Leaf
    INTEGER value
ENDTYPE
TYPE Branch
    Leaf rows[1]
ENDTYPE
Branch item
Branch roots[1]
item.rows{open}0{close} = Leaf {{ value = 7 }}
LET item.rows{open}0{close}.value += 2
roots[0].rows{open}1{close}.value = 11
roots(0).rows{open}1{close}.value += 3
PRINT item.rows[0].value, " ", roots[0].rows[1].value, " ", item.rows[1].value
"#
        );
        assert_eq!(run_ppl(&source), "9 14 0", "{open}{close}");
    }
}

#[test]
fn multidimensional_record_array_field_writes_accept_both_delimiters() {
    for (open, close) in [("[", "]"), ("(", ")")] {
        let source = format!(
            r#"
;$LANGVERSION 400
TYPE Item
    INTEGER matrix[1, 1]
    INTEGER cube[1, 1, 1]
ENDTYPE
Item item
item.matrix{open}1, 0{close} = 7
item.matrix{open}1, 0{close} += 3
LET item.cube{open}0, 1, 0{close} = 11
item.cube{open}0, 1, 0{close} *= 2
PRINT item.matrix(1, 0), " ", item.cube(0, 1, 0), " ", item.cube(1, 1, 1)
"#
        );
        assert_eq!(run_ppl(&source), "10 22 0", "{open}{close}");
    }
}

#[test]
fn bracket_compound_paths_capture_each_index_once_before_the_rhs() {
    let source = r#"
;$LANGVERSION 400
TYPE Leaf
    INTEGER cells[1, 1]
    INTEGER sibling
ENDTYPE
TYPE Branch
    Leaf leaves[1]
ENDTYPE
Branch roots[1]
INTEGER calls
roots[0].leaves[0].cells[0, 0] = 10
IF FALSE roots[Index()].leaves[Index()].cells[Index(), Index()] += Change()
roots[Index()].leaves[Index()].cells[Index(), Index()] += Change()
PRINT " ", calls, " ", roots[0].leaves[0].cells(0, 0), " ", roots[0].leaves[0].sibling
FUNCTION Index() INTEGER
    calls += 1
    PRINT calls
    RETURN 0
ENDFUNC
FUNCTION Change() INTEGER
    PRINT "R"
    roots[0].leaves[0].cells[0, 0] = 100
    roots[0].leaves[0].sibling = 77
    RETURN 5
ENDFUNC
"#;
    assert_eq!(run_ppl(source), "1234R 4 15 77");
}

#[test]
fn bracket_record_array_writes_preserve_value_copy_isolation() {
    assert_eq!(
        run_ppl(
            r#"
;$LANGVERSION 400
TYPE Item
    STRING names[1]
ENDTYPE
Item original, copied
original.names[0] = "old"
copied = original
copied.names[0] += "!"
PRINT original.names[0], " ", copied.names[0]
"#
        ),
        "old old!"
    );
}

#[test]
fn string_character_reads_remain_distinct_from_record_array_elements() {
    assert_eq!(
        run_ppl(
            r#"
;$LANGVERSION 400
TYPE Item
    STRING text
    STRING names[1]
ENDTYPE
Item item
item.text = "abc"
item.names(0) = "xyz"
PRINT item.text[1], " ", item.names[0], " ", item.names[0][1], " ", "abc"[1]
"#
        ),
        "b xyz y b"
    );
}

#[test]
fn record_string_character_indices_do_not_become_writable() {
    for target in ["item.text[0]", "item.names[0][0]"] {
        for operator in ["=", "+="] {
            let source = format!(";$LANGVERSION 400\nTYPE Item\n STRING text\n STRING names[1]\nENDTYPE\nItem item\n{target} {operator} \"x\"\n");
            assert!(!compile_errors(&source).is_empty(), "{source}");
        }
    }
}

#[test]
fn bracket_writes_do_not_enable_readonly_properties_or_snapshot_paths() {
    assert_eq!(
        compile_errors(";$LANGVERSION 400\nBoard.Conferences[0].Name = \"x\""),
        vec!["'Name' can only be read"]
    );
    assert_eq!(
        compile_errors(";$LANGVERSION 400\nSession.User.Contacts[0].Account = \"x\""),
        vec!["Can't assign value to."]
    );
    for write in [
        "LET Board.Conferences[0].Doors[0].Description += \"x\"",
        "USER person = Session.User\nperson.Contacts[0].Account = \"x\"",
        "CONFERENCE conf = Session.Conference\nconf.Areas[0].Name += \"x\"",
        "Session.Conference.Name[0] = \"x\"",
        "CONTACT entry\nSession.User.Contacts[0] = entry",
        "CONTACT entry\nUSER person = Session.User\nperson.Contacts[0] = entry",
    ] {
        let source = format!(";$LANGVERSION 400\n{write}\n");
        assert!(!compile_errors(&source).is_empty(), "{write}");
    }
}

#[test]
fn bracket_record_array_targets_reject_wrong_rank_and_unknown_fields() {
    for target in ["item.values[0]", "item.values[0, 0, 0]", "item.missing[0]"] {
        let source = format!(";$LANGVERSION 400\nTYPE Item\n INTEGER values[1, 1]\nENDTYPE\nItem item\n{target} = 7\n");
        let errors = compile_errors(&source);
        assert!(!errors.is_empty(), "{target}");
        let legacy_target = target.replace('[', "(").replace(']', ")");
        assert_eq!(errors, compile_errors(&source.replace(target, &legacy_target)), "{target}");
    }
}
