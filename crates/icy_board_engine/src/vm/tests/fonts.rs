use super::{compile_errors_with_runtime, run_ppl, run_ppl_with_files_and_input};

/// A raw 8x16 font is recognised by its length alone: 256 glyphs of 16 rows.
fn raw_font() -> Vec<u8> {
    (0..256u32).flat_map(|glyph| (0..16).map(move |row| (glyph as u8).wrapping_add(row))).collect()
}

#[test]
fn font_api_requires_runtime_400() {
    for runtime in [330, 340] {
        let errors = compile_errors_with_runtime("Terminal.Font.Set(0, 5)\nTerminal.Font.Load(43, \"f.psf\")", runtime);
        assert!(!errors.is_empty(), "runtime {runtime} unexpectedly accepted member calls");
    }
    assert!(compile_errors_with_runtime("Terminal.Font.Set(0, 5)\nTerminal.Font.Load(43, \"f.psf\")", 400).is_empty());
}

#[test]
fn set_font_binds_a_font_to_an_attribute_slot() {
    let output = run_ppl(
        r#"
        Terminal.Font.Set(0, 5)
        Terminal.Font.Set(3, 42)
        "#,
    );

    assert_eq!(output, "\x1b[0;5 D\x1b[3;42 D");
}

#[test]
fn set_font_accepts_numbers_above_the_built_in_range() {
    // 0-42 are the terminal's own fonts, higher numbers address uploaded ones.
    let output = run_ppl("Terminal.Font.Set(0, 43)");

    assert_eq!(output, "\x1b[0;43 D");
}

#[test]
fn set_font_binds_every_slot_at_once() {
    let output = run_ppl("Terminal.Font.SetAll(43)");

    assert_eq!(output, "\x1b[0;43 D\x1b[1;43 D\x1b[2;43 D\x1b[3;43 D");
}

#[test]
fn font_statements_are_ignored_without_ansi() {
    let output = run_ppl(
        r#"
        GRAFMODE 4
        Terminal.Font.Set(0, 5)
        "#,
    );

    assert_eq!(output, "");
}

/// Without ANSI nothing is sent, but `ERR()` still has to answer for this statement
/// rather than leaving an older one's result in place.
#[test]
fn font_statements_report_unavailable_without_ansi() {
    let output = run_ppl(
        r#"
        GRAFMODE 4
        Terminal.Font.Set(0, 5)
        PrintLn "set=", ERR().Code, " ", ERR().Kind
        Terminal.Font.Load(43, "any.psf")
        PrintLn "load=", ERR().Code, " ", ERR().Kind
        "#,
    );

    assert!(output.contains("set=1 5\n"), "{output:?}");
    assert!(output.contains("load=1 5\n"), "{output:?}");
}

#[test]
fn invalid_font_arguments_are_not_sent() {
    let output = run_ppl(
        r#"
        Terminal.Font.Set(4, 5)
        Terminal.Font.Set(-2, 5)
        Terminal.Font.Set(0, -1)
        Terminal.Font.Set(0, 256)
        "#,
    );

    assert_eq!(output, "");
}

#[test]
fn set_font_reports_an_invalid_slot() {
    let output = run_ppl(
        r#"
        Terminal.Font.Set(0, 5)
        PrintLn ERR().Code
        Terminal.Font.Set(9, 5)
        PrintLn ERR().Code, " ", ERR().Kind
        "#,
    );

    assert!(output.ends_with("0\n2 5\n"), "{output:?}");
}

#[test]
fn load_font_uploads_the_glyph_data() {
    let font = raw_font();
    let output = run_ppl_with_files_and_input(
        r#"
        Terminal.Font.Load(43, "custom.fnt")
        PrintLn "ok=", ERR().OK
        "#,
        &[("custom.fnt", &font)],
        b"",
    );

    assert!(output.contains("\x1bPCTerm:Font:43:"), "{output:?}");
    assert!(output.contains("ok=1\n"), "{output:?}");
}

#[test]
fn load_font_rejects_the_built_in_range() {
    let font = raw_font();
    let output = run_ppl_with_files_and_input(
        r#"
        Terminal.Font.Load(5, "custom.fnt")
        PrintLn "err=", ERR().Code
        "#,
        &[("custom.fnt", &font)],
        b"",
    );

    assert!(!output.contains("CTerm:Font"), "{output:?}");
    assert!(output.contains("err=2\n"), "{output:?}");
}

#[test]
fn load_font_reports_a_missing_file() {
    let output = run_ppl(
        r#"
        Terminal.Font.Load(43, "nope.fnt")
        PrintLn "err=", ERR().Code
        "#,
    );

    assert!(!output.contains("CTerm:Font"), "{output:?}");
    assert!(output.contains("err=3\n"), "{output:?}");
}

#[test]
fn load_font_reports_an_unknown_format() {
    let output = run_ppl_with_files_and_input(
        r#"
        Terminal.Font.Load(43, "broken.fnt")
        PrintLn "err=", ERR().Code
        "#,
        &[("broken.fnt", b"not a font")],
        b"",
    );

    assert!(!output.contains("CTerm:Font"), "{output:?}");
    assert!(output.contains("err=4\n"), "{output:?}");
}
