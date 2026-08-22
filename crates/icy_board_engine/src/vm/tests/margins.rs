use super::{compile_errors_with_runtime, run_ppl, run_ppl_with_input};

#[test]
fn margin_api_requires_runtime_402() {
    for runtime in [400, 401] {
        let errors = compile_errors_with_runtime("SetVMargins 1, 2\nSetHMargins 1, 2\nResetMargins\nTERMSTATE s = TermState()", runtime);
        for needed in ["SetVMargins needs runtime 402", "SetHMargins needs runtime 402", "TermState needs runtime 402"] {
            assert!(errors.iter().any(|error| error == needed), "runtime {runtime}: {errors:?}");
        }
    }
    assert!(compile_errors_with_runtime("SetVMargins 1, 2\nSetHMargins 1, 2\nResetMargins\nTERMSTATE s = TermState()", 402).is_empty());
}

#[test]
fn margin_statements_emit_independent_ansi_regions() {
    let output = run_ppl(
        r#"
        SetVMargins 4, 23
        SetHMargins 10, 70
        ResetVMargins
        ResetHMargins
        ResetMargins
        "#,
    );

    assert_eq!(output, "\x1b[4;23r\x1b[?69h\x1b[10;70s\x1b[r\x1b[?69l\x1b[r\x1b[?69l");
}

#[test]
fn margin_statements_are_ignored_without_ansi() {
    let output = run_ppl(
        r#"
        GRAFMODE 4
        SetVMargins 4, 23
        SetHMargins 10, 70
        ResetMargins
        "#,
    );

    assert_eq!(output, "");
}

#[test]
fn invalid_margin_regions_are_not_sent() {
    let output = run_ppl(
        r#"
        SetVMargins 0, 23
        SetVMargins 23, 4
        SetVMargins 5, 5
        SetHMargins -1, 70
        SetHMargins 70, 10
        "#,
    );

    assert_eq!(output, "");
}

#[test]
fn term_state_reports_margin_snapshots() {
    let output = run_ppl(
        r#"
        TERMSTATE before = TermState()
        PRINTLN before.VerticalMargins, ":", before.HorizontalMargins
        PRINTLN before.MarginTop, ":", before.MarginBottom, ":", before.MarginLeft, ":", before.MarginRight

        SetVMargins 4, 23
        SetHMargins 10, 70
        TERMSTATE active = TermState()
        PRINTLN active.VerticalMargins, ":", active.HorizontalMargins
        PRINTLN active.MarginTop, ":", active.MarginBottom, ":", active.MarginLeft, ":", active.MarginRight
        PRINTLN before.VerticalMargins, ":", before.HorizontalMargins
        "#,
    );

    assert!(
        output.ends_with("0:0\n0:0:0:0\n\x1b[4;23r\x1b[?69h\x1b[10;70s1:1\n4:23:10:70\n0:0\n"),
        "{output:?}"
    );
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
