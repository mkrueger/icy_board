use super::{compile_errors_with_runtime, run_ppl, run_ppl_with_input};

#[test]
fn terminal_input_requires_runtime_402() {
    for runtime in [400, 401] {
        let errors = compile_errors_with_runtime("TERMINPUT input = TermInput()", runtime);
        assert!(errors.iter().any(|error| error == "TermInput needs runtime 402"), "runtime {runtime}: {errors:?}");
    }
    assert!(compile_errors_with_runtime("TERMINPUT input = TermInput()", 402).is_empty());
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
        assert!(!compile_errors_with_runtime(source, 402).is_empty(), "{source}");
    }
}

#[test]
fn terminal_input_is_a_releasable_singleton() {
    let output = run_ppl(
        r#"
        TERMINPUT first = TermInput()
        TERMINPUT duplicate = TermInput()
        PRINTLN first.Valid, duplicate.Valid, ERR().Code
        first.Free()
        TERMINPUT replacement = TermInput()
        PRINTLN first.Valid, replacement.Valid
        replacement.Free()
        "#,
    );

    assert_eq!(output, "102\n01\n");
}

#[test]
fn terminal_input_polls_translated_keys() {
    let output = run_ppl_with_input(
        r#"
        TERMINPUT input = TermInput()
        EVENT event = input.Poll()
        PRINTLN event.Kind, ":", event.Text
        input.Free()
        "#,
        b"\x1b[A",
    );

    assert_eq!(output, "1:UP\n");
}

#[test]
fn unified_events_consume_character_key_edge_and_mouse_input() {
    let output = run_ppl_with_input(
        r#"
        TERMINPUT input = TermInput()
        EVENT e
        input.KeyboardOn()
        input.MouseOn(MOUSE_TEXT)

        e = input.Poll()
        PRINTLN e.Kind, ":", e.Code, ":", e.Text, ":", e.Pressed
        e = input.Poll()
        PRINTLN e.Kind, ":", e.Code, ":", e.Text, ":", e.Pressed
        e = input.Poll()
        PRINTLN e.Kind, ":", e.Code, ":", e.X, ":", e.Y, ":", e.Button, ":", e.Modifiers, ":", e.Pixels
        e = input.Poll()
        PRINTLN e.Kind, ":", e.Code, ":", e.Text, ":", e.Pressed, ":", e.X, ":", e.Y

        input.Free()
        "#,
        b"x\x1b[=30K\x1b[<20;11;6M",
    );

    assert!(output.contains("1:120:x:1\n2:30::1\n3:1:10:5:0:5:0\n0:0::0:0:0\n"), "{output:?}");
}

#[test]
fn unified_events_preserve_wire_order_between_sources() {
    let output = run_ppl_with_input(
        r#"
        TERMINPUT input = TermInput()
        EVENT e
        input.KeyboardOn()
        input.MouseOn(MOUSE_TEXT)
        e = input.Poll()
        PRINTLN e.Kind, ":", e.Code
        e = input.Poll()
        PRINTLN e.Kind, ":", e.Text
        e = input.Poll()
        PRINTLN e.Kind, ":", e.Code
        input.Free()
        "#,
        b"\x1b[<0;3;4Mx\x1b[=31K",
    );

    assert!(output.contains("3:1\n1:x\n2:31\n"), "{output:?}");
}

#[test]
fn event_objects_keep_independent_snapshots() {
    let output = run_ppl_with_input(
        r#"
        TERMINPUT input = TermInput()
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
        TERMINPUT input = TermInput()
        EVENT e = input.Poll()
        PRINTLN e.Kind, ":", e.Code, ":", e.Text, ":", e.Repeated, ":", e.Modifiers
        "#,
        b"\x1b[1;5A",
    );
    assert_eq!(output, format!("1:{key}:UP:0:4\n", key = 0x11_0001));
}

#[test]
fn mouse_events_report_held_buttons_and_wheel_deltas() {
    let output = run_ppl_with_input(
        r#"
        TERMINPUT input = TermInput()
        EVENT e
        input.MouseOn(MOUSE_TEXT)
        e = input.Poll()
        PRINTLN e.Buttons, ":", e.WheelX, ":", e.WheelY
        e = input.Poll()
        PRINTLN e.Buttons, ":", e.Button, ":", e.WheelX, ":", e.WheelY
        e = input.Poll()
        PRINTLN e.Buttons
        input.Free()
        "#,
        b"\x1b[<0;1;1M\x1b[<66;1;1M\x1b[<0;1;1m",
    );
    assert!(output.contains("1:0:0\n1:5:-1:0\n0\n"), "{output:?}");
}

#[test]
fn negative_event_wait_means_indefinite() {
    let output = run_ppl_with_input(
        r#"
        TERMINPUT input = TermInput()
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
        TERMINPUT input = TermInput()
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
        TERMINPUT input = TermInput()
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
