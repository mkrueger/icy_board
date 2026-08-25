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
        SURFACE s = Terminal.Gfx.NewSurface(4, 4)
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
        Terminal.Gfx.SetPacing(1)
        PrintLn Terminal.Gfx.Pacing
        SURFACE s = Terminal.Gfx.NewSurface(2, 2)
        s.Present()
        Terminal.Gfx.Shutdown()
        ",
        b"\x1b[1;1R",
    );

    assert!(output.starts_with("0\n1\n"), "{output:?}");
    assert!(output.contains("\x1b[6n"), "{output:?}");
}

#[test]
fn the_geometry_calls_report_what_the_terminal_answered() {
    let output = run_ppl_with_input(
        r#"
        PrintLn Terminal.Gfx.CellWidth(), "x", Terminal.Gfx.CellHeight()
        PrintLn Terminal.Gfx.ScreenWidth(), "x", Terminal.Gfx.ScreenHeight()
        "#,
        JXL_TERMINAL,
    );

    assert!(output.ends_with("10x20\n800x600\n"), "{output:?}");
}

/// The capability calls replace the `GfxCaps()` bitmask, so they have to agree with it.
#[test]
fn the_capability_calls_agree_with_the_bitmask() {
    let output = run_ppl_with_input(
        r"
        PrintLn Terminal.Gfx.Sixel(), Terminal.Gfx.Jxl(), Terminal.Gfx.JxlBlob()
        PrintLn Terminal.Gfx.PixelMouse(), Terminal.Gfx.ClientBlit(), Terminal.Gfx.Audio()
        PrintLn GfxCaps()
        ",
        JXL_TERMINAL,
    );

    assert!(output.ends_with("110\n000\n35\n"), "{output:?}");
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
