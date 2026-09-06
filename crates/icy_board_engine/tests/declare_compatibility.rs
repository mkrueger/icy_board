//! Authored fixtures checked against the original PPLC 3.40 logs. The golden
//! expectations do not require PPLC, DOSBox, or any proprietary PPE artifacts.
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    executable::{Executable, VarHeader, VariableType},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};

const LEGACY_TARGETS: [(u16, u16); 3] = [(340, 340), (340, 400), (350, 400)];
const PARAMETERS: &str = "FUNCTION/PROCEDURE parameters not match with declaration (Work)";
const RETURN_TYPE: &str = "FUNCTION return type does not match with declaration (Work)";
const KIND: &str = "Procedure used as function";
const VARIABLE: &str = "Argument should be a variable (1)";
const ARRAY_ARGUMENTS: &str = "Not enough arguments passed (value:0:";

#[derive(Clone, Debug, PartialEq)]
struct Shape {
    variable_type: VariableType,
    rank: u8,
    bounds: [usize; 3],
}

impl Shape {
    const fn scalar(variable_type: VariableType) -> Self {
        Self {
            variable_type,
            rank: 0,
            bounds: [0; 3],
        }
    }

    fn header(header: &VarHeader) -> Self {
        Self {
            variable_type: header.variable_type,
            rank: header.dim,
            bounds: [header.vector_size, header.matrix_size, header.cube_size],
        }
    }
}

const INTEGER: Shape = Shape::scalar(VariableType::Integer);
const STRING: Shape = Shape::scalar(VariableType::String);
const VECTOR: Shape = Shape {
    variable_type: VariableType::Integer,
    rank: 1,
    bounds: [3, 0, 0],
};

#[derive(Clone, Debug, PartialEq)]
struct Signature {
    kind: VariableType,
    parameters: Vec<Shape>,
    // Function descriptors contain a result ID, not procedure VAR flags.
    pass_flags: Option<u16>,
    result: Option<Shape>,
}

fn procedure(parameters: &[Shape], pass_flags: u16) -> Signature {
    Signature {
        kind: VariableType::Procedure,
        parameters: parameters.to_vec(),
        pass_flags: Some(pass_flags),
        result: None,
    }
}

fn function(parameters: &[Shape], result: Shape) -> Signature {
    Signature {
        kind: VariableType::Function,
        parameters: parameters.to_vec(),
        pass_flags: None,
        result: Some(result),
    }
}

struct Case {
    name: &'static str,
    source: &'static str,
    legacy: Result<Signature, &'static str>,
    strict_error: Option<&'static str>,
}

fn cases() -> Vec<Case> {
    macro_rules! case {
        ($name:literal, $legacy:expr, $strict:expr) => {
            Case {
                name: $name,
                source: include_str!(concat!("../../../compat/declare/", $name, ".pps")),
                legacy: $legacy,
                strict_error: $strict,
            }
        };
    }
    // Original accepted list: 13 cases. In particular VAR on DECLARE alone
    // does not reject a constant, and declared PROCEDURE/implemented FUNCTION
    // retains PROCEDURE kind but takes VAR modes from the implementation.
    // Rejection strings identify our diagnostics, not PPLC's wording. PPLC
    // rejects `dim` while parsing its two-dimensional implementation parameter.
    vec![
        case!("count", Err(PARAMETERS), Some(PARAMETERS)),
        case!("dim", Err("Missing close ')' found: ,"), Some(PARAMETERS)),
        case!("dim_bound", Err(ARRAY_ARGUMENTS), Some(PARAMETERS)),
        case!("dim_bound_index", Ok(procedure(&[VECTOR], 0)), Some(PARAMETERS)),
        case!("dim_decl_only", Ok(procedure(&[INTEGER], 0)), Some(PARAMETERS)),
        case!("dim_scalar", Err(ARRAY_ARGUMENTS), Some(PARAMETERS)),
        case!("dim_scalar_index", Ok(procedure(&[VECTOR], 0)), Some(PARAMETERS)),
        case!("function_count", Err(PARAMETERS), Some(PARAMETERS)),
        case!("function_type", Ok(function(&[STRING], INTEGER)), Some(PARAMETERS)),
        case!("kind_func", Err(KIND), Some(KIND)),
        case!("kind_proc", Ok(procedure(&[], 0)), Some(KIND)),
        case!(
            "kind_proc_var",
            Ok(procedure(&[INTEGER], 1)),
            Some("VAR parameters are not allowed in functions")
        ),
        case!("param_names", Ok(procedure(&[INTEGER], 0)), None),
        case!("return", Ok(function(&[], STRING)), Some(RETURN_TYPE)),
        case!("return_reverse", Ok(function(&[], INTEGER)), Some(RETURN_TYPE)),
        case!("type", Ok(procedure(&[STRING], 0)), Some(PARAMETERS)),
        case!("var_const_decl", Ok(procedure(&[INTEGER], 0)), Some(PARAMETERS)),
        case!("var_const_impl", Err(VARIABLE), Some(PARAMETERS)),
        case!("var_decl", Ok(procedure(&[INTEGER], 0)), Some(PARAMETERS)),
        case!("var_expression", Err(VARIABLE), Some(PARAMETERS)),
        case!("var_impl", Ok(procedure(&[INTEGER], 1)), Some(PARAMETERS)),
    ]
}

fn compile(sources: &[(&str, &str)], language: u16, runtime: u16) -> Result<Executable, Vec<String>> {
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut workspace = Workspace::default();
    workspace.set_default_language_version(Some(language));
    workspace.package.runtime = Some(runtime);
    let asts = sources
        .iter()
        .map(|(name, source)| parse_ast(PathBuf::from(name), errors.clone(), source, &registry, Encoding::Utf8, &workspace))
        .collect::<Vec<_>>();
    let messages = || errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>();
    let parser_errors = messages();
    if !parser_errors.is_empty() {
        return Err(parser_errors);
    }
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&asts.iter().collect::<Vec<_>>());
    let diagnostics = messages();
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    // Check the on-disk PPE, not just the compiler's transient variable table.
    let executable = compiler.create_executable().map_err(|error| vec![format!("create executable: {error}")])?;
    let mut bytes = executable.to_buffer().map_err(|error| vec![format!("serialize PPE: {error}")])?;
    Executable::from_buffer(&mut bytes, false).map_err(|error| vec![format!("deserialize PPE: {error}")])
}

fn signatures(executable: &Executable) -> Vec<Signature> {
    let entries = executable.variable_table.get_entries();
    entries
        .iter()
        .filter_map(|entry| {
            let (first, count, pass_flags, result) = match entry.header.variable_type {
                VariableType::Procedure => {
                    // The header's type selects the active union member.
                    let descriptor = unsafe { entry.value.data.procedure_value };
                    (descriptor.first_var_id, descriptor.parameters, Some(descriptor.pass_flags), None)
                }
                VariableType::Function => {
                    let descriptor = unsafe { entry.value.data.function_value };
                    let result = &entries[usize::try_from(descriptor.return_var).unwrap() - 1].header;
                    (descriptor.first_var_id, descriptor.parameters, None, Some(Shape::header(result)))
                }
                _ => return None,
            };
            // first_var_id is the ID immediately BEFORE the first parameter:
            // it is therefore already the parameter's zero-based table index.
            let first = usize::try_from(first).unwrap();
            Some(Signature {
                kind: entry.header.variable_type,
                parameters: entries[first..first + usize::from(count)]
                    .iter()
                    .map(|entry| Shape::header(&entry.header))
                    .collect(),
                pass_flags,
                result,
            })
        })
        .collect()
}

fn check_outcome(actual: Result<Executable, Vec<String>>, expected: &Result<Signature, &str>) -> Result<(), String> {
    match (actual, expected) {
        (Ok(executable), Ok(expected)) => {
            let actual = signatures(&executable);
            if actual == [expected.clone()] {
                Ok(())
            } else {
                Err(format!("expected signature {expected:?}, got {actual:?}"))
            }
        }
        (Err(errors), Err(expected)) if errors.iter().any(|error| error.contains(expected)) => Ok(()),
        (Err(errors), _) => Err(format!("expected {expected:?}, got diagnostics {errors:?}")),
        (Ok(executable), Err(expected)) => Err(format!("expected rejection {expected:?}, got signatures {:?}", signatures(&executable))),
    }
}

fn check_legacy(language: u16, runtime: u16) {
    let mut failures = Vec::new();
    for case in cases() {
        if let Err(error) = check_outcome(compile(&[(case.name, case.source)], language, runtime), &case.legacy) {
            failures.push(format!("{}: {error}", case.name));
        }
    }
    assert!(failures.is_empty(), "language {language}, runtime {runtime}:\n{}", failures.join("\n"));
}

#[test]
fn original_340_acceptance_rejection_and_signatures() {
    check_legacy(340, 340);
}

#[test]
fn language_340_keeps_original_contract_on_runtime_400() {
    check_legacy(340, 400);
}

#[test]
fn language_350_keeps_legacy_contract_on_runtime_400() {
    check_legacy(350, 400);
}

#[test]
fn strict_language_checks_every_authored_signature_mismatch() {
    let mut failures = Vec::new();
    // Strictness follows source language, not the target PPE format. Do not
    // infer support for any earlier dialect from these 3.40-authored fixtures.
    for runtime in [340, 400] {
        for case in cases() {
            // Standalone END is legacy syntax; EXIT is its strict counterpart.
            let source = case.source.replace("\nEND\n", "\nEXIT\n");
            let expected = match case.strict_error {
                Some(error) => Err(error),
                None => case.legacy.clone(),
            };
            if let Err(error) = check_outcome(compile(&[(case.name, &source)], 400, runtime), &expected) {
                failures.push(format!("{}/runtime {runtime}: {error}", case.name));
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn optional_original_artifacts_agree_with_committed_goldens() {
    let directory = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../target/declare-oracle");
    for case in cases() {
        let log = directory.join(format!("{}.pcboard.log", case.name));
        match std::fs::read_to_string(&log) {
            Ok(log) => {
                assert_eq!(
                    log.contains("Source compilation complete..."),
                    case.legacy.is_ok(),
                    "{} original acceptance",
                    case.name
                );
                assert_eq!(
                    log.contains("Error(s) encountered, compile aborted..."),
                    case.legacy.is_err(),
                    "{} original rejection",
                    case.name
                );
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => panic!("{}: {error}", log.display()),
        }
        if let Ok(expected) = &case.legacy {
            let path = directory.join(format!("{}.pcboard.ppe", case.name));
            let mut bytes = match std::fs::read(&path) {
                Ok(bytes) => bytes,
                Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
                Err(error) => panic!("{}: {error}", path.display()),
            };
            let executable = Executable::from_buffer(&mut bytes, false).unwrap_or_else(|error| panic!("{}: {error}", path.display()));
            assert_eq!(signatures(&executable), [expected.clone()], "{} original signature", case.name);
        }
    }
}

fn check_cross_file(declaration: &str, call: &str, implementation: &str, expected: Result<Signature, &str>) {
    let declaration = format!("{declaration}\n");
    let caller = format!("{call}\nEND\n");
    let mut failures = Vec::new();
    for (language, runtime) in LEGACY_TARGETS {
        for implementation_first in [false, true] {
            // DECLARE after an implementation is a duplicate declaration in
            // this compiler. Keep declarations first, varying only the order
            // of the caller and implementation under test. No runtime execution
            // is intended: these assertions concern semantic checking and PPEs.
            let mut sources = [
                ("declarations.pps", declaration.as_str()),
                ("caller.pps", caller.as_str()),
                ("implementation.pps", implementation),
            ];
            if implementation_first {
                sources.swap(1, 2);
            }
            if let Err(error) = check_outcome(compile(&sources, language, runtime), &expected) {
                failures.push(format!(
                    "language {language}, runtime {runtime}, implementation_first={implementation_first}: {error}"
                ));
            }
        }
    }
    assert!(failures.is_empty(), "{declaration}\n{}", failures.join("\n"));
}

#[test]
fn cross_file_implementation_var_rejects_constants_in_both_file_orders() {
    check_cross_file(
        "DECLARE PROCEDURE Work(INTEGER value)",
        "Work(1)",
        "PROCEDURE Work(VAR INTEGER value)\nPRINTLN value\nENDPROC\n",
        Err(VARIABLE),
    );
}

#[test]
fn cross_file_implementation_var_rejects_expressions_in_both_file_orders() {
    check_cross_file(
        "DECLARE PROCEDURE Work(INTEGER value)",
        "INTEGER value\nvalue = 1\nWork(value + 1)",
        "PROCEDURE Work(VAR INTEGER value)\nvalue = 9\nENDPROC\n",
        Err(VARIABLE),
    );
}

#[test]
fn cross_file_declaration_var_does_not_reject_constants_in_both_file_orders() {
    check_cross_file(
        "DECLARE PROCEDURE Work(VAR INTEGER value)",
        "Work(1)",
        "PROCEDURE Work(INTEGER value)\nPRINTLN value\nENDPROC\n",
        Ok(procedure(&[INTEGER], 0)),
    );
}

#[test]
fn cross_file_implementation_var_is_written_to_the_descriptor() {
    check_cross_file(
        "DECLARE PROCEDURE Work(INTEGER value)",
        "INTEGER value\nvalue = 1\nWork(value)\nPRINTLN value",
        "PROCEDURE Work(VAR INTEGER value)\nvalue = 9\nENDPROC\n",
        Ok(procedure(&[INTEGER], 1)),
    );
}

#[test]
fn cross_file_implementation_parameter_type_wins_in_both_file_orders() {
    check_cross_file(
        "DECLARE PROCEDURE Work(INTEGER value)",
        "Work(\"abc\")",
        "PROCEDURE Work(STRING value)\nPRINTLN value\nENDPROC\n",
        Ok(procedure(&[STRING], 0)),
    );
}

#[test]
fn cross_file_implementation_return_type_wins_in_both_file_orders() {
    for (declared, implemented, value, result) in [("INTEGER", "STRING", "\"abc\"", STRING), ("STRING", "INTEGER", "7", INTEGER)] {
        check_cross_file(
            &format!("DECLARE FUNCTION Work() {declared}"),
            "PRINTLN Work()",
            &format!("FUNCTION Work() {implemented}\nWork = {value}\nENDFUNC\n"),
            Ok(function(&[], result)),
        );
    }
}
