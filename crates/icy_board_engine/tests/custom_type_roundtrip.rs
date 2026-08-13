use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    executable::{Executable, FIRST_TYPE_TABLE_RUNTIME, GenericVariableData, VariableType},
    parser::{Encoding, ErrorReporter, MAX_TYPE_FIELDS, UserTypeRegistry, parse_ast},
};

fn compile(source: &str) -> Executable {
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let workspace = Workspace::default();
    let ast = parse_ast(PathBuf::from("record.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    let reporter = errors.lock().unwrap();
    assert!(reporter.errors.is_empty(), "{:?}", reporter.errors.iter().map(|e| e.error.to_string()).collect::<Vec<_>>());
    drop(reporter);
    compiler.create_executable().unwrap()
}

fn diagnostics(source: &str) -> Vec<String> {
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let workspace = Workspace::default();
    parse_ast(PathBuf::from("record.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let reporter = errors.lock().unwrap();
    reporter.errors.iter().map(|e| e.error.to_string()).collect()
}

fn compile_diagnostics(source: &str, runtime: u16) -> Vec<String> {
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut workspace = Workspace::default();
    workspace.package.runtime = Some(runtime);
    let ast = parse_ast(PathBuf::from("record.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    let reporter = errors.lock().unwrap();
    reporter.errors.iter().map(|e| e.error.to_string()).collect()
}

#[test]
fn custom_type_layouts_survive_the_ppe_round_trip() {
    let executable = compile(
        "TYPE Inner\n  INTEGER Number\n  STRING Text\nENDTYPE\nTYPE Outer\n  Inner Value\n  BOOLEAN Flag\nENDTYPE\nOuter item\n",
    );
    assert_eq!(
        vec![vec![VariableType::Integer, VariableType::String], vec![VariableType::UserData(100), VariableType::Boolean]],
        executable.user_types
    );

    let mut bytes = executable.to_buffer().unwrap();
    let loaded = Executable::from_buffer(&mut bytes, false).unwrap();
    assert_eq!(executable.user_types, loaded.user_types);
}

#[test]
fn loading_a_ppe_rebuilds_nested_record_defaults() {
    let executable = compile(
        "TYPE Inner\n  INTEGER Number\n  STRING Text\nENDTYPE\nTYPE Outer\n  Inner Value\nENDTYPE\nOuter item\nPRINT item.Value.Number\n",
    );
    let mut bytes = executable.to_buffer().unwrap();
    let loaded = Executable::from_buffer(&mut bytes, false).unwrap();
    let value = loaded
        .variable_table
        .get_entries()
        .iter()
        .find(|entry| entry.header.variable_type == VariableType::UserData(101))
        .expect("record variable missing")
        .value
        .clone();

    let GenericVariableData::Record(outer) = value.generic_data else {
        panic!("outer record was not rebuilt");
    };
    let GenericVariableData::Record(inner) = &outer[0].generic_data else {
        panic!("inner record was not rebuilt");
    };
    assert_eq!(VariableType::Integer, inner[0].vtype);
    assert_eq!(0, inner[0].as_int());
    assert_eq!(VariableType::String, inner[1].vtype);
    assert_eq!("", inner[1].as_string());
}

#[test]
fn a_runtime_before_400_does_not_gain_a_custom_type_section() {
    let executable = Executable {
        runtime: 340,
        ..Executable::default()
    };
    let mut bytes = executable.to_buffer().unwrap();
    let loaded = Executable::from_buffer(&mut bytes, false).unwrap();
    assert!(loaded.user_types.is_empty());
}

/// 4.00 understands the syntax but has no type table, so the layout would be
/// dropped on the way out and every field access would read nothing.
#[test]
fn a_type_is_rejected_on_a_runtime_that_cannot_store_it() {
    let errors = compile_diagnostics("TYPE Point\n  INTEGER X\nENDTYPE\nPoint Pt\n", 400);
    assert_eq!(
        vec![format!("'TYPE' needs runtime {FIRST_TYPE_TABLE_RUNTIME}, an older PPE has nowhere to store the layout")],
        errors
    );
}

#[test]
fn a_type_passes_on_the_runtime_that_stores_it() {
    let errors = compile_diagnostics("TYPE Point\n  INTEGER X\nENDTYPE\nPoint Pt\n", FIRST_TYPE_TABLE_RUNTIME);
    assert!(errors.is_empty(), "{errors:?}");
}

/// 4.00 shipped without a type table, so nothing may be written or expected there
/// - reading one byte too many would shift the code size and take the file apart.
#[test]
fn runtime_400_carries_no_type_table() {
    let executable = Executable {
        runtime: 400,
        script_buffer: vec![1, 2, 3],
        ..Executable::default()
    };
    let mut bytes = executable.to_buffer().unwrap();
    let loaded = Executable::from_buffer(&mut bytes, false).unwrap();
    assert!(loaded.user_types.is_empty());
    assert_eq!(executable.script_buffer, loaded.script_buffer);
}

/// The field count is a single byte, so the last record that still fits has to
/// come back unharmed.
#[test]
fn a_record_filled_to_the_byte_limit_round_trips() {
    let mut source = String::from("TYPE Wide\n");
    for i in 0..MAX_TYPE_FIELDS {
        source.push_str(&format!("  INTEGER Field{i}\n"));
    }
    source.push_str("ENDTYPE\nWide item\n");

    let executable = compile(&source);
    assert_eq!(MAX_TYPE_FIELDS, executable.user_types[0].len());

    let mut bytes = executable.to_buffer().unwrap();
    let loaded = Executable::from_buffer(&mut bytes, false).unwrap();
    assert_eq!(executable.user_types, loaded.user_types);
}

#[test]
fn a_record_past_the_byte_limit_is_rejected() {
    let mut source = String::from("TYPE Wide\n");
    for i in 0..=MAX_TYPE_FIELDS {
        source.push_str(&format!("  INTEGER Field{i}\n"));
    }
    source.push_str("ENDTYPE\n");

    let errors = diagnostics(&source);
    assert_eq!(vec![format!("No room for another field, {MAX_TYPE_FIELDS} is the most a type may hold")], errors);
}
