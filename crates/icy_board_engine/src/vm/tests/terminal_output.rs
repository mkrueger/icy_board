use super::{compile_errors_with_runtime, run_ppl, run_ppl_with_cleanup};

#[test]
fn terminal_output_api_requires_runtime_400() {
    let source = r#"
        Terminal.BeginUpdate()
        Terminal.EndUpdate()
        Terminal.Macros.Record(0)
        Terminal.Macros.End()
        Terminal.Macros.Play(0)
        Terminal.Macros.Delete(0)
        Terminal.Macros.Clear()
    "#;
    for runtime in [330, 340] {
        let errors = compile_errors_with_runtime(source, runtime);
        assert!(
            errors.iter().any(|error| error.contains("Terminal needs runtime 400")),
            "runtime {runtime}: {errors:?}"
        );
    }
    assert!(compile_errors_with_runtime(source, 400).is_empty());
}

#[test]
fn synchronized_updates_only_emit_the_outer_pair() {
    let output = run_ppl(
        r#"
        Terminal.BeginUpdate()
        Terminal.BeginUpdate()
        Print "frame"
        Terminal.EndUpdate()
        Terminal.EndUpdate()
        "#,
    );

    assert_eq!(output, "\x1b[?2026hframe\x1b[?2026l");
}

#[test]
fn ending_an_inactive_update_reports_an_error() {
    let output = run_ppl(
        r#"
        Terminal.EndUpdate()
        PrintLn ERR().Code, " ", ERR().Kind
        "#,
    );

    assert_eq!(output, "2 7\n");
}

#[test]
fn synchronized_output_reports_unavailable_without_ansi() {
    let output = run_ppl(
        r#"
        GrafMode 4
        Terminal.BeginUpdate()
        PrintLn ERR().Code, " ", ERR().Kind
        "#,
    );

    assert_eq!(output, "1 7\n");
}

#[test]
fn terminal_macros_report_unavailable_without_ansi() {
    let output = run_ppl(
        r#"
        GrafMode 4
        Terminal.Macros.Record(0)
        PrintLn ERR().Code, " ", ERR().Kind
        "#,
    );

    assert_eq!(output, "1 7\n");
}

#[test]
fn outer_cleanup_flushes_recording_before_ending_synchronization() {
    let output = run_ppl_with_cleanup(
        r#"
        Terminal.BeginUpdate()
        Terminal.Macros.Record(4)
        Print "last frame"
        "#,
    );

    assert_eq!(output, "\x1b[?2026h\x1bP4;0;1!z6C617374206672616D65\x1b\\\x1b[4*z\x1bP4;0;1!z\x1b\\\x1b[?2026l");
}

#[test]
fn macros_hide_recorded_output_until_playback() {
    let output = run_ppl(
        r#"
        Terminal.Macros.Record(3)
        Print "hidden"
        Terminal.Macros.End()
        Print "before:"
        Terminal.Macros.Play(3)
        Terminal.Macros.Play(3)
        "#,
    );

    assert_eq!(output, "\x1bP3;0;1!z68696464656E\x1b\\before:\x1b[3*z\x1b[3*z");
}

#[test]
fn macros_can_compose_other_macros() {
    let output = run_ppl(
        r#"
        Terminal.Macros.Record(1)
        Print "A"
        Terminal.Macros.End()
        Terminal.Macros.Record(2)
        Print "["
        Terminal.Macros.Play(1)
        Print "]"
        Terminal.Macros.End()
        Terminal.Macros.Play(2)
        "#,
    );

    assert_eq!(output, "\x1bP1;0;1!z41\x1b\\\x1bP2;0;1!z5B1B5B312A7A5D\x1b\\\x1b[2*z");
}

#[test]
fn deleted_and_invalid_macros_report_errors() {
    let output = run_ppl(
        r#"
        Terminal.Macros.Record(1)
        Print "gone"
        Terminal.Macros.End()
        Terminal.Macros.Delete(1)
        Terminal.Macros.Play(1)
        PrintLn ERR().Code, " ", ERR().Kind
        Terminal.Macros.Record(64)
        PrintLn ERR().Code, " ", ERR().Kind
        "#,
    );

    assert_eq!(output, "\x1bP1;0;1!z676F6E65\x1b\\\x1bP1;0;1!z\x1b\\2 7\n2 7\n");
}

#[test]
fn clear_macros_emits_decdmac_delete_all() {
    let output = run_ppl(
        r#"
        Terminal.Macros.Record(0)
        Print "A"
        Terminal.Macros.End()
        Terminal.Macros.Clear()
        Terminal.Macros.Play(0)
        PrintLn ERR().Code, " ", ERR().Kind
        "#,
    );

    assert_eq!(output, "\x1bP0;0;1!z41\x1b\\\x1bP0;1;1!z\x1b\\2 7\n");
}

#[test]
fn a_second_recording_cannot_start_before_the_first_ends() {
    let output = run_ppl(
        r#"
        Terminal.Macros.Record(1)
        Terminal.Macros.Record(2)
        PrintLn ERR().Code, " ", ERR().Kind
        Terminal.Macros.End()
        Terminal.Macros.Play(1)
        "#,
    );

    assert_eq!(output, "\x1bP1;0;1!z3220370D0A\x1b\\\x1b[1*z");
}

#[test]
fn dec_macro_demo_uploads_composes_and_reuses_terminal_macros() {
    let source = include_str!("../../../../../ppe/dec_macros/src/dec_macros.pps");
    let output = run_ppl(source);

    assert_eq!(output.matches("\x1bP0;0;1!z").count(), 1, "{output:?}");
    assert_eq!(output.matches("\x1bP1;0;1!z").count(), 1, "{output:?}");
    assert!(output.contains("1B5B302A7A"), "panel macro does not invoke divider: {output:?}");
    assert_eq!(output.matches("\x1b[1*z").count(), 2, "{output:?}");
    assert_eq!(output.matches("\x1b[?2026h").count(), 1, "{output:?}");
    assert_eq!(output.matches("\x1b[?2026l").count(), 1, "{output:?}");
    assert!(output.contains("\x1bP0;1;1!z\x1b\\"), "{output:?}");
}
