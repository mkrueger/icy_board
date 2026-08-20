use super::run_ppl_with_input;

#[test]
fn mouse_accessors_have_empty_defaults() {
    let output = super::run_ppl(
        r#"
        PrintLn MouseX()
        PrintLn MouseY()
        PrintLn MouseButton()
        PrintLn MouseModifiers()
        "#,
    );
    assert_eq!(output, "0\n0\n0\n0\n");
}

#[test]
fn mouse_modifiers_report_shift_and_ctrl_bits() {
    let output = run_ppl_with_input(
        r"
        MouseOn 0
        PrintLn MousePoll()
        PrintLn MouseModifiers()
        MouseOff
        ",
        b"\x1b[<20;11;6M",
    );
    assert!(output.contains("1\n5\n"), "{output:?}");
}

#[test]
fn text_mouse_reports_cells_wheel_modifiers_and_preserves_keys() {
    let output = run_ppl_with_input(
        r"
        MouseOn 0
        PrintLn MousePoll()
        PrintLn MouseX()
        PrintLn MouseY()
        PrintLn MouseButton()
        PrintLn MouseModifiers()
        PrintLn MousePoll()
        PrintLn MouseX()
        PrintLn MouseY()
        PrintLn MouseButton()
        PrintLn InKey()
        MouseOff
        ",
        b"\x1b[<20;11;6M\x1b[<65;12;7M\x1b[D",
    );

    assert!(output.starts_with("\x1b[?1000l\x1b[?1002l\x1b[?1003h\x1b[?1006h\x1b[?1016l"));
    assert!(output.contains("1\n10\n5\n0\n5\n4\n11\n6\n4\nLEFT\n"), "{output:?}");
    assert!(output.ends_with("\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1006l\x1b[?1016l"));
}

#[test]
fn graphics_mouse_reports_pixels() {
    let output = run_ppl_with_input(
        r"
        MouseOn 1
        PrintLn MousePoll()
        PrintLn MouseX()
        PrintLn MouseY()
        PrintLn MousePixels()
        MouseOff
        ",
        b"\x1b[?1016;1$y\x1b[<0;101;51M",
    );

    assert!(output.starts_with("\x1b[?1000l\x1b[?1002l\x1b[?1003h\x1b[?1006h\x1b[?1016h\x1b[?1016$p"));
    assert!(output.contains("1\n100\n50\n1\n"), "{output:?}");
}

#[test]
fn paint_demo_uses_pixel_mouse_and_preserves_quit_key() {
    let source = include_str!("../../../../../ppe/paint/src/paint.pps");
    let output = super::run_ppl_with_files_and_input(
        source,
        &[("paint_ui.png", include_bytes!("../../../../../ppe/paint/paint_ui.png"))],
        b"\x1b[?1016;1$yq",
    );

    assert!(output.contains("Starting PPE Paint..."));
    assert!(output.contains("\x1bP"));
    assert!(output.contains("\x1b[?1016h"));
    assert!(output.ends_with("\x1b[?1070h\x1b[?7h\x1b[?25h\x1b[2J\x1b[H"));
}

#[test]
fn paint_demo_batches_queued_motion_into_one_present() {
    let source = include_str!("../../../../../ppe/paint/src/paint.pps");
    let output = super::run_ppl_with_files_and_input(
        source,
        &[("paint_ui.png", include_bytes!("../../../../../ppe/paint/paint_ui.png"))],
        b"\x1b[?1016;1$y\x1b[<0;101;101M\x1b[<32;111;106M\x1b[<32;121;111M\x1b[<32;131;116M\x1b[<32;141;121M\x1b[<0;141;121mq",
    );

    assert_eq!(output.matches("\x1bP").count(), 2, "{output:?}");
}
