use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    compiler::{PPECompiler, workspace::Workspace},
    executable::{Executable, ExecutableError},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};

fn compile(source: &str, language: u16, runtime: u16, optimize: bool) -> Executable {
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let registry = UserTypeRegistry::icy_board_registry();
    let mut workspace = Workspace::default();
    workspace.hard_coded_files = Some(vec![PathBuf::from("s2.pps")]);
    workspace.package.runtime = Some(runtime);
    workspace.set_default_language_version(Some(language));
    let source = source.replace("\nSTOP\n", if language < 400 { "\nEND\n" } else { "\nEXIT\n" });
    let ast = parse_ast(PathBuf::from("s2.pps"), errors.clone(), &source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone()).with_optimization(optimize);
    compiler.compile(&[&ast]);
    let reporter = errors.lock().unwrap();
    assert!(
        !reporter.has_errors(),
        "{}",
        reporter.errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>().join("\n")
    );
    drop(reporter);
    compiler.create_executable().unwrap()
}

fn output(executable: Executable) -> String {
    let executable = if executable.in_memory_script.is_some() {
        assert_eq!(executable.to_buffer().unwrap_err(), ExecutableError::UnsupportedShortCircuitEncoding);
        executable
    } else {
        Executable::from_buffer(&mut executable.to_buffer().unwrap(), false).unwrap()
    };
    let (success, text) = super::run_executable_collecting(executable, |_| {}, &[], None, &[], false, false);
    assert!(success, "{text}");
    text.replace('\r', "")
}

#[test]
fn s2_side_effect_matrix_across_language_runtime_and_optimizer() {
    for (language, runtime) in [(340, 340), (340, 400), (400, 400)] {
        for optimize in [false, true] {
            for operator in ["&", "&&", "|", "||"] {
                for left in [false, true] {
                    for right in [false, true] {
                        let source = format!(
                            r#"
DECLARE FUNCTION Witness(STRING markText, BOOLEAN answerValue) BOOLEAN
STRING traceText
BOOLEAN answer
traceText = ""
answer = Witness("L", {}) {operator} Witness("R", {})
PRINTLN answer, ":", traceText
STOP
FUNCTION Witness(STRING markText, BOOLEAN answerValue) BOOLEAN
    traceText = traceText + markText
    Witness = answerValue
ENDFUNC
"#,
                            i32::from(left),
                            i32::from(right)
                        );
                        let is_and = operator.starts_with('&');
                        let skipped = language >= 400 && operator.len() == 2 && left != is_and;
                        let result = if is_and { left && right } else { left || right };
                        let expected = format!("{}:{}\n", i32::from(result), if skipped { "L" } else { "LR" });
                        assert_eq!(
                            output(compile(&source, language, runtime, optimize)),
                            expected,
                            "{language}/{runtime}, opt={optimize}: {source}"
                        );
                    }
                }
            }
        }
    }
}

#[test]
fn s2_original_precedence_and_scalar_conversions() {
    let source = r#"
PRINTLN TRUE | FALSE & FALSE
PRINTLN TRUE || FALSE && FALSE
PRINTLN (TRUE | FALSE) & FALSE
PRINTLN !1 = 2
PRINTLN (!1) = 2
PRINTLN 2 & 4
PRINTLN 2 | 4
PRINTLN AND(2, 4)
PRINTLN OR(2, 4)
PRINTLN -2^2
PRINTLN 2^3^2
"#;
    for language in [340, 400] {
        for optimize in [false, true] {
            assert_eq!(output(compile(source, language, language, optimize)), "1\n1\n0\n1\n0\n1\n1\n0\n6\n4\n64\n");
        }
    }
}

#[test]
fn s2_short_circuit_in_arguments_loops_indices_and_negation() {
    let source = r#"
DECLARE FUNCTION Witness(STRING markText, BOOLEAN answerValue) BOOLEAN
DECLARE FUNCTION Pair(BOOLEAN firstValue, BOOLEAN secondValue) BOOLEAN
STRING traceText
BOOLEAN answer
INTEGER count
INTEGER values(1)
traceText = ""
answer = Pair(FALSE && Witness("X", TRUE), TRUE || Witness("Y", FALSE))
PRINTLN answer, ":", traceText
answer = Witness("A", TRUE) && (Witness("B", FALSE) || Witness("C", TRUE))
PRINTLN answer, ":", traceText
traceText = ""
WHILE count < 2 && Witness("W", TRUE) DO
    count = count + 1
ENDWHILE
PRINTLN count, ":", traceText
traceText = ""
IF !(Witness("N", FALSE) && Witness("X", TRUE)) THEN
    PRINTLN traceText
ENDIF
values(FALSE && Witness("X", TRUE)) = 7
PRINTLN values(0), ":", traceText
PRINTLN FALSE && (1 / 0 > 0)
PRINTLN TRUE || (1 / 0 > 0)
STOP
FUNCTION Witness(STRING markText, BOOLEAN answerValue) BOOLEAN
    traceText = traceText + markText
    Witness = answerValue
ENDFUNC
FUNCTION Pair(BOOLEAN firstValue, BOOLEAN secondValue) BOOLEAN
    traceText = traceText + "P"
    Pair = firstValue | secondValue
ENDFUNC
"#;
    for optimize in [false, true] {
        assert_eq!(output(compile(source, 400, 400, optimize)), "1:P\n1:PABC\n2:WW\nN\n7:N\n0\n1\n");
    }
}

#[test]
fn s2_skipped_operands_are_still_checked() {
    assert!(!super::compile_errors("PRINTLN FALSE && UnknownFunction()\n").is_empty());
    assert!(!super::compile_errors("CONST BOOLEAN result = FALSE && UnknownFunction()\nPRINTLN result\n").is_empty());
    assert!(!super::compile_errors_with_runtime("BOOLEAN condition\nPRINTLN condition && TRUE\n", 340).is_empty());
    assert!(!super::compile_errors("PRINTLN FALSE && GfxBackend.Sixel\n").is_empty());
    assert!(!super::compile_errors("CONST BOOLEAN result = FALSE && GfxBackend.Sixel\nPRINTLN result\n").is_empty());
}

#[test]
fn s2_decompiler_roundtrip_preserves_operator_effects_and_grouping() {
    let source = r#"
DECLARE FUNCTION Witness(STRING markText, BOOLEAN answerValue) BOOLEAN
STRING traceText
BOOLEAN answer
traceText = ""
answer = Witness("A", TRUE) || Witness("B", FALSE) && Witness("C", FALSE)
PRINTLN answer, ":", traceText
traceText = ""
answer = (Witness("A", TRUE) || Witness("B", FALSE)) && Witness("C", FALSE)
PRINTLN answer, ":", traceText
traceText = ""
answer = Witness("A", TRUE) | Witness("B", FALSE) & Witness("C", FALSE)
PRINTLN answer, ":", traceText
PRINTLN (!1) = 2, ":", !1 = 2
IF !(Witness("D", FALSE) && Witness("E", TRUE)) THEN
    PRINTLN traceText
ENDIF
STOP
FUNCTION Witness(STRING markText, BOOLEAN answerValue) BOOLEAN
    traceText = traceText + markText
    Witness = answerValue
ENDFUNC
"#;
    let expected = "1:A\n0:AC\n1:ABC\n0:1\nABCD\n";
    for optimize in [false, true] {
        let executable = compile(source, 400, 400, optimize);
        assert_eq!(output(executable.clone()), expected);
        for raw in [false, true] {
            let (ast, issues) = crate::decompiler::decompile(executable.clone(), raw, 400).unwrap();
            assert!(issues.is_empty());
            let mut visitor = crate::ast::output_visitor::OutputVisitor::default();
            visitor.version = 400;
            ast.visit(&mut visitor);
            assert!(visitor.output.contains("&&") && visitor.output.contains("||"), "{}", visitor.output);
            assert_eq!(output(compile(&visitor.output, 400, 400, optimize)), expected, "{}", visitor.output);
        }
    }
}

#[test]
fn s2_skipped_calls_do_not_set_error_state() {
    let source = r#"
DECLARE FUNCTION FailingRead() BOOLEAN
BOOLEAN answer
Error.Clear()
answer = FALSE && FailingRead()
PRINTLN answer, ":", Error.Last().OK
answer = TRUE || FailingRead()
PRINTLN answer, ":", Error.Last().OK
answer = FALSE & FailingRead()
PRINTLN answer, ":", Error.Last().OK
STOP
FUNCTION FailingRead() BOOLEAN
    Terminal.LoadFont(43, "s2-missing-font.fnt")
    FailingRead = TRUE
ENDFUNC
"#;
    for optimize in [false, true] {
        assert_eq!(output(compile(source, 400, 400, optimize)), "0:1\n1:1\n0:0\n");
    }
}
