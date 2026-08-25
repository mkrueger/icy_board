//! The graphics object does what the GFX* globals do, reached from the terminal.

use super::{compile_errors, run_ppl, run_ppl_with_input};

const JXL_TERMINAL: &[u8] = b"\x1b[<1;4;7;8c\x1b[6;20;10t\x1b[4;600;800t\x1b[=1;1-n\x1b_SyncTERM:C;L\n\x1b\\";

#[test]
fn the_backend_reads_back_what_init_selected() {
    let output = run_ppl(
        r"
        PrintLn Terminal.Gfx.Backend
        Terminal.Gfx.Init(GFX_SIXEL, FALSE)
        PrintLn Terminal.Gfx.Backend
        Terminal.Gfx.Shutdown()
        PrintLn Terminal.Gfx.Backend
        ",
    );

    assert_eq!(output, "-1\n2\n-1\n");
}

#[test]
fn a_surface_made_through_the_object_draws_the_same_way() {
    let output = run_ppl(
        r#"
        Terminal.Gfx.Init(GFX_SIXEL, FALSE)
        SURFACE s = Surface.New(4, 4)
        PrintLn s.Valid, " ", s.Width, "x", s.Height
        s.Clear(Rgb(0, 0, 0))
        s.SetPixel(1, 1, Rgb(255, 0, 0))
        PrintLn s.GetPixel(1, 1) = Rgb(255, 0, 0)
        s.Free()
        Terminal.Gfx.Shutdown()
        "#,
    );

    assert_eq!(output, "1 4x4\n1\n");
}

#[test]
fn pacing_reads_back_what_it_was_set_to() {
    let output = run_ppl_with_input(
        r"
        Terminal.Gfx.Init(GFX_SIXEL, FALSE)
        PrintLn Terminal.Gfx.Pacing
        Terminal.Gfx.Pacing = 1
        PrintLn Terminal.Gfx.Pacing
        SURFACE s = Surface.New(2, 2)
        s.Present()
        Terminal.Gfx.Shutdown()
        ",
        b"\x1b[1;1R",
    );

    assert!(output.starts_with("0\n1\n"), "{output:?}");
    assert!(output.contains("\x1b[6n"), "{output:?}");
}

#[test]
fn a_writable_property_may_be_set_through_a_stored_object() {
    let output = run_ppl(
        r"
        Terminal.Gfx.Init(GfxBackend.Sixel, FALSE)
        GFX graphics = Terminal.Gfx
        graphics.Pacing = 2
        PrintLn graphics.Pacing
        Terminal.Gfx.Shutdown()
        ",
    );

    assert_eq!(output, "1\n");
}

#[test]
fn a_read_only_property_cannot_be_assigned() {
    let errors = compile_errors("Terminal.Gfx.Backend = GfxBackend.Sixel");
    assert!(!errors.is_empty(), "a read-only property must stay read-only");
}

#[test]
fn a_writable_property_checks_the_value_type() {
    let errors = compile_errors("Terminal.Gfx.Pacing = GfxBackend.Sixel");
    assert!(!errors.is_empty(), "an enum must not be assigned to an integer property");
}

#[test]
fn terminal_info_reports_the_geometry_the_terminal_answered() {
    let output = run_ppl_with_input(
        r#"
        Terminal.Gfx.Init(GfxBackend.Auto, FALSE)
        PrintLn Terminal.Info.CellWidth, "x", Terminal.Info.CellHeight
        PrintLn Terminal.Info.ScreenWidth, "x", Terminal.Info.ScreenHeight
        "#,
        JXL_TERMINAL,
    );

    assert!(output.ends_with("10x20\n800x600\n"), "{output:?}");
}

/// The terminal's capabilities all live in one cached snapshot.
#[test]
fn terminal_info_reports_all_capabilities() {
    let output = run_ppl_with_input(
        r"
        Terminal.Gfx.Init(GfxBackend.Auto, FALSE)
        PrintLn Terminal.Info.Sixel, Terminal.Info.Jxl, Terminal.Info.InlineGraphics
        PrintLn Terminal.Info.PixelMouse, Terminal.Info.ClientBlit, Terminal.Info.Sound
        ",
        JXL_TERMINAL,
    );

    assert!(output.ends_with("110\n000\n"), "{output:?}");
}

#[test]
fn an_unknown_backend_leaves_the_session_without_graphics() {
    let output = run_ppl(
        r"
        PrintLn Terminal.Gfx.Init(99)
        PrintLn Terminal.Gfx.Backend
        ",
    );

    assert_eq!(output, "0\n-1\n");
}

/// Gfx has no instance of its own; it is reached through the terminal that draws.
#[test]
fn the_graphics_object_is_only_reachable_through_the_terminal() {
    let errors = compile_errors("PRINTLN Gfx.Backend");
    assert_eq!(errors, vec!["'Gfx' is a type, and this one has no value of its own to read members from"]);
}
