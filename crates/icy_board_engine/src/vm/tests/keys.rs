use super::run_ppl_with_input;

#[test]
fn physical_key_edges_are_polled_without_eating_normal_input() {
    let output = run_ppl_with_input(
        r"
        KeyEvents KEY_EVENTS_ON
        PrintLn KeyPoll()
        PrintLn KeyCode()
        PrintLn KeyPressed()
        PrintLn KeyPoll()
        PrintLn KeyPressed()
        PrintLn InKey()
        KeyEvents KEY_EVENTS_OFF
        ",
        b"x\x1b[=30K\x1b[=30k",
    );

    assert!(output.contains("1\n30\n1\n1\n0\nx\n"), "{output:?}");
    assert!(output.contains("\x1b[=2l\x1b[=1h"), "{output:?}");
    assert!(output.ends_with("\x1b[=2l\x1b[=1l"), "{output:?}");
}
