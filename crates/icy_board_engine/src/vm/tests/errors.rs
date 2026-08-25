use super::{compile_errors_with_runtime, run_ppl, run_ppl_with_files_and_input};

#[test]
fn the_error_api_requires_runtime_402() {
    for runtime in [400, 401] {
        let errors = compile_errors_with_runtime("PrintLn ERR().Code\nErrClr", runtime);
        for needed in ["Err needs runtime 402", "ErrClr needs runtime 402"] {
            assert!(errors.iter().any(|error| error == needed), "runtime {runtime}: {errors:?}");
        }
    }
    assert!(compile_errors_with_runtime("PrintLn ERR().Code\nErrClr", 402).is_empty());
}

#[test]
fn nothing_has_gone_wrong_yet() {
    let output = run_ppl(r#"PrintLn ERR().OK, " ", ERR().Kind, " ", ERR().Code, " ", ERR().Channel, " [", ERR().Message, "]""#);

    assert_eq!(output, "1 0 0 -1 []\n");
}

#[test]
fn err_can_still_be_a_variable_name() {
    let output = run_ppl(
        r#"
        INTEGER err = 42
        PrintLn err, " ", ERR().OK
        "#,
    );

    assert_eq!(output, "42 1\n");
}

#[test]
fn a_failed_operation_names_its_subsystem() {
    let output = run_ppl(
        r#"
        Terminal.Font.Load(43, "nope.fnt")
        PrintLn ERR().OK, " ", ERR().Kind, " ", ERR().Code
        "#,
    );

    // ERR_KIND_FONT, ERR_IO
    assert_eq!(output, "0 5 3\n");
}

#[test]
fn constants_name_the_codes() {
    let output = run_ppl(
        r#"
        Terminal.Font.Load(43, "nope.fnt")
        IF (ERR().Kind = ErrKind.Font & ERR().Code = ErrCode.Io) PrintLn "matched"
        "#,
    );

    assert_eq!(output, "matched\n");
}

#[test]
fn a_later_success_clears_the_error() {
    let output = run_ppl(
        r#"
        Terminal.Font.Load(43, "nope.fnt")
        PrintLn ERR().OK
        Terminal.Font.Set(0, 5)
        PrintLn ERR().OK
        "#,
    );

    assert_eq!(output, "0\n\x1b[0;5 D1\n");
}

#[test]
fn errclr_forgets_the_error() {
    let output = run_ppl(
        r#"
        Terminal.Font.Load(43, "nope.fnt")
        PrintLn ERR().OK
        ErrClr
        PrintLn ERR().OK, " ", ERR().Kind
        "#,
    );

    assert!(output.ends_with("0\n1 0\n"), "{output:?}");
}

#[test]
fn the_error_can_be_kept_while_work_carries_on() {
    let output = run_ppl(
        r#"
        ERROR saved
        Terminal.Font.Load(43, "nope.fnt")
        saved = ERR()
        Terminal.Font.Set(0, 5)
        PrintLn "now=", ERR().Code, " saved=", saved.Code
        "#,
    );

    assert_eq!(output, "\x1b[0;5 Dnow=0 saved=3\n");
}

#[test]
fn a_message_describes_what_happened() {
    let output = run_ppl(
        r#"
        Terminal.Font.Load(43, "nope.fnt")
        PrintLn ERR().Message <> ""
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
        Terminal.Font.Load(43, "nope.fnt")
        PrintLn "not reached"
        EXIT
        :Failed
        PrintLn "handled ", ERR().Code
        "#,
    );

    assert_eq!(output, "before\nhandled 3\n");
}

#[test]
fn on_error_goto_is_disarmed_before_cleanup() {
    let output = run_ppl(
        r#"
        ON ERROR GOTO Failed
        Terminal.Font.Load(43, "first-missing.fnt")
        EXIT
        :Failed
        PrintLn "handled"
        Terminal.Font.Load(43, "second-missing.fnt")
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
        Terminal.Font.Load(43, "first-missing.fnt")
        EXIT
        :Failed
        ON ERROR GOSUB Report
        Terminal.Font.Load(43, "second-missing.fnt")
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
        Terminal.Font.Load(43, "nope.fnt")
        EXIT
        :Failed
        PrintLn "handled"
        "#,
    );
    let two_words = run_ppl(
        r#"
        ON ERROR GOTO Failed
        Terminal.Font.Load(43, "nope.fnt")
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
        Terminal.Font.Load(43, "nope.fnt")
        PrintLn "carried on"
        EXIT
        :Failed
        PrintLn "handled ", ERR().Code
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
        Terminal.Font.Load(43, "nope.fnt")
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
        Terminal.Font.Load(43, "nope.fnt")
        EXIT

        PROCEDURE Report()
            PrintLn "handled ", ERR().Code
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
        402,
    );

    assert!(errors.iter().any(|error| error.contains("ON ERROR handler")), "{errors:?}");
}

#[test]
fn on_error_off_stops_handling() {
    let output = run_ppl(
        r#"
        ON ERROR GOSUB Failed
        Terminal.Font.Load(43, "nope.fnt")
        ON ERROR OFF
        Terminal.Font.Load(43, "nope.fnt")
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
        Terminal.Font.Load(43, "nope.fnt")
        PrintLn "carried on"
        EXIT
        :Failed
        PrintLn "handled"
        Terminal.Font.Load(43, "also-missing.fnt")
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
        Terminal.Font.Load(43, "nope.fnt")
        Terminal.Font.Load(43, "nope.fnt")
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
        PrintLn "eof=", FERR(1), " ok=", ERR().OK
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
        PrintLn ERR().Kind, " ", ERR().Code, " ", ERR().Channel, " ", FERR(1)
        "#,
    );

    // ERR_KIND_FILE, ERR_IO, channel 1
    assert_eq!(output, "1 3 1 1\n");
}

#[test]
fn a_successful_file_operation_clears_an_older_error() {
    let output = run_ppl_with_files_and_input(
        r#"
        Terminal.Font.Load(43, "missing.fnt")
        FOPEN 1, "present.txt", O_RD, S_DN
        PrintLn ERR().OK
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
        PrintLn FERR(1), " ", FERR(1), " ", ERR().Code
        "#,
    );

    assert_eq!(output, "1 0 3\n");
}

#[test]
fn a_dbase_failure_names_its_channel() {
    let output = run_ppl(
        r#"
        DOPEN 0, "nope", 0
        PrintLn ERR().Kind, " ", ERR().Channel, " ", DERR(0)
        "#,
    );

    // ERR_KIND_DBASE on channel 0
    assert_eq!(output, "2 0 1\n");
}
