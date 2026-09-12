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
    let source = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ppe/paint/src/paint.pps")).unwrap();
    let output = run_ppl_with_input(&source, b"\x1b[<1;4;7c\x1b[?1016;1$y\x1b[<0;101;101Mq");

    assert!(output.contains("Paint | Sixel"));
    assert!(output.contains("\x1bP"));
    assert!(output.contains("\x1b[?1016h"));
    assert!(output.rfind("\x1b[?1006l").unwrap() > output.rfind("\x1bP").unwrap());
    assert!(output.ends_with("\x1b[2J\x1b[H"));
}

#[test]
fn paint_demo_batches_queued_motion_into_one_present() {
    let source = std::fs::read_to_string(std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../ppe/paint/src/paint.pps")).unwrap();
    let output = run_ppl_with_input(
        &source,
        b"\x1b[<1;4;7c\x1b[?1016;1$y\x1b[<0;101;101M\x1b[<32;111;106M\x1b[<32;121;111M\x1b[<32;131;116M\x1b[<32;141;121M\x1b[<0;141;121mq",
    );

    assert_eq!(output.matches("\x1bP").count(), 2, "{output:?}");
}
