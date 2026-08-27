use super::run_ppl;

#[test]
fn trig_and_log_functions_match_known_values() {
    let output = run_ppl(
        r#"
        ;$LANGVERSION 400
        DOUBLE pi
        pi = 3.14159265358979
        PrintLn Abs(Sin(0.0)) < 0.0000001
        PrintLn Abs(Cos(0.0) - 1.0) < 0.0000001
        PrintLn Abs(Sin(pi / 2.0) - 1.0) < 0.0000001
        PrintLn Abs(Tan(pi / 4.0) - 1.0) < 0.0001
        PrintLn Abs(Atan(1.0) - pi / 4.0) < 0.0000001
        PrintLn Abs(Log(1.0)) < 0.0000001
        PrintLn Abs(Sqrt(4.0) - 2.0) < 0.0000001
        "#,
    );

    assert_eq!(output, "1\n1\n1\n1\n1\n1\n1\n");
}
