use super::{compile_errors_with_runtime, run_ppl, run_ppl_with_input};

#[test]
fn api_review_margin_mutations_publish_failure_and_success() {
    let output = run_ppl(
        r#"
Error.Clear()
BOOLEAN changed = Terminal.Margins.SetVertical(0, 10)
PRINTLN changed, "|", Error.Last().Kind = ErrKind.Term, "|", Error.Last().Code = ErrCode.Invalid
changed = Terminal.Margins.SetVertical(1, 10)
PRINTLN changed, "|", Error.Last().OK
changed = Terminal.Margins.SetHorizontal(5, 5)
PRINTLN changed, "|", Error.Last().Kind = ErrKind.Term, "|", Error.Last().Code = ErrCode.Invalid
changed = Terminal.Margins.SetHorizontal(1, 20)
PRINTLN changed, "|", Error.Last().OK
Terminal.Margins.SetVertical(0, 10)
Terminal.Margins.ResetVertical()
PRINTLN Error.Last().OK
Terminal.Margins.SetHorizontal(0, 10)
Terminal.Margins.ResetHorizontal()
PRINTLN Error.Last().OK
Terminal.Margins.SetVertical(0, 10)
Terminal.Margins.ResetAll()
PRINTLN Error.Last().OK
"#,
    );
    assert_eq!(
        output,
        "0|1|1\n\x1b[1;10r1|1\n0|1|1\n\x1b[?69h\x1b[1;20s1|1\n\x1b[r1\n\x1b[?69l1\n\x1b[r\x1b[?69l1\n"
    );
}

#[test]
fn api_review_invalid_margin_region_enters_the_error_handler() {
    let output = run_ppl(
        r#"
ON ERROR GOTO Failed
Terminal.Margins.SetHorizontal(-1, 10)
PRINTLN "not reached"
EXIT
:Failed
PRINTLN Error.Last().Kind = ErrKind.Term, "|", Error.Last().Code = ErrCode.Invalid
"#,
    );
    assert_eq!(output, "1|1\n");
}

#[test]
fn margin_api_requires_runtime_400() {
    for runtime in [330, 340] {
        let errors = compile_errors_with_runtime(
            "Terminal.Margins.SetVertical(1, 2)\nTerminal.Margins.SetHorizontal(1, 2)\nTerminal.Margins.ResetAll()",
            runtime,
        );
        assert!(!errors.is_empty(), "runtime {runtime} unexpectedly accepted member calls");
    }
    assert!(
        compile_errors_with_runtime(
            "Terminal.Margins.SetVertical(1, 2)\nTerminal.Margins.SetHorizontal(1, 2)\nTerminal.Margins.ResetAll()",
            400,
        )
        .is_empty()
    );
}

#[test]
fn margin_statements_emit_independent_ansi_regions() {
    let output = run_ppl(
        r#"
        Terminal.Margins.SetVertical(4, 23)
        Terminal.Margins.SetHorizontal(10, 70)
        Terminal.Margins.ResetVertical()
        Terminal.Margins.ResetHorizontal()
        Terminal.Margins.ResetAll()
        "#,
    );

    assert_eq!(output, "\x1b[4;23r\x1b[?69h\x1b[10;70s\x1b[r\x1b[?69l\x1b[r\x1b[?69l");
}

#[test]
fn margin_statements_are_ignored_without_ansi() {
    let output = run_ppl(
        r#"
        GRAFMODE 4
        Terminal.Margins.SetVertical(4, 23)
        Terminal.Margins.SetHorizontal(10, 70)
        Terminal.Margins.ResetAll()
        "#,
    );

    assert_eq!(output, "");
}

#[test]
fn invalid_margin_regions_are_not_sent() {
    let output = run_ppl(
        r#"
        Terminal.Margins.SetVertical(0, 23)
        Terminal.Margins.SetVertical(23, 4)
        Terminal.Margins.SetVertical(5, 5)
        Terminal.Margins.SetHorizontal(-1, 70)
        Terminal.Margins.SetHorizontal(70, 10)
        "#,
    );

    assert_eq!(output, "");
}

#[test]
fn margins_report_current_state() {
    let output = run_ppl(
        r#"
        PRINTLN Terminal.Margins.HasVertical, ":", Terminal.Margins.HasHorizontal
        PRINTLN Terminal.Margins.Top, ":", Terminal.Margins.Bottom, ":", Terminal.Margins.Left, ":", Terminal.Margins.Right

        Terminal.Margins.SetVertical(4, 23)
        Terminal.Margins.SetHorizontal(10, 70)
        PRINTLN Terminal.Margins.HasVertical, ":", Terminal.Margins.HasHorizontal
        PRINTLN Terminal.Margins.Top, ":", Terminal.Margins.Bottom, ":", Terminal.Margins.Left, ":", Terminal.Margins.Right
        "#,
    );

    assert!(output.ends_with("0:0\n0:0:0:0\n\x1b[4;23r\x1b[?69h\x1b[10;70s1:1\n4:23:10:70\n"), "{output:?}");
}

#[test]
fn margin_demo_handles_click_and_wheel_input() {
    let source = include_str!("../../../../../ppe/margins/src/margins.pps");
    let output = run_ppl_with_input(source, b"\x1b[<0;20;18M\x1b[<65;20;18M\r");

    assert!(output.contains("\x1b[5;18r\x1b[?69h\x1b[18;63s"), "{output:?}");
    let cleared = output.rfind("\x1b[18;18H\x1b[37;40m Quick scan").expect("old selection was not cleared");
    let scrolled = output.find("\x1b[1S").expect("list did not scroll");
    assert!(cleared < scrolled, "old selection must be cleared before scrolling: {output:?}");
    assert!(output.ends_with("Selected: Recent uploads\n"), "{output:?}");
}

#[test]
fn margin_demo_clears_selection_before_scrolling_up() {
    let source = include_str!("../../../../../ppe/margins/src/margins.pps");
    let output = run_ppl_with_input(source, b"\x1b[<0;20;18M\x1b[<65;20;18M\x1b[H\r");

    let cleared = output.rfind("\x1b[18;18H\x1b[37;40m Recent uploads").expect("old selection was not cleared");
    let scrolled = output.find("\x1b[1T").expect("list did not scroll up");
    assert!(cleared < scrolled, "old selection must be cleared before scrolling up: {output:?}");
    assert!(output.ends_with("Selected: Amber terminal\n"), "{output:?}");
}
