use super::{compile_errors_with_runtime, run_ppl, run_ppl_with_input, run_ppl_with_input_after_output};

#[test]
fn terminal_input_requires_runtime_400() {
    for runtime in [330, 340] {
        let errors = compile_errors_with_runtime("TERMINPUT input = Terminal.Input", runtime);
        assert!(
            errors.iter().any(|error| error.contains("Terminal needs runtime 400")),
            "runtime {runtime}: {errors:?}"
        );
    }
    assert!(compile_errors_with_runtime("TERMINPUT input = Terminal.Input", 400).is_empty());
}

#[test]
fn retired_global_event_api_is_not_in_source_language() {
    for source in [
        "EVENT event = EventPoll()",
        "EVENT event = EventWait(1)",
        "MouseOn MOUSE_TEXT",
        "MouseOff",
        "KeyEvents 1",
    ] {
        assert!(!compile_errors_with_runtime(source, 400).is_empty(), "{source}");
    }
}

/// Input is the terminal's, not a thing to be taken out and handed back, so naming it
/// twice names the same keyboard both times.
#[test]
fn terminal_input_is_the_terminals_own() {
    let output = run_ppl_with_input(
        r"
        TERMINPUT first = Terminal.Input
        TERMINPUT duplicate = Terminal.Input
        PRINTLN ERR().Code
        PRINTLN first.Poll().Text, duplicate.Poll().Text
        first.Release()
        ",
        b"ab",
    );

    assert_eq!(output, "0\nab\n");
}

#[test]
fn terminal_input_polls_translated_keys() {
    let output = run_ppl_with_input(
        r#"
        TERMINPUT input = Terminal.Input
        EVENT event = input.Poll()
        PRINTLN event.Kind, ":", event.Text
        input.Release()
        "#,
        b"\x1b[A",
    );

    assert_eq!(output, "1:UP\n");
}

#[test]
fn freeing_terminal_input_hands_typeahead_to_classic_input() {
    let output = run_ppl_with_input(
        r#"
        TERMINPUT input = Terminal.Input
        EVENT event = input.Poll()
        input.Release()
        STRING name
        INPUT "Name:", name
        PRINTLN "[", name, "]"
        "#,
        b"qMike\r",
    );

    assert!(output.ends_with("[Mike]\n"), "{output:?}");
}

#[test]
fn classic_input_reads_keys_sent_after_terminal_input_is_freed() {
    let output = run_ppl_with_input_after_output(
        r#"
        TERMINPUT input = Terminal.Input
        EVENT event = input.Wait(-1)
        input.Release()
        STRING name
        INPUT "Name:", name
        PRINTLN "[", name, "]"
        "#,
        b"Name:",
        b"Mike\r",
    );

    assert!(output.ends_with("[Mike]\n"), "{output:?}");
}

#[test]
fn timed_event_wait_restores_the_channel_before_classic_input() {
    let output = run_ppl_with_input_after_output(
        r#"
        TERMINPUT input = Terminal.Input
        EVENT event = input.Wait(1)
        input.Release()
        STRING name
        INPUT "Name:", name
        PRINTLN "[", name, "]"
        "#,
        b"Name:",
        b"Mike\r",
    );

    assert!(output.ends_with("[Mike]\n"), "{output:?}");
}

#[test]
fn unified_events_consume_character_key_edge_and_mouse_input() {
    let output = run_ppl_with_input(
        r#"
        TERMINPUT input = Terminal.Input
        EVENT e
        input.KeyboardOn()
        input.MouseOn(MouseMode.Text)

        e = input.Poll()
        PRINTLN e.Kind, ":", e.Code, ":", e.Text, ":", e.Pressed
        e = input.Poll()
        PRINTLN e.Kind, ":", e.Code, ":", e.Text, ":", e.Pressed
        e = input.Poll()
        PRINTLN e.Kind, ":", e.Code, ":", e.X, ":", e.Y, ":", e.Button, ":", e.Shift, e.Alt, e.Ctrl, e.Meta, ":", e.Pixels
        e = input.Poll()
        PRINTLN e.Kind, ":", e.Code, ":", e.Text, ":", e.Pressed, ":", e.X, ":", e.Y

        input.Release()
        "#,
        b"x\x1b[=30K\x1b[<20;11;6M",
    );

    assert!(output.contains("1:120:x:1\n2:30::1\n3:1:10:5:0:1010:0\n0:0::0:0:0\n"), "{output:?}");
}

#[test]
fn unified_events_preserve_wire_order_between_sources() {
    let output = run_ppl_with_input(
        r#"
        TERMINPUT input = Terminal.Input
        EVENT e
        input.KeyboardOn()
        input.MouseOn(MouseMode.Text)
        e = input.Poll()
        PRINTLN e.Kind, ":", e.Code
        e = input.Poll()
        PRINTLN e.Kind, ":", e.Text
        e = input.Poll()
        PRINTLN e.Kind, ":", e.Code
        input.Release()
        "#,
        b"\x1b[<0;3;4Mx\x1b[=31K",
    );

    assert!(output.contains("3:1\n1:x\n2:31\n"), "{output:?}");
}

#[test]
fn event_objects_keep_independent_snapshots() {
    let output = run_ppl_with_input(
        r#"
        TERMINPUT input = Terminal.Input
        EVENT first = input.Poll()
        EVENT second = input.Poll()
        PRINTLN first.Text, second.Text, first.Code, second.Code
        "#,
        b"ab",
    );

    assert_eq!(output, "ab9798\n");
}

#[test]
fn logical_ansi_keys_are_one_event() {
    let output = run_ppl_with_input(
        r#"
        TERMINPUT input = Terminal.Input
        EVENT e = input.Poll()
        PRINTLN e.Kind, ":", e.Code, ":", e.Text, ":", e.Repeated, ":", e.Shift, e.Alt, e.Ctrl, e.Meta
        "#,
        b"\x1b[1;5A",
    );
    assert_eq!(output, format!("1:{key}:UP:0:0010\n", key = 0x11_0001));
}

#[test]
fn mouse_events_report_held_buttons_and_wheel_deltas() {
    let output = run_ppl_with_input(
        r#"
        TERMINPUT input = Terminal.Input
        EVENT e
        input.MouseOn(MouseMode.Text)
        e = input.Poll()
        PRINTLN e.LeftDown, e.MiddleDown, e.RightDown, ":", e.WheelX, ":", e.WheelY
        e = input.Poll()
        PRINTLN e.LeftDown, e.MiddleDown, e.RightDown, ":", e.Button, ":", e.WheelX, ":", e.WheelY
        e = input.Poll()
        PRINTLN e.LeftDown, e.MiddleDown, e.RightDown
        input.Release()
        "#,
        b"\x1b[<0;1;1M\x1b[<66;1;1M\x1b[<0;1;1m",
    );
    assert!(output.contains("100:0:0\n100:5:-1:0\n000\n"), "{output:?}");
}

#[test]
fn negative_event_wait_means_indefinite() {
    let output = run_ppl_with_input(
        r#"
        TERMINPUT input = Terminal.Input
        EVENT e = input.Wait(-1)
        PRINT e.Text
        "#,
        b"q",
    );
    assert_eq!(output, "q");
}

#[test]
fn keyflush_discards_input_buffered_after_an_event() {
    let output = run_ppl_with_input(
        r#"
        TERMINPUT input = Terminal.Input
        EVENT e = input.Poll()
        KeyFlush
        PRINT "[", InKey(), "]"
        "#,
        b"qX",
    );

    assert_eq!(output, "[]");
}

#[test]
fn event_wait_returns_an_empty_event_on_timeout() {
    let output = run_ppl(
        r#"
        TERMINPUT input = Terminal.Input
        EVENT e = input.Wait(1)
        PRINTLN e.Kind
        PRINTLN e.Code
        PRINTLN e.Text
        PRINTLN e.Pressed
        PRINTLN e.Pixels
        "#,
    );

    assert_eq!(output, "0\n0\n\n0\n0\n");
}
