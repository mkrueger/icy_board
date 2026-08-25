use super::run_ppl_with_input;

#[test]
fn graphics_mouse_reports_pixels() {
    let output = run_ppl_with_input(
        r#"
        TERMINPUT input = Terminal.Input
        EVENT e
        input.MouseOn(MouseMode.Pixels)
        e = input.Poll()
        PrintLn e.Kind
        PrintLn e.Action
        PrintLn e.X
        PrintLn e.Y
        PrintLn e.Pixels
        input.Release()
        "#,
        b"\x1b[?1016;1$y\x1b[<0;101;51M",
    );

    assert!(output.starts_with("\x1b[?1000l\x1b[?1002l\x1b[?1003l\x1b[?1003h\x1b[?1006h\x1b[?1016h\x1b[?1016$p"));
    assert!(output.contains("3\n1\n100\n50\n1\n"), "{output:?}");
}

#[test]
fn paint_demo_uses_pixel_mouse_and_preserves_quit_key() {
    let source = include_str!("../../../../../ppe/paint/src/paint.pps");
    let output = super::run_ppl_with_files_and_input(
        source,
        &[("paint_ui.png", include_bytes!("../../../../../ppe/paint/paint_ui.png"))],
        b"\x1b[<1;4;7c\x1b[?1016;1$yq",
    );

    assert!(output.contains("Starting PPE Paint..."));
    assert!(output.contains("\x1bP"));
    assert!(output.contains("\x1b[?1016h"));
    assert!(output.ends_with("\x1b[?1070h\x1b[?80h\x1b[?7h\x1b[?25h\x1b[2J\x1b[H"));
}

#[test]
fn paint_demo_batches_queued_motion_into_one_present() {
    let source = include_str!("../../../../../ppe/paint/src/paint.pps");
    let output = super::run_ppl_with_files_and_input(
        source,
        &[("paint_ui.png", include_bytes!("../../../../../ppe/paint/paint_ui.png"))],
        b"\x1b[<1;4;7c\x1b[?1016;1$y\x1b[<0;101;101M\x1b[<32;111;106M\x1b[<32;121;111M\x1b[<32;131;116M\x1b[<32;141;121M\x1b[<0;141;121mq",
    );

    assert_eq!(output.matches("\x1bP").count(), 2, "{output:?}");
}
