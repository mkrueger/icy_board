use super::{compile_errors_with_runtime, run_ppl, run_ppl_with_files_and_input};

#[test]
fn the_error_api_requires_runtime_400() {
    for runtime in [330, 340] {
        let errors = compile_errors_with_runtime("PrintLn Error.Last().Code\nError.Clear()", runtime);
        assert!(
            errors.iter().any(|error| error.contains("Error.Last needs runtime 400")),
            "runtime {runtime}: {errors:?}"
        );
    }
    assert!(compile_errors_with_runtime("PrintLn Error.Last().Code\nError.Clear()", 400).is_empty());
}

#[test]
fn nothing_has_gone_wrong_yet() {
    let output =
        run_ppl(r#"PrintLn Error.Last().OK, " ", Error.Last().Kind, " ", Error.Last().Code, " ", Error.Last().Channel, " [", Error.Last().Message, "]""#);

    assert_eq!(output, "1 0 0 -1 []\n");
}

#[test]
fn err_can_still_be_a_variable_name() {
    let output = run_ppl(
        r#"
        INTEGER err = 42
        PrintLn err, " ", Error.Last().OK
        "#,
    );

    assert_eq!(output, "42 1\n");
}

#[test]
fn a_failed_operation_names_its_subsystem() {
    let output = run_ppl(
        r#"
        Terminal.LoadFont(43, "nope.fnt")
        PrintLn Error.Last().OK, " ", Error.Last().Kind, " ", Error.Last().Code
        "#,
    );

    // ERR_KIND_FONT, ERR_IO
    assert_eq!(output, "0 5 3\n");
}

#[test]
fn constants_name_the_codes() {
    let output = run_ppl(
        r#"
        Terminal.LoadFont(43, "nope.fnt")
        IF (Error.Last().Kind = ErrKind.Font & Error.Last().Code = ErrCode.Io) PrintLn "matched"
        "#,
    );

    assert_eq!(output, "matched\n");
}

#[test]
fn a_later_success_clears_the_error() {
    let output = run_ppl(
        r#"
        Terminal.LoadFont(43, "nope.fnt")
        PrintLn Error.Last().OK
        Terminal.SetFont(5, 0)
        PrintLn Error.Last().OK
        "#,
    );

    assert_eq!(output, "0\n\x1b[0;5 D1\n");
}

#[test]
fn clearing_forgets_the_error() {
    let output = run_ppl(
        r#"
        Terminal.LoadFont(43, "nope.fnt")
        PrintLn Error.Last().OK
        Error.Clear()
        PrintLn Error.Last().OK, " ", Error.Last().Kind
        "#,
    );

    assert!(output.ends_with("0\n1 0\n"), "{output:?}");
}

#[test]
fn the_error_can_be_kept_while_work_carries_on() {
    let output = run_ppl(
        r#"
        ERROR saved
        Terminal.LoadFont(43, "nope.fnt")
        saved = Error.Last()
        Terminal.SetFont(5, 0)
        PrintLn "now=", Error.Last().Code, " saved=", saved.Code
        "#,
    );

    assert_eq!(output, "\x1b[0;5 Dnow=0 saved=3\n");
}

#[test]
fn a_message_describes_what_happened() {
    let output = run_ppl(
        r#"
        Terminal.LoadFont(43, "nope.fnt")
        PrintLn Error.Last().Message <> ""
        "#,
    );

    assert_eq!(output, "1\n");
}

#[test]
fn on_error_goto_jumps_and_stays_there() {
    let output = run_ppl(
        r#"
        ON ERROR GOTO Failed
        PrintLn "before"
        Terminal.LoadFont(43, "nope.fnt")
        PrintLn "not reached"
        EXIT
        :Failed
        PrintLn "handled ", Error.Last().Code
        "#,
    );

    assert_eq!(output, "before\nhandled 3\n");
}

#[test]
fn on_error_goto_is_disarmed_before_cleanup() {
    let output = run_ppl(
        r#"
        ON ERROR GOTO Failed
        Terminal.LoadFont(43, "first-missing.fnt")
        EXIT
        :Failed
        PrintLn "handled"
        Terminal.LoadFont(43, "second-missing.fnt")
        PrintLn "done"
        "#,
    );

    assert_eq!(output, "handled\ndone\n");
}

#[test]
fn a_goto_handler_can_arm_another_handler() {
    let output = run_ppl(
        r#"
        ON ERROR GOTO Failed
        Terminal.LoadFont(43, "first-missing.fnt")
        EXIT
        :Failed
        ON ERROR GOSUB Report
        Terminal.LoadFont(43, "second-missing.fnt")
        PrintLn "done"
        EXIT
        :Report
        PrintLn "reported"
        RETURN
        "#,
    );

    assert_eq!(output, "reported\ndone\n");
}

#[test]
fn onerror_is_the_same_as_on_error() {
    let one_word = run_ppl(
        r#"
        ONERROR GOTO Failed
        Terminal.LoadFont(43, "nope.fnt")
        EXIT
        :Failed
        PrintLn "handled"
        "#,
    );
    let two_words = run_ppl(
        r#"
        ON ERROR GOTO Failed
        Terminal.LoadFont(43, "nope.fnt")
        EXIT
        :Failed
        PrintLn "handled"
        "#,
    );

    assert_eq!(one_word, "handled\n");
    assert_eq!(two_words, one_word);
}

#[test]
fn on_error_gosub_comes_back() {
    let output = run_ppl(
        r#"
        ON ERROR GOSUB Failed
        Terminal.LoadFont(43, "nope.fnt")
        PrintLn "carried on"
        EXIT
        :Failed
        PrintLn "handled ", Error.Last().Code
        RETURN
        "#,
    );

    assert_eq!(output, "handled 3\ncarried on\n");
}

#[test]
fn on_error_calls_a_procedure_with_the_error() {
    let output = run_ppl(
        r#"
        DECLARE PROCEDURE Report(ERROR e)
        ON ERROR Report
        Terminal.LoadFont(43, "nope.fnt")
        PrintLn "carried on"
        EXIT

        PROCEDURE Report(ERROR e)
            PrintLn "handler kind=", e.Kind, " code=", e.Code
        ENDPROC
        "#,
    );

    assert_eq!(output, "handler kind=5 code=3\ncarried on\n");
}

#[test]
fn a_handler_procedure_may_take_no_arguments() {
    let output = run_ppl(
        r#"
        DECLARE PROCEDURE Report()
        ON ERROR Report
        Terminal.LoadFont(43, "nope.fnt")
        EXIT

        PROCEDURE Report()
            PrintLn "handled ", Error.Last().Code
        ENDPROC
        "#,
    );

    assert_eq!(output, "handled 3\n");
}

#[test]
fn a_handler_procedure_cannot_take_a_var_parameter() {
    let errors = compile_errors_with_runtime(
        r"
        DECLARE PROCEDURE Report(VAR ERROR e)
        ON ERROR Report
        PROCEDURE Report(VAR ERROR e)
        ENDPROC
        ",
        400,
    );

    assert!(errors.iter().any(|error| error.contains("ON ERROR handler")), "{errors:?}");
}

#[test]
fn on_error_off_stops_handling() {
    let output = run_ppl(
        r#"
        ON ERROR GOSUB Failed
        Terminal.LoadFont(43, "nope.fnt")
        ON ERROR OFF
        Terminal.LoadFont(43, "nope.fnt")
        PrintLn "done"
        EXIT
        :Failed
        PrintLn "handled"
        RETURN
        "#,
    );

    assert_eq!(output, "handled\ndone\n");
}

#[test]
fn a_failure_inside_the_handler_does_not_call_it_again() {
    let output = run_ppl(
        r#"
        ON ERROR GOSUB Failed
        Terminal.LoadFont(43, "nope.fnt")
        PrintLn "carried on"
        EXIT
        :Failed
        PrintLn "handled"
        Terminal.LoadFont(43, "also-missing.fnt")
        RETURN
        "#,
    );

    assert_eq!(output, "handled\ncarried on\n");
}

#[test]
fn the_handler_runs_again_for_a_later_error() {
    let output = run_ppl(
        r#"
        ON ERROR GOSUB Failed
        Terminal.LoadFont(43, "nope.fnt")
        Terminal.LoadFont(43, "nope.fnt")
        PrintLn "done"
        EXIT
        :Failed
        PrintLn "handled"
        RETURN
        "#,
    );

    assert_eq!(output, "handled\nhandled\ndone\n");
}

#[test]
fn reading_a_file_to_its_end_is_not_an_error() {
    let output = run_ppl_with_files_and_input(
        r#"
        STRING s
        ON ERROR GOTO Failed
        FOPEN 1, "data.txt", O_RD, S_DN
        FGET 1, s
        FGET 1, s
        PrintLn "eof=", FERR(1), " ok=", Error.Last().OK
        FCLOSE 1
        EXIT
        :Failed
        PrintLn "handler should not run"
        "#,
        &[("data.txt", b"one\n")],
        b"",
    );

    assert_eq!(output, "eof=1 ok=1\n");
}

#[test]
fn a_file_that_is_not_there_is_an_error() {
    let output = run_ppl(
        r#"
        FOPEN 1, "nope.txt", O_RD, S_DN
        PrintLn Error.Last().Kind, " ", Error.Last().Code, " ", Error.Last().Channel, " ", FERR(1)
        "#,
    );

    // ERR_KIND_FILE, ERR_IO, channel 1
    assert_eq!(output, "1 3 1 1\n");
}

#[test]
fn a_successful_file_operation_clears_an_older_error() {
    let output = run_ppl_with_files_and_input(
        r#"
        Terminal.LoadFont(43, "missing.fnt")
        FOPEN 1, "present.txt", O_RD, S_DN
        PrintLn Error.Last().OK
        FCLOSE 1
        "#,
        &[("present.txt", b"present\n")],
        b"",
    );

    assert_eq!(output, "1\n");
}

#[test]
fn ferr_clears_itself_but_leaves_the_error() {
    let output = run_ppl(
        r#"
        FOPEN 1, "nope.txt", O_RD, S_DN
        PrintLn FERR(1), " ", FERR(1), " ", Error.Last().Code
        "#,
    );

    assert_eq!(output, "1 0 3\n");
}

#[test]
fn a_dbase_failure_names_its_channel() {
    let output = run_ppl(
        r#"
        DOPEN 0, "nope", 0
        PrintLn Error.Last().Kind, " ", Error.Last().Channel, " ", DERR(0)
        "#,
    );

    // ERR_KIND_DBASE on channel 0
    assert_eq!(output, "2 0 1\n");
}
