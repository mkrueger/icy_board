use std::{
    fmt::Write,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};

fn diagnostics(source: &str) -> Vec<String> {
    let registry = UserTypeRegistry::default();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let workspace = Workspace::default();
    let ast = parse_ast(PathBuf::from("limits.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());

    compiler.compile(&[&ast]);

    let reporter = errors.lock().unwrap();
    reporter.errors.iter().map(|error| error.error.to_string()).collect()
}

#[test]
fn too_many_routine_parameters_are_reported_without_wrapping() {
    let parameters = (0..=u8::MAX).map(|index| format!("INTEGER p{index}")).collect::<Vec<_>>().join(", ");
    let arguments = (0..=u8::MAX).map(|_| "0").collect::<Vec<_>>().join(", ");
    let errors = diagnostics(&format!("PROCEDURE Many({parameters})\nENDPROC\nBEGIN\n  Many({arguments})\nEND\n"));

    assert!(
        errors.iter().any(|error| error == "Routine Many has too many parameters (256; maximum is 255)"),
        "{errors:?}"
    );
}

#[test]
fn too_many_function_locals_are_reported_without_overflowing() {
    let locals = (0..u8::MAX)
        .map(|index| format!("  INTEGER local{index}\n  PRINTLN local{index}\n"))
        .collect::<String>();
    let errors = diagnostics(&format!("FUNCTION Many() INTEGER\n{locals}  RETURN 0\nENDFUNC\nBEGIN\n  PRINTLN Many()\nEND\n"));

    assert!(
        errors
            .iter()
            .any(|error| error == "Routine Many has too many local variables (255; maximum is 254)"),
        "{errors:?}"
    );
}

#[test]
fn var_parameters_beyond_the_pass_mask_are_reported_without_overflowing() {
    let parameters = (0..17)
        .map(|index| format!("{}INTEGER p{index}", if index == 16 { "VAR " } else { "" }))
        .collect::<Vec<_>>()
        .join(", ");
    let arguments = (0..17).map(|index| format!("value{index}")).collect::<Vec<_>>().join(", ");
    let variables = (0..17).map(|index| format!("INTEGER value{index}\n")).collect::<String>();
    let errors = diagnostics(&format!("{variables}PROCEDURE Many({parameters})\nENDPROC\nBEGIN\n  Many({arguments})\nEND\n"));

    assert!(
        errors
            .iter()
            .any(|error| error == "Procedure Many has a VAR parameter at position 17; the maximum supported position is 16"),
        "{errors:?}"
    );
}

#[test]
fn too_many_declarations_are_rejected_before_ids_wrap() {
    let mut source = String::new();
    for index in 0..=i16::MAX {
        writeln!(source, "INTEGER value{index}").unwrap();
        writeln!(source, "PRINTLN value{index}").unwrap();
    }
    let registry = UserTypeRegistry::default();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let workspace = Workspace::default();
    let ast = parse_ast(
        PathBuf::from("declarations.pps"),
        errors.clone(),
        &source,
        &registry,
        Encoding::Utf8,
        &workspace,
    );
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());

    compiler.compile(&[&ast]);

    assert!(!errors.lock().unwrap().has_errors());
    assert!(matches!(
        compiler.create_executable(),
        Err(icy_board_engine::compiler::CompilationErrorType::TooManyDeclarations(count, max))
            if count > max && max == i16::MAX as usize
    ));
}
