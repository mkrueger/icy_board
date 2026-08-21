use super::{compile_errors_with_runtime, run_ppl, run_ppl_with_input};

#[test]
fn event_api_requires_runtime_402() {
    for runtime in [400, 401] {
        let errors = compile_errors_with_runtime("EVENT e = EventPoll()\ne = EventWait(1)", runtime);
        assert!(
            errors.iter().any(|error| error == "EventPoll needs runtime 402"),
            "runtime {runtime}: {errors:?}"
        );
        assert!(
            errors.iter().any(|error| error == "EventWait needs runtime 402"),
            "runtime {runtime}: {errors:?}"
        );
    }
    assert!(compile_errors_with_runtime("EVENT e = EventPoll()\ne = EventWait(1)", 402).is_empty());
}

#[test]
fn unified_events_consume_character_key_edge_and_mouse_input() {
    let output = run_ppl_with_input(
        r#"
        EVENT e
        KeyEvents KEY_EVENTS_ON
        MouseOn MOUSE_TEXT

        e = EventPoll()
        PRINTLN e.Kind, ":", e.Code, ":", e.Text, ":", e.Pressed
        e = EventPoll()
        PRINTLN e.Kind, ":", e.Code, ":", e.Text, ":", e.Pressed
        e = EventPoll()
        PRINTLN e.Kind, ":", e.Code, ":", e.X, ":", e.Y, ":", e.Button, ":", e.Modifiers, ":", e.Pixels
        e = EventPoll()
        PRINTLN e.Kind, ":", e.Code, ":", e.Text, ":", e.Pressed, ":", e.X, ":", e.Y

        MouseOff
        KeyEvents KEY_EVENTS_OFF
        "#,
        b"x\x1b[=30K\x1b[<20;11;6M",
    );

    assert!(output.contains("1:120:x:1\n2:30::1\n3:1:10:5:0:5:0\n0:0::0:0:0\n"), "{output:?}");
}

#[test]
fn unified_events_preserve_wire_order_between_sources() {
    let output = run_ppl_with_input(
        r#"
        EVENT e
        KeyEvents KEY_EVENTS_ON
        MouseOn MOUSE_TEXT
        e = EventPoll()
        PRINTLN e.Kind, ":", e.Code
        e = EventPoll()
        PRINTLN e.Kind, ":", e.Text
        e = EventPoll()
        PRINTLN e.Kind, ":", e.Code
        MouseOff
        KeyEvents KEY_EVENTS_OFF
        "#,
        b"\x1b[<0;3;4Mx\x1b[=31K",
    );

    assert!(output.contains("3:1\n1:x\n2:31\n"), "{output:?}");
}

#[test]
fn event_objects_keep_independent_snapshots() {
    let output = run_ppl_with_input(
        r#"
        EVENT first = EventPoll()
        EVENT second = EventPoll()
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
        EVENT e = EventPoll()
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
        EVENT e
        MouseOn MOUSE_TEXT
        e = EventPoll()
        PRINTLN e.Buttons, ":", e.WheelX, ":", e.WheelY
        e = EventPoll()
        PRINTLN e.Buttons, ":", e.Button, ":", e.WheelX, ":", e.WheelY
        e = EventPoll()
        PRINTLN e.Buttons
        MouseOff
        "#,
        b"\x1b[<0;1;1M\x1b[<66;1;1M\x1b[<0;1;1m",
    );
    assert!(output.contains("1:0:0\n1:5:-1:0\n0\n"), "{output:?}");
}

#[test]
fn negative_event_wait_means_indefinite() {
    let output = run_ppl_with_input(
        r#"
        EVENT e = EventWait(-1)
        PRINT e.Text
        "#,
        b"q",
    );
    assert_eq!(output, "q");
}

#[test]
fn event_wait_returns_an_empty_event_on_timeout() {
    let output = run_ppl(
        r#"
        EVENT e = EventWait(1)
        PRINTLN e.Kind
        PRINTLN e.Code
        PRINTLN e.Text
        PRINTLN e.Pressed
        PRINTLN e.Pixels
        "#,
    );

    assert_eq!(output, "0\n0\n\n0\n0\n");
}
