use crate::vm::tests::{compile_errors, compile_errors_with_runtime, run_ppl};

#[test]
fn callback_rank_syntax_survives_visitors_output_and_formatting() {
    use std::{
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    use crate::{
        ast::{AstVisitorMut, OutputVisitor},
        compiler::workspace::Workspace,
        formatting::{FormattingOptions, FormattingVisitor, StringFormattingBackend},
        parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    };

    struct Identity;
    impl AstVisitorMut for Identity {}

    let parse = |source: &str| {
        let errors = Arc::new(Mutex::new(ErrorReporter::default()));
        let ast = parse_ast(
            PathBuf::from("callback_ranks.pps"),
            errors.clone(),
            source,
            &UserTypeRegistry::icy_board_registry(),
            Encoding::Utf8,
            &Workspace::default(),
        );
        let reporter = errors.lock().unwrap();
        assert!(
            !reporter.has_errors(),
            "{source}\n{:?}",
            reporter.errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>()
        );
        ast
    };

    for rank in ["", "[]", "[,]", "[,,]"] {
        let signature = format!("FUNCTION outer(PROCEDURE inner(FUNCTION leaf(INTEGER n, STRING label) INTEGER{rank})) INTEGER{rank}");
        let source = format!(";$LANGVERSION 400\nDECLARE PROCEDURE WORK({signature}, INTEGER n)\nPROCEDURE WORK({signature}, INTEGER n)\nENDPROC\n");
        let ast = parse(&source);
        let cloned = ast.visit_mut(&mut Identity);
        assert_eq!(ast.nodes, cloned.nodes);
        // Keep the source's language directive rather than emitting a second one.
        let mut output = OutputVisitor::default();
        cloned.visit(&mut output);
        assert!(output.output.contains(&signature), "{}", output.output);
        let reparsed = parse(&output.output);
        assert_eq!(cloned.nodes.len(), reparsed.nodes.len());
        for (before, after) in cloned.nodes.iter().zip(&reparsed.nodes) {
            assert!(before.is_similar(after), "{}", output.output);
        }
        let errors = compile_errors(&output.output);
        assert!(errors.is_empty(), "{errors:?}");

        let format = |source: &str| {
            let ast = parse(source);
            let mut backend = StringFormattingBackend::new(source);
            FormattingVisitor::new(&mut backend, &FormattingOptions::default()).format(&ast);
            backend.apply()
        };
        let formatted = format(&source.replace(", STRING", ",STRING").replace(", INTEGER", ",INTEGER"));
        assert!(formatted.contains(&signature), "{formatted}");
        assert_eq!(formatted, format(&formatted));
        assert!(compile_errors(&formatted).is_empty());
    }

    // Similarity must not erase a rank difference at any nesting depth.
    let signature = |rank| {
        parse(&format!(
            ";$LANGVERSION 400\nPROCEDURE WORK(PROCEDURE outer(FUNCTION leaf() INTEGER{rank}))\nENDPROC\n"
        ))
    };
    for expected in ["", "[]", "[,]", "[,,]"] {
        for actual in ["", "[]", "[,]", "[,,]"] {
            assert_eq!(
                expected == actual,
                signature(expected).nodes.last().unwrap().is_similar(signature(actual).nodes.last().unwrap())
            );
        }
    }
}

#[test]
fn nested_array_callbacks_can_execute_through_function_parameters() {
    for (rank, bounds) in [("[]", "1"), ("[,]", "1, 2"), ("[,,]", "1, 2, 3")] {
        let source = format!(
            r#";$LANGVERSION 400
DECLARE FUNCTION Invoke(FUNCTION callback() INTEGER{rank}) INTEGER{rank}
Relay(Invoke, Make)
PROCEDURE Relay(FUNCTION factory(FUNCTION callback() INTEGER{rank}) INTEGER{rank}, FUNCTION source() INTEGER{rank})
    INTEGER values{rank} = factory(source)
    PRINT values[{bounds}]
ENDPROC
FUNCTION Invoke(FUNCTION callback() INTEGER{rank}) INTEGER{rank}
    RETURN callback()
ENDFUNC
FUNCTION Make() INTEGER{rank}
    INTEGER result[{bounds}]
    result[{bounds}] = 17
    RETURN result
ENDFUNC
"#
        );
        assert_eq!("17", run_ppl(&source), "rank {rank}");
    }
}

#[test]
fn callback_return_ranks_are_checked_for_direct_and_forwarded_arguments() {
    for expected in ["", "[]", "[,]", "[,,]"] {
        for actual in ["", "[]", "[,]", "[,,]"] {
            for forwarded in [false, true] {
                let call = if forwarded { "Relay(Make)" } else { "Apply(Make)" };
                let relay = if forwarded {
                    format!("PROCEDURE Relay(FUNCTION source() INTEGER{actual})\n    Apply(source)\nENDPROC\n")
                } else {
                    String::new()
                };
                let source = format!(
                    ";$LANGVERSION 400\nDECLARE FUNCTION Make() INTEGER{actual}\n{call}\n\
                     PROCEDURE Apply(FUNCTION callback() INTEGER{expected})\nENDPROC\n\
                     {relay}FUNCTION Make() INTEGER{actual}\nENDFUNC\n"
                );
                let errors = compile_errors(&source);
                if expected == actual {
                    assert!(errors.is_empty(), "{source}\n{errors:?}");
                } else {
                    assert!(errors.iter().any(|error| error.contains("parameters not match")), "{source}\n{errors:?}");
                }
            }
        }
    }
}

#[test]
fn array_callbacks_can_be_called_and_forwarded_at_every_rank() {
    for (rank, bounds) in [("[]", "1"), ("[,]", "1, 2"), ("[,,]", "1, 2, 3")] {
        let source = format!(
            r#";$LANGVERSION 400
Relay(Make)
PROCEDURE Relay(FUNCTION source() INTEGER{rank})
    Apply(source)
ENDPROC
PROCEDURE Apply(FUNCTION callback() INTEGER{rank})
    INTEGER values{rank} = callback()
    PRINT values[{bounds}]
ENDPROC
FUNCTION Make() INTEGER{rank}
    INTEGER result[{bounds}]
    result[{bounds}] = 42
    RETURN result
ENDFUNC
"#
        );
        assert_eq!("42", run_ppl(&source), "rank {rank}");
    }
}

#[test]
fn nested_callback_return_ranks_are_checked_recursively() {
    for expected in ["", "[]", "[,]", "[,,]"] {
        for actual in ["", "[]", "[,]", "[,,]"] {
            // A function nested inside a procedure nested inside a function.
            let expected_signature = format!("FUNCTION outer(PROCEDURE inner(FUNCTION leaf() INTEGER{expected})) INTEGER");
            let actual_signature = format!("FUNCTION outer(PROCEDURE inner(FUNCTION leaf() INTEGER{actual})) INTEGER");
            for forwarded in [false, true] {
                let call = if forwarded { "Relay(Work)" } else { "Apply(Work)" };
                let relay = if forwarded {
                    format!("PROCEDURE Relay(PROCEDURE callback({actual_signature}))\n    Apply(callback)\nENDPROC\n")
                } else {
                    String::new()
                };
                let source = format!(
                    ";$LANGVERSION 400\n{call}\n\
                     PROCEDURE Apply(PROCEDURE callback({expected_signature}))\nENDPROC\n\
                     {relay}PROCEDURE Work({actual_signature})\nENDPROC\n"
                );
                let errors = compile_errors(&source);
                if expected == actual {
                    assert!(errors.is_empty(), "{source}\n{errors:?}");
                } else {
                    assert!(errors.iter().any(|error| error.contains("parameters not match")), "{source}\n{errors:?}");
                }
            }
        }
    }
}

#[test]
fn declarations_check_callback_return_ranks_recursively() {
    for (kind, result, end) in [("PROCEDURE", "", "ENDPROC"), ("FUNCTION", " INTEGER", "ENDFUNC")] {
        for expected in ["", "[]", "[,]", "[,,]"] {
            for actual in ["", "[]", "[,]", "[,,]"] {
                for nested in [false, true] {
                    let signature = |rank| {
                        let leaf = format!("FUNCTION leaf() INTEGER{rank}");
                        if nested {
                            format!("FUNCTION outer(PROCEDURE inner({leaf})) INTEGER")
                        } else {
                            leaf
                        }
                    };
                    let source = format!(
                        ";$LANGVERSION 400\nDECLARE {kind} Work({}){result}\n{kind} Work({}){result}\n{end}\n",
                        signature(expected),
                        signature(actual)
                    );
                    let errors = compile_errors(&source);
                    if expected == actual {
                        assert!(errors.is_empty(), "{source}\n{errors:?}");
                    } else {
                        assert!(errors.iter().any(|error| error.contains("parameters not match")), "{source}\n{errors:?}");
                    }
                }
            }
        }
    }
}

/// A routine reference stores its parameter count where an array keeps its bounds,
/// so it must not be mistaken for one.
#[test]
fn a_routine_parameter_is_not_an_array() {
    let errors = compile_errors("PROCEDURE Apply(FUNCTION callback(INTEGER value) INTEGER)\n    PRINT callback + 1\nENDPROC\n");
    assert_eq!(vec!["Function used as variable (callback)"], errors);
}

#[test]
fn a_function_can_be_passed_and_called() {
    assert_eq!(
        "42",
        run_ppl(
            r"
PrintWith(Twice)

PROCEDURE PrintWith(FUNCTION callback(INTEGER value) INTEGER)
    PRINT callback(21)
ENDPROC

FUNCTION Twice(INTEGER value) INTEGER
    RETURN value * 2
ENDFUNC
",
        )
    );
}

#[test]
fn a_procedure_parameter_keeps_var_semantics() {
    assert_eq!(
        "5",
        run_ppl(
            r"
Apply(Increment)

PROCEDURE Apply(PROCEDURE callback(VAR INTEGER value))
    INTEGER number = 4
    callback(number)
    PRINT number
ENDPROC

PROCEDURE Increment(VAR INTEGER value)
    value = value + 1
ENDPROC
",
        )
    );
}

#[test]
fn by_value_parameters_beyond_the_var_mask_do_not_overflow() {
    assert_eq!(
        "42",
        run_ppl(
            r"
INTEGER value = 1
Change(value, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16)
PRINT value

PROCEDURE Change(VAR INTEGER target, INTEGER p1, INTEGER p2, INTEGER p3, INTEGER p4, INTEGER p5, INTEGER p6, INTEGER p7, INTEGER p8, INTEGER p9, INTEGER p10, INTEGER p11, INTEGER p12, INTEGER p13, INTEGER p14, INTEGER p15, INTEGER p16)
    target = 42
ENDPROC
",
        )
    );
}

#[test]
fn a_routine_parameter_can_be_forwarded() {
    assert_eq!(
        "7",
        run_ppl(
            r"
Relay(PrintValue)

PROCEDURE Relay(PROCEDURE callback(INTEGER value))
    Invoke(callback)
ENDPROC

PROCEDURE Invoke(PROCEDURE callback(INTEGER value))
    callback(7)
ENDPROC

PROCEDURE PrintValue(INTEGER value)
    PRINT value
ENDPROC
",
        )
    );
}

#[test]
fn a_routine_argument_must_have_the_declared_signature() {
    let errors = compile_errors(
        r"
Apply(Wrong)

PROCEDURE Apply(PROCEDURE callback(INTEGER value))
ENDPROC

PROCEDURE Wrong(STRING value)
ENDPROC
",
    );
    assert!(errors.iter().any(|error| error.contains("parameters not match")), "{errors:?}");
}

#[test]
fn a_procedure_call_reports_excess_arguments() {
    let errors = compile_errors(
        r"
Show(1, 2)

PROCEDURE Show(INTEGER value)
ENDPROC
",
    );
    assert!(errors.iter().any(|error| error.contains("Too many arguments passed (Show:2:1)")), "{errors:?}");
}

#[test]
fn a_bare_routine_name_is_still_not_a_general_value() {
    let errors = compile_errors(
        r"
PRINT Work

PROCEDURE Work()
ENDPROC
",
    );
    assert!(errors.iter().any(|error| error == "Function used as variable (Work)"), "{errors:?}");
}

#[test]
fn a_function_name_is_not_its_return_value_outside_that_function() {
    let errors = compile_errors(
        r"
PRINT Work

FUNCTION Work() INTEGER
    Work = 1
ENDFUNC
EXIT
",
    );
    assert!(errors.iter().any(|error| error == "Function used as variable (Work)"), "{errors:?}");
}

#[test]
fn passing_a_routine_needs_runtime_400() {
    let errors = compile_errors_with_runtime(
        r"
Apply(Work)

PROCEDURE Apply(PROCEDURE callback())
ENDPROC

PROCEDURE Work()
ENDPROC
",
        340,
    );
    assert!(
        errors.iter().any(|error| error == "Passing a FUNCTION/PROCEDURE needs runtime 400"),
        "{errors:?}"
    );
}
