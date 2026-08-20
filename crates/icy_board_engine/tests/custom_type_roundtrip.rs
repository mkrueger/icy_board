use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    executable::{Executable, ExecutableError, FIRST_TYPE_TABLE_RUNTIME, GenericVariableData, TableEntry, VariableType},
    parser::{Encoding, ErrorReporter, MAX_TYPE_FIELDS, MAX_USER_TYPES, UserTypeRegistry, parse_ast},
};
use std::fmt::Write as _;

fn compile(source: &str) -> Executable {
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let workspace = Workspace::default();
    let ast = parse_ast(PathBuf::from("record.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    let reporter = errors.lock().unwrap();
    assert!(
        reporter.errors.is_empty(),
        "{:?}",
        reporter.errors.iter().map(|e| e.error.to_string()).collect::<Vec<_>>()
    );
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
    let executable = compile("TYPE Inner\n  INTEGER Number\n  STRING Text\nENDTYPE\nTYPE Outer\n  Inner Value\n  BOOLEAN Flag\nENDTYPE\nOuter item\n");
    assert_eq!(
        vec![
            vec![VariableType::Integer, VariableType::String],
            vec![VariableType::UserData(100), VariableType::Boolean]
        ],
        executable.user_types
    );

    let mut bytes = executable.to_buffer().unwrap();
    let loaded = Executable::from_buffer(&mut bytes, false).unwrap();
    assert_eq!(executable.user_types, loaded.user_types);
}

#[test]
fn loading_a_ppe_rebuilds_nested_record_defaults() {
    let executable = compile("TYPE Inner\n  INTEGER Number\n  STRING Text\nENDTYPE\nTYPE Outer\n  Inner Value\nENDTYPE\nOuter item\nPRINT item.Value.Number\n");
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
fn a_runtime_before_401_does_not_gain_a_custom_type_section() {
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
        vec![format!(
            "'TYPE' needs runtime {FIRST_TYPE_TABLE_RUNTIME}, an older PPE has nowhere to store the layout"
        )],
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
        let _ = writeln!(source, "  INTEGER Field{i}");
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
        let _ = writeln!(source, "  INTEGER Field{i}");
    }
    source.push_str("ENDTYPE\n");

    let errors = diagnostics(&source);
    assert_eq!(
        vec![format!("No room for another field, {MAX_TYPE_FIELDS} is the most a type may hold")],
        errors
    );
}

#[test]
fn the_serializer_rejects_custom_types_before_runtime_401() {
    let executable = Executable {
        runtime: 400,
        user_types: vec![vec![VariableType::Integer]],
        ..Executable::default()
    };
    assert_eq!(
        ExecutableError::CustomTypesNotSupported(FIRST_TYPE_TABLE_RUNTIME),
        executable.to_buffer().unwrap_err()
    );
}

#[test]
fn the_serializer_rejects_counts_that_do_not_fit_the_format() {
    let too_many_types = Executable {
        user_types: vec![vec![VariableType::Integer]; MAX_USER_TYPES + 1],
        ..Executable::default()
    };
    assert_eq!(
        ExecutableError::TypeCountExceedsMaximum(MAX_USER_TYPES + 1, MAX_USER_TYPES),
        too_many_types.to_buffer().unwrap_err()
    );

    let too_many_fields = Executable {
        user_types: vec![vec![VariableType::Integer; MAX_TYPE_FIELDS + 1]],
        ..Executable::default()
    };
    assert_eq!(
        ExecutableError::InvalidTypeFieldCount(100, MAX_TYPE_FIELDS + 1),
        too_many_fields.to_buffer().unwrap_err()
    );
}

#[test]
fn the_serializer_rejects_recursive_or_forward_type_references() {
    let self_reference = Executable {
        user_types: vec![vec![VariableType::UserData(100)]],
        ..Executable::default()
    };
    assert_eq!(ExecutableError::InvalidTypeReference(100, 100), self_reference.to_buffer().unwrap_err());

    let forward_reference = Executable {
        user_types: vec![vec![VariableType::UserData(101)], vec![VariableType::Integer]],
        ..Executable::default()
    };
    assert_eq!(ExecutableError::InvalidTypeReference(100, 101), forward_reference.to_buffer().unwrap_err());
}

#[test]
fn the_serializer_rejects_a_board_object_field() {
    let executable = Executable {
        user_types: vec![vec![VariableType::UserData(30)]],
        ..Executable::default()
    };
    assert_eq!(ExecutableError::BoardObjectTypeField(100, 30), executable.to_buffer().unwrap_err());
}

#[test]
fn the_serializer_rejects_a_variable_whose_type_is_missing() {
    let mut executable = Executable::default();
    let mut entry = TableEntry::default();
    entry.header.id = 1;
    entry.header.variable_type = VariableType::UserData(100);
    executable.variable_table.push(entry);

    assert_eq!(ExecutableError::MissingTypeDefinition(100), executable.to_buffer().unwrap_err());
}

#[test]
fn the_loader_rejects_a_recursive_type_table() {
    let executable = Executable {
        user_types: vec![vec![VariableType::Integer]],
        ..Executable::default()
    };
    let mut bytes = executable.to_buffer().unwrap();
    // Header (48), empty variable table count (2), type count, field count, field type.
    bytes[52] = 100;
    assert!(matches!(
        Executable::from_buffer(&mut bytes, false),
        Err(error) if error.to_string() == "Type 100 refers to type 100, which has not been declared yet"
    ));
}

#[test]
fn the_loader_rejects_a_truncated_type_table() {
    let executable = Executable::default();
    let mut bytes = executable.to_buffer().unwrap();
    bytes[50] = 1;
    bytes.truncate(51);
    assert!(matches!(
        Executable::from_buffer(&mut bytes, false),
        Err(error) if error.to_string().starts_with("Buffer too short")
    ));
}
