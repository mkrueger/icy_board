//! The board's own enums, which name the numbers the runtime already reports.

use super::{compile_errors, run_ppl, run_ppl_with_input};

#[test]
fn an_enum_member_stands_for_the_value_it_always_had() {
    let output = run_ppl(
        r"
        PrintLn EventKind.None, EventKind.Key, EventKind.KeyEdge, EventKind.Mouse, EventKind.Overflow, EventKind.Audio
        PrintLn GfxBackend.None, GfxBackend.Auto, GfxBackend.Sixel, GfxBackend.Jxl
        PrintLn MouseButton.Left, MouseButton.Middle, MouseButton.Right
        PrintLn ErrCode.Ok, ErrCode.Invalid, ErrKind.Audio, ErrKind.Term
        ",
    );

    assert_eq!(output, "012345\n-1023\n012\n0267\n");
}

#[test]
fn an_event_kind_is_compared_by_name() {
    let output = run_ppl_with_input(
        r"
        EVENT e = Terminal.Input.Poll()
        PrintLn e.Kind = EventKind.Key
        PrintLn e.Kind = EventKind.Mouse
        PrintLn e.Text
        ",
        b"a",
    );

    assert_eq!(output, "1\n0\na\n");
}

/// The point of the enum: a number that means something else cannot be compared to it.
#[test]
fn an_enum_will_not_compare_against_a_bare_number() {
    let errors = compile_errors("EVENT e\nPRINTLN e.Kind = 1");
    assert!(errors.iter().any(|error| error.contains("EventKind")), "{errors:?}");
}

#[test]
fn an_enum_will_not_compare_against_another_enum() {
    let errors = compile_errors("EVENT e\nPRINTLN e.Kind = MouseButton.Left");
    assert!(errors.iter().any(|error| error.contains("EventKind")), "{errors:?}");
}

#[test]
fn an_enum_variable_holds_what_it_was_given() {
    let output = run_ppl(
        r"
        EVENTKIND kind = EventKind.Mouse
        PrintLn kind = EventKind.Mouse
        PrintLn kind = EventKind.Key
        ",
    );

    assert_eq!(output, "1\n0\n");
}

/// `Code` used to answer for the key, the mouse action, the sound channel and the
/// number of dropped events, so each of those now says its own name.
#[test]
fn each_meaning_code_carried_has_a_name_of_its_own() {
    let output = run_ppl_with_input(
        r"
        TERMINPUT input = Terminal.Input
        EVENT e
        input.MouseOn(MouseMode.Text)
        e = input.Poll()
        PrintLn e.Kind = EventKind.Mouse
        PrintLn e.Action = MouseAction.Press
        PrintLn e.Button = MouseButton.Left
        PrintLn e.LeftDown, e.MiddleDown, e.RightDown
        input.Release()
        ",
        b"\x1b[<0;1;1M",
    );

    assert!(output.contains("1\n1\n1\n100\n"), "{output:?}");
}

#[test]
fn a_kind_that_does_not_act_reports_no_action() {
    let output = run_ppl_with_input(
        r#"
        EVENT e = Terminal.Input.Poll()
        PrintLn e.Kind = EventKind.Key
        PrintLn e.Action = MouseAction.None
        PrintLn e.Channel, " ", e.Dropped
        "#,
        b"a",
    );

    assert_eq!(output, "1\n1\n-1 0\n");
}

#[test]
fn modifiers_answer_as_booleans() {
    let output = run_ppl_with_input(
        r"
        EVENT e = Terminal.Input.Poll()
        PrintLn e.Shift, e.Alt, e.Ctrl, e.Meta
        ",
        b"\x1b[1;5A",
    );

    assert_eq!(output, "0010\n");
}

#[test]
fn the_backend_answers_as_the_backend_it_is() {
    let output = run_ppl(
        r"
        PrintLn Terminal.Gfx.Backend = GfxBackend.None
        Terminal.Gfx.Init(GfxBackend.Sixel, FALSE)
        PrintLn Terminal.Gfx.Backend = GfxBackend.Sixel
        Terminal.Gfx.Shutdown()
        ",
    );

    assert_eq!(output, "1\n1\n");
}

#[test]
fn an_error_names_its_kind_and_code() {
    let output = run_ppl(
        r"
        Terminal.Palette.Set(16, Rgb(0, 0, 0))
        PrintLn Error.Last().Kind = ErrKind.Gfx
        PrintLn Error.Last().Code = ErrCode.Invalid
        PrintLn Error.Last().OK
        ",
    );

    assert_eq!(output, "1\n1\n0\n");
}

#[test]
fn the_message_error_kind_keeps_its_runtime_value() {
    assert_eq!(run_ppl("PrintLn ErrKind.Msg"), "8\n");
}
