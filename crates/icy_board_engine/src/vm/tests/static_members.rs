//! A static member belongs to the type, so it is called without a value in hand.

use super::{compile_errors, compile_errors_with_runtime, run_ppl};

#[test]
fn a_surface_is_made_by_its_own_type() {
    let output = run_ppl(
        r#"
        Terminal.Gfx.Init(GfxBackend.Sixel, FALSE)
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
fn a_static_answers_the_same_as_the_global_it_replaces() {
    let output = run_ppl(
        r"
        Terminal.Gfx.Init(GfxBackend.Sixel, FALSE)
        SURFACE a = Surface.New(3, 2)
        SURFACE b = Surface.New(3, 2)
        PrintLn a.Width, a.Height, b.Width, b.Height
        PrintLn ERR().Code
        Terminal.Gfx.Shutdown()
        ",
    );

    assert_eq!(output, "3232\n0\n");
}

#[test]
fn a_static_reports_a_failure_the_same_way() {
    let output = run_ppl(
        r#"
        Terminal.Gfx.Init(GfxBackend.Sixel, FALSE)
        SURFACE missing = Surface.Load("missing.png")
        PrintLn missing.Valid, " ", ERR().Code
        Terminal.Gfx.Shutdown()
        "#,
    );

    assert_eq!(output, "0 3\n");
}

#[test]
fn audio_is_loaded_through_its_own_type() {
    let output = run_ppl(
        r#"
        AUDIO music = Audio.Load("missing.opus")
        PrintLn music.Valid, " ", music.Channel
        "#,
    );

    assert!(output.ends_with("0 -1\n"), "{output:?}");
}

#[test]
fn a_static_may_stand_in_a_chain() {
    let output = run_ppl(
        r"
        Terminal.Gfx.Init(GfxBackend.Sixel, FALSE)
        PrintLn Surface.New(6, 3).Width
        Surface.New(2, 2).Free()
        PrintLn ERR().Code
        Terminal.Gfx.Shutdown()
        ",
    );

    assert_eq!(output, "6\n0\n");
}

#[test]
fn a_static_checks_its_arguments() {
    let errors = compile_errors("SURFACE s = Surface.New(4)");
    assert!(!errors.is_empty(), "a missing argument should be reported");
}

#[test]
fn an_unknown_static_is_reported() {
    let errors = compile_errors("SURFACE s = Surface.Create(4, 4)");
    assert!(
        errors.iter().any(|error| error.contains("'Surface' is a type")),
        "a name that is neither a static nor an instance should say so: {errors:?}"
    );
}

#[test]
fn a_static_needs_runtime_400() {
    let errors = compile_errors_with_runtime("SURFACE s = Surface.New(4, 4)", 340);
    assert!(errors.iter().any(|error| error.contains("Surface.New")), "{errors:?}");
}

/// The surface constructors moved to the type, so the graphics session no longer has them.
#[test]
fn the_graphics_session_no_longer_makes_surfaces() {
    let errors = compile_errors("SURFACE s = Terminal.Gfx.Surface.New(4, 4)");
    assert!(errors.iter().any(|error| error == "Member not found"), "{errors:?}");
}

/// A constructor belongs to the type, so reaching it through a value would only confuse
/// what it makes with what it was called on.
#[test]
fn a_static_cannot_be_reached_through_a_value() {
    let errors = compile_errors(
        r"
        Terminal.Gfx.Init(GfxBackend.Sixel, FALSE)
        SURFACE s = Surface.New(2, 2)
        SURFACE other = s.New(4, 4)
        ",
    );

    assert!(
        errors
            .iter()
            .any(|error| error == "'New' belongs to the type itself, so it cannot be reached through a value"),
        "{errors:?}"
    );
}
