//! Margins, palette and fonts, reached from the terminal they change.

use super::{compile_errors, run_ppl, run_ppl_with_cleanup};

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
        Terminal.Margins.ResetAll()
        PrintLn Terminal.Margins.HasVertical, Terminal.Margins.HasHorizontal
        "#,
    );

    assert!(output.starts_with("00\n"), "{output:?}");
    assert!(output.contains("\x1b[4;23r"), "{output:?}");
    assert!(output.contains("\x1b[?69h\x1b[10;70s"), "{output:?}");
    assert!(output.contains("4-23\n10-70\n11\n"), "{output:?}");
    assert!(output.ends_with("00\n"), "{output:?}");
}

#[test]
fn margins_answer_their_current_bounds() {
    let output = run_ppl(
        r#"
        Terminal.Margins.SetVertical(4, 23)
        PrintLn Terminal.Margins.Top, "-", Terminal.Margins.Bottom
        PrintLn Terminal.Margins.HasVertical
        "#,
    );

    assert!(output.ends_with("4-23\n1\n"), "{output:?}");
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

/// The two axes are independent: DECSTBM owns top and bottom, DECLRMM owns left and
/// right, so resetting one leaves the other standing.
#[test]
fn resetting_the_vertical_axis_keeps_the_horizontal_one() {
    let output = run_ppl(
        r"
        Terminal.Margins.SetVertical(4, 23)
        Terminal.Margins.SetHorizontal(10, 70)
        Terminal.Margins.ResetVertical()
        PrintLn Terminal.Margins.HasVertical, Terminal.Margins.HasHorizontal
        ",
    );

    assert!(output.ends_with("01\n"), "{output:?}");
}

/// Resetting the horizontal axis leaves the vertical one alone, which is what makes the
/// vertical case above stand out rather than being a rule about both.
#[test]
fn resetting_the_horizontal_axis_keeps_the_vertical_one() {
    let output = run_ppl(
        r#"
        Terminal.Margins.SetVertical(4, 23)
        Terminal.Margins.SetHorizontal(10, 70)
        Terminal.Margins.ResetHorizontal()
        PrintLn Terminal.Margins.HasVertical, Terminal.Margins.HasHorizontal
        PrintLn Terminal.Margins.Top, "-", Terminal.Margins.Bottom
        "#,
    );

    assert!(output.ends_with("10\n4-23\n"), "{output:?}");
}

/// Resetting only the vertical axis leaves DECLRMM enabled on the wire, and the screen
/// model cannot be trusted to remember that, so the PPE's own record of having set a
/// margin is what gets the caller's terminal put back.
#[test]
fn a_horizontal_margin_does_not_outlive_the_ppe() {
    let output = run_ppl_with_cleanup(
        r"
        Terminal.Margins.SetHorizontal(10, 70)
        Terminal.Margins.SetVertical(4, 23)
        Terminal.Margins.ResetVertical()
        ",
    );

    assert!(output.contains("\x1b[?69h"), "the horizontal margin was turned on: {output:?}");
    assert!(output.ends_with("\x1b[r\x1b[?69l"), "the caller must not keep it: {output:?}");
}

/// Resetting both axes puts the terminal provably back, so there is nothing left to undo.
#[test]
fn resetting_both_axes_leaves_nothing_for_cleanup_to_do() {
    let output = run_ppl_with_cleanup(
        r"
        Terminal.Margins.SetVertical(4, 23)
        Terminal.Margins.SetHorizontal(10, 70)
        Terminal.Margins.ResetAll()
        ",
    );

    assert_eq!(output, "\x1b[4;23r\x1b[?69h\x1b[10;70s\x1b[r\x1b[?69l");
}

#[test]
fn a_palette_colour_takes_a_packed_value_or_components() {
    let output = run_ppl(
        r"
        PrintLn Terminal.Palette.Set(1, Rgb(0, 64, 255))
        PrintLn Terminal.Palette.Set(1, Rgb(0, 64, 255))
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
        PrintLn Error.Last().Code
        PrintLn Terminal.Palette.Set(-1, Rgb(0, 0, 0))
        PrintLn Error.Last().Code
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
        PrintLn Error.Last().Code
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
