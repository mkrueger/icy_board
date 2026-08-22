use super::{compile_errors_with_runtime, run_ppl};

#[test]
fn terminal_info_requires_runtime_402() {
    for runtime in [400, 401] {
        let errors = compile_errors_with_runtime("TERMINFO info = TermInfo()", runtime);
        assert!(
            errors.iter().any(|error| error == "TermInfo needs runtime 402"),
            "runtime {runtime}: {errors:?}"
        );
    }
    assert!(compile_errors_with_runtime("TERMINFO info = TermInfo()", 402).is_empty());
}

#[test]
fn terminal_info_returns_the_cached_local_snapshot() {
    let output = run_ppl(
        r#"
        TERMINFO info = TermInfo()
        PrintLn info.Program
        PrintLn info.DeviceAttrs = ""
        PrintLn info.Columns, "x", info.Rows
        PrintLn info.Utf8, " ", info.RipVersion = "", " ", info.CTermLevel
        PrintLn info.Sixel, " ", info.Jxl, " ", info.InlineGraphics, " ", info.Sound, " ", info.PhysicalKeys
        PrintLn info.CellWidth, "x", info.CellHeight, " ", info.ScreenWidth, "x", info.ScreenHeight
        "#,
    );

    assert_eq!(output, "Unknown\n1\n80x25\n1 1 0\n0 0 0 0 0\n8x16 0x0\n");
}
