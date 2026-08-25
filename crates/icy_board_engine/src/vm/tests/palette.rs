use super::{compile_errors_with_runtime, run_ppl};

#[test]
fn palette_api_requires_runtime_400() {
    for runtime in [330, 340] {
        let errors = compile_errors_with_runtime(
            "Terminal.Palette.Set(1, Rgb(0, 64, 255))\nTerminal.Palette.Reset(1)\nTerminal.Palette.ResetAll()",
            runtime,
        );
        assert!(!errors.is_empty(), "runtime {runtime} unexpectedly accepted member calls");
    }
    assert!(
        compile_errors_with_runtime(
            "Terminal.Palette.Set(1, Rgb(0, 64, 255))\nTerminal.Palette.Reset(1)\nTerminal.Palette.ResetAll()",
            400,
        )
        .is_empty()
    );
}

#[test]
fn set_palette_color_accepts_packed_rgb_and_ignores_alpha() {
    let output = run_ppl("Terminal.Palette.Set(1, Rgb(0, 64, 255, 17))");

    assert_eq!(output, "\x1b]4;4;rgb:00/40/FF\x1b\\");
}

#[test]
fn set_palette_color_accepts_rgb_components() {
    let output = run_ppl("Terminal.Palette.SetRgb(4, 170, 16, 32)");

    assert_eq!(output, "\x1b]4;1;rgb:AA/10/20\x1b\\");
}

#[test]
fn palette_resets_one_color_or_all_colors() {
    let output = run_ppl("Terminal.Palette.Reset(1)\nTerminal.Palette.ResetAll()");

    assert_eq!(output, "\x1b]104;4\x1b\\\x1b]104\x1b\\");
}

#[test]
fn invalid_palette_values_are_not_sent() {
    let output = run_ppl(
        r#"
        Terminal.Palette.Set(16, Rgb(0, 0, 0))
        PrintLn ERR().Code, " ", ERR().Kind
        Terminal.Palette.SetRgb(1, -1, 0, 0)
        PrintLn ERR().Code, " ", ERR().Kind
        Terminal.Palette.Reset(-1)
        PrintLn ERR().Code, " ", ERR().Kind
        "#,
    );

    assert_eq!(output, "2 4\n2 4\n2 4\n");
}

#[test]
fn palette_statements_report_unavailable_without_ansi() {
    let output = run_ppl(
        r#"
        GRAFMODE 4
        Terminal.Palette.Set(1, Rgb(0, 64, 255))
        PrintLn ERR().Code, " ", ERR().Kind
        Terminal.Palette.Reset(1)
        PrintLn ERR().Code, " ", ERR().Kind
        Terminal.Palette.ResetAll()
        PrintLn ERR().Code, " ", ERR().Kind
        "#,
    );

    assert_eq!(output, "1 4\n1 4\n1 4\n");
}

#[test]
fn set_palette_color_rejects_three_arguments() {
    let errors = compile_errors_with_runtime("Terminal.Palette.Set(1, 2, 3)", 400);
    assert!(!errors.is_empty(), "{errors:?}");
}
