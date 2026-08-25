//! Margins, palette and fonts, reached from the terminal they change.

use super::{compile_errors, run_ppl};

#[test]
fn margins_report_the_region_they_were_given() {
    let output = run_ppl(
        r#"
        PrintLn Terminal.Margins.HasVertical, Terminal.Margins.HasHorizontal
        Terminal.Margins.SetVertical(4, 23)
        Terminal.Margins.SetHorizontal(10, 70)
        PrintLn Terminal.Margins.Top, "-", Terminal.Margins.Bottom
        PrintLn Terminal.Margins.Left, "-", Terminal.Margins.Right
        PrintLn Terminal.Margins.HasVertical, Terminal.Margins.HasHorizontal
        Terminal.Margins.Reset()
        PrintLn Terminal.Margins.HasVertical, Terminal.Margins.HasHorizontal
        "#,
    );

    assert!(output.starts_with("00\n"), "{output:?}");
    assert!(output.contains("\x1b[4;23r"), "{output:?}");
    assert!(output.contains("\x1b[?69h\x1b[10;70s"), "{output:?}");
    assert!(output.contains("4-23\n10-70\n11\n"), "{output:?}");
    assert!(output.ends_with("00\n"), "{output:?}");
}

/// The old snapshot only ever held margins, so it is the margins that answer now.
#[test]
fn margins_answer_what_the_snapshot_used_to() {
    let output = run_ppl(
        r"
        Terminal.Margins.SetVertical(4, 23)
        TERMSTATE state = TermState()
        PrintLn state.MarginTop = Terminal.Margins.Top
        PrintLn state.VerticalMargins = Terminal.Margins.HasVertical
        ",
    );

    assert!(output.ends_with("1\n1\n"), "{output:?}");
}

#[test]
fn an_empty_region_is_refused_and_says_so() {
    let output = run_ppl(
        r"
        PrintLn Terminal.Margins.SetVertical(0, 23)
        PrintLn Terminal.Margins.SetVertical(23, 4)
        PrintLn Terminal.Margins.SetVertical(5, 5)
        PrintLn Terminal.Margins.SetHorizontal(-1, 70)
        PrintLn Terminal.Margins.SetVertical(4, 23)
        ",
    );

    assert!(output.starts_with("0\n0\n0\n0\n"), "{output:?}");
    assert!(output.ends_with("1\n"), "{output:?}");
}

/// Resetting one axis sends the sequence for that axis alone, but the screen behind it
/// treats DECSTBM as clearing the whole region, so both flags fall. The statements this
/// replaces behave the same way.
#[test]
fn resetting_one_axis_clears_both_flags() {
    let through_the_object = run_ppl(
        r"
        Terminal.Margins.SetVertical(4, 23)
        Terminal.Margins.SetHorizontal(10, 70)
        Terminal.Margins.ResetVertical()
        PrintLn Terminal.Margins.HasVertical, Terminal.Margins.HasHorizontal
        ",
    );
    let through_the_statements = run_ppl(
        r"
        SetVMargins 4, 23
        SetHMargins 10, 70
        ResetVMargins
        TERMSTATE state = TermState()
        PrintLn state.VerticalMargins, state.HorizontalMargins
        ",
    );

    assert!(through_the_object.ends_with("00\n"), "{through_the_object:?}");
    assert_eq!(through_the_object, through_the_statements);
}

#[test]
fn a_palette_colour_takes_a_packed_value_or_components() {
    let output = run_ppl(
        r"
        PrintLn Terminal.Palette.Set(1, Rgb(0, 64, 255))
        PrintLn Terminal.Palette.SetRgb(1, 0, 64, 255)
        PrintLn Terminal.Palette.Reset(1)
        PrintLn Terminal.Palette.ResetAll()
        ",
    );

    assert_eq!(output.matches("1\n").count(), 4, "every palette change should report success: {output:?}");
    assert!(output.contains("\x1b]4;4;rgb:00/40/FF\x1b\\"), "{output:?}");
    assert!(output.contains("\x1b]104;4\x1b\\"), "{output:?}");
    assert!(output.contains("\x1b]104\x1b\\"), "{output:?}");
}

#[test]
fn a_palette_colour_outside_the_sixteen_is_refused() {
    let output = run_ppl(
        r"
        PrintLn Terminal.Palette.Set(16, Rgb(0, 0, 0))
        PrintLn ERR().Code
        PrintLn Terminal.Palette.SetRgb(1, 0, 300, 0)
        PrintLn ERR().Code
        ",
    );

    assert!(output.ends_with("0\n2\n0\n2\n"), "{output:?}");
}

#[test]
fn a_font_binds_one_attribute_class_or_all_of_them() {
    let output = run_ppl(
        r"
        PrintLn Terminal.Font.Set(0, 5)
        PrintLn Terminal.Font.SetAll(5)
        ",
    );

    assert!(output.contains("\x1b[0;5 D"), "{output:?}");
    assert!(output.contains("\x1b[3;5 D"), "{output:?}");
}

#[test]
fn a_built_in_font_slot_cannot_be_uploaded_over() {
    let output = run_ppl(
        r#"
        PrintLn Terminal.Font.Load(42, "topaz.psf")
        PrintLn ERR().Code
        "#,
    );

    assert!(output.ends_with("0\n2\n"), "{output:?}");
}

/// These change the terminal, so they are reached through it rather than named on their own.
#[test]
fn the_appearance_objects_have_no_value_of_their_own() {
    for source in ["PRINTLN Margins.Top", "PRINTLN Palette.Set(1, 0)", "PRINTLN Font.Set(0, 1)"] {
        let errors = compile_errors(source);
        assert!(errors.iter().any(|error| error.contains("is a type")), "{source}: {errors:?}");
    }
}
