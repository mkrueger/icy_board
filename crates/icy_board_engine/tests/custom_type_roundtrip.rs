use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    executable::{Executable, GenericVariableData, VariableType},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
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
