//! Member calls are resolved by the receiver's type, not by how the receiver was written.

use super::{compile_errors, compile_errors_with_runtime, run_ppl};

#[test]
fn a_method_can_be_called_on_a_function_result() {
    assert!(compile_errors("PRINTLN TermInfo().Columns").is_empty());

    let output = run_ppl(
        r"
        TERMINPUT input = TermInput()
        PrintLn input.Wait(0).Kind
        ",
    );

    assert_eq!(output, "0\n");
}

#[test]
fn a_method_call_on_a_call_result_no_longer_reports_a_missing_function() {
    assert!(compile_errors("PRINTLN TermInput().Poll().Kind").is_empty());
}

/// `TermInfo` names both a type and a function, and the type used to win and drop the member.
#[test]
fn a_member_of_a_type_named_like_a_function_is_not_dropped() {
    let errors = compile_errors("PRINTLN TermInput.Poll().Kind");
    assert!(!errors.iter().any(|error| error.contains("Poll")), "{errors:?}");
}

#[test]
fn an_unknown_member_is_still_reported() {
    let errors = compile_errors("PRINTLN TermInfo().NoSuchMember()");
    assert_eq!(errors, vec!["Member not found"]);
}

#[test]
fn a_type_name_stands_in_for_its_one_instance() {
    assert!(compile_errors("PRINTLN TermInfo.Columns").is_empty());

    let output = run_ppl(
        r#"
        PrintLn TermInfo.Columns, "x", TermInfo.Rows
        PrintLn TermState.VerticalMargins
        PrintLn Error.OK
        "#,
    );

    assert_eq!(output, "80x25\n0\n1\n");
}

#[test]
fn a_static_receiver_reads_the_same_state_as_the_call() {
    let output = run_ppl(
        r#"
        SetVMargins 4, 23
        PrintLn TermState.MarginTop, " ", TermState().MarginTop
        "#,
    );

    assert!(output.ends_with("4 4\n"), "{output:?}");
}

/// A variable is the nearer meaning of the name, so it keeps winning over the type.
#[test]
fn a_variable_shadows_a_type_of_the_same_name() {
    let output = run_ppl(
        r"
        TERMINFO TermInfo = TermInfo()
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
fn a_static_receiver_needs_runtime_402() {
    let errors = compile_errors_with_runtime("PRINTLN TermInfo.Columns", 401);
    assert!(errors.iter().any(|error| error == "TermInfo needs runtime 402"), "{errors:?}");
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
        Terminal.Gfx.Init(GFX_SIXEL, FALSE)
        Terminal.Gfx.NewSurface(2, 2).Free()
        PrintLn ERR().Code
        Terminal.Gfx.Shutdown()
        ",
    );

    assert_eq!(output, "0\n");
}

#[test]
fn a_static_call_is_a_statement_of_its_own() {
    assert!(compile_errors("Terminal.Gfx.Shutdown()").is_empty());
    assert!(compile_errors("Terminal.Gfx.Init(GFX_SIXEL, FALSE)").is_empty());
}

/// What a call answers is a copy, so there is nothing behind it to assign to.
#[test]
fn a_call_in_the_chain_cannot_be_assigned_through() {
    let errors = compile_errors("Terminal.Gfx.NewSurface(2, 2).Width = 3");
    assert!(!errors.is_empty(), "assigning through a call should be reported");
}

#[test]
fn a_member_that_is_neither_called_nor_assigned_is_reported() {
    let errors = compile_errors("Terminal.Info.Columns");
    assert!(!errors.is_empty(), "a bare member reference is not a statement");
}
