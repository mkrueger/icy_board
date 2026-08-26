//! Member calls are resolved by the receiver's type, not by how the receiver was written.

use super::{compile_errors, compile_errors_with_runtime, run_ppl};

#[test]
fn a_method_can_be_called_on_a_function_result() {
    assert!(compile_errors("PRINTLN Terminal.Info.Columns").is_empty());

    let output = run_ppl(
        r"
        TERMINPUT input = Terminal.Input
        PrintLn input.Wait(0).Kind
        ",
    );

    assert_eq!(output, "0\n");
}

#[test]
fn a_method_call_on_a_call_result_no_longer_reports_a_missing_function() {
    assert!(compile_errors("PRINTLN Terminal.Input.Poll().Kind").is_empty());
}

#[test]
fn an_unknown_member_is_still_reported() {
    let errors = compile_errors("PRINTLN Terminal.Info.NoSuchMember()");
    assert_eq!(errors, vec!["Member not found"]);
}

/// A variable is the nearer meaning of the name, so it keeps winning over the type.
#[test]
fn a_variable_shadows_a_type_of_the_same_name() {
    let output = run_ppl(
        r"
        TERMINFO TermInfo = Terminal.Info
        PrintLn TermInfo.Rows
        ",
    );

    assert_eq!(output, "25\n");
}

/// Surface has no instance of its own, so naming the type cannot mean a value.
#[test]
fn a_type_without_an_instance_is_rejected() {
    let errors = compile_errors("PRINTLN Surface.Width");
    assert_eq!(errors, vec!["'Surface' is a type, and this one has no value of its own to read members from"]);
}

#[test]
fn a_static_receiver_needs_runtime_400() {
    let errors = compile_errors_with_runtime("PRINTLN Terminal.Info.Columns", 340);
    assert!(errors.iter().any(|error| error.contains("Terminal needs runtime 400")), "{errors:?}");
}

/// The facade hangs objects off objects, so a property has to be able to answer with one.
#[test]
fn a_property_can_answer_with_another_object() {
    assert!(compile_errors("PRINTLN Terminal.Info.Columns").is_empty());

    let output = run_ppl(
        r#"
        PrintLn Terminal.Info.Columns, "x", Terminal.Info.Rows
        TERMINAL term = Terminal()
        TERMINFO info = term.Info
        PrintLn info.Program
        "#,
    );

    assert_eq!(output, "80x25\nUnknown\n");
}

/// A statement is parsed as the expression it is, so a call may stand anywhere in the
/// chain rather than only at its end.
#[test]
fn a_statement_may_call_in_the_middle_of_a_chain() {
    let output = run_ppl(
        r"
        Terminal.Gfx.Init(GfxBackend.Sixel, FALSE)
        Surface.New(2, 2).Free()
        PrintLn Error.Last().Code
        Terminal.Gfx.Shutdown()
        ",
    );

    assert_eq!(output, "0\n");
}

#[test]
fn a_static_call_is_a_statement_of_its_own() {
    assert!(compile_errors("Terminal.Gfx.Shutdown()").is_empty());
    assert!(compile_errors("Terminal.Gfx.Init(GfxBackend.Sixel, FALSE)").is_empty());
}

/// What a call answers is a copy, so there is nothing behind it to assign to.
#[test]
fn a_call_in_the_chain_cannot_be_assigned_through() {
    let errors = compile_errors("Surface.New(2, 2).Width = 3");
    assert_eq!(errors, vec!["'Width' can only be read"]);
}

#[test]
fn a_writable_property_on_a_call_result_uses_its_setter() {
    assert!(compile_errors("Audio.Load(\"missing\").Volume = 50").is_empty());
    assert!(compile_errors("LET Audio.Load(\"missing\").Volume = 50").is_empty());
    assert!(run_ppl("Audio.Load(\"missing\").Volume = 50\nPrintLn \"alive\"").ends_with("alive\n"));

    let errors = compile_errors("Audio.Load(\"missing\").Volume = GfxBackend.Sixel");
    assert_eq!(errors, vec!["Argument 1 expects Integer, got GfxBackend"]);

    let errors = compile_errors("Audio.Load(\"missing\").Volume(50)");
    assert_eq!(errors, vec!["Function not found (Volume)"]);
}

#[test]
fn a_member_that_is_neither_called_nor_assigned_is_reported() {
    let errors = compile_errors("Terminal.Info.Columns");
    assert!(!errors.is_empty(), "a bare member reference is not a statement");
}

/// A board object's name is an ordinary word, so a program that already uses it for a
/// variable keeps it. Only a type followed by a name declares.
#[test]
fn a_variable_may_be_named_after_a_board_object() {
    let output = run_ppl(
        r"
        UNSIGNED palette(4)
        INTEGER font
        palette[1] = 7
        font = 3
        PRINTLN palette[1], font
        ",
    );

    assert_eq!(output, "73\n");
}

/// Input reaches the same keyboard however it is named, so a PPE need not carry it around.
#[test]
fn the_input_object_is_reached_from_the_terminal() {
    let output = super::run_ppl_with_input(
        r"
        PRINTLN Terminal.Input.Poll().Text
        TERMINPUT input = Terminal.Input
        PRINTLN input.Poll().Text
        input.Release()
        ",
        b"ab",
    );

    assert_eq!(output, "a\nb\n");
}
