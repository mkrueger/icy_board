//! Synchronized updates, the terminal's macro slots and what it can play.

use super::{compile_errors, run_ppl};

#[test]
fn an_update_pair_wraps_the_redraw_in_mode_2026() {
    let output = run_ppl(
        r#"
        Terminal.BeginUpdate()
        PRINT "a"
        Terminal.EndUpdate()
        "#,
    );

    assert_eq!(output, "\x1b[?2026ha\x1b[?2026l");
}

/// Only the outer pair reaches the terminal, so a redraw may be built out of parts that
/// each open one of their own.
#[test]
fn nested_updates_only_emit_the_outer_pair() {
    let output = run_ppl(
        r#"
        Terminal.BeginUpdate()
        Terminal.BeginUpdate()
        PRINT "a"
        Terminal.EndUpdate()
        Terminal.EndUpdate()
        "#,
    );

    assert_eq!(output, "\x1b[?2026ha\x1b[?2026l");
}

#[test]
fn ending_an_update_that_never_began_says_so() {
    let output = run_ppl(
        r"
        PrintLn Terminal.EndUpdate()
        PrintLn Error.Last().Code
        ",
    );

    assert_eq!(output, "0\n2\n");
}

#[test]
fn a_macro_records_output_and_plays_it_back() {
    let output = run_ppl(
        r#"
        BOOLEAN before, during, after
        before = Terminal.Macros.Recording
        Terminal.Macros.StartRecord(3)
        during = Terminal.Macros.Recording
        PRINT "hi"
        Terminal.Macros.StopRecord()
        after = Terminal.Macros.Recording
        Terminal.Macros.Play(3)
        PrintLn before, during, after
        "#,
    );

    // Recorded output is held back until the definition is uploaded, so the text only
    // ever reaches the terminal as hex inside the definition.
    assert!(!output.contains("hi"), "{output:?}");
    assert!(output.contains("\x1bP3;0;1!z6869\x1b\\"), "{output:?}");
    assert!(output.contains("\x1b[3*z"), "{output:?}");
    assert!(output.ends_with("010\n"), "{output:?}");
}

#[test]
fn a_slot_outside_the_sixty_four_is_refused() {
    let output = run_ppl(
        r"
        PrintLn Terminal.Macros.StartRecord(64)
        PrintLn Error.Last().Code
        PrintLn Terminal.Macros.StartRecord(-1)
        PrintLn Error.Last().Code
        ",
    );

    assert_eq!(output, "0\n2\n0\n2\n");
}

#[test]
fn a_second_recording_is_refused_while_one_is_active() {
    let output = run_ppl(
        r"
        BOOLEAN second
        ERRCODE code
        Terminal.Macros.StartRecord(1)
        second = Terminal.Macros.StartRecord(2)
        code = Error.Last().Code
        Terminal.Macros.StopRecord()
        PrintLn second, code = ErrCode.Invalid
        ",
    );

    assert!(output.ends_with("01\n"), "{output:?}");
}

#[test]
fn playing_an_empty_slot_says_so() {
    let output = run_ppl(
        r"
        PrintLn Terminal.Macros.Play(7)
        PrintLn Error.Last().Code
        ",
    );

    assert_eq!(output, "0\n2\n");
}

#[test]
fn stopping_every_sound_flushes_the_channels_that_were_playing() {
    let output = run_ppl(
        r"
        PrintLn Audio.StopAll()
        ",
    );

    assert!(output.ends_with("1\n"), "{output:?}");
}

/// These belong to the terminal, so they are reached through it rather than named alone.
#[test]
fn the_terminal_objects_have_no_value_of_their_own() {
    for source in ["PRINTLN Macros.Recording", "PRINTLN Margins.Top"] {
        let errors = compile_errors(source);
        assert!(errors.iter().any(|error| error.contains("is a type")), "{source}: {errors:?}");
    }
}

#[test]
fn macro_capabilities_live_only_in_terminal_info() {
    let errors = compile_errors("PRINTLN Terminal.Macros.Available");
    assert!(errors.iter().any(|error| error.contains("Member not found")), "{errors:?}");
}
