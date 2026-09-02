use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    executable::{EntryType, Executable, PPECommand, RecordField, VariableType},
    hir::{HirCommand, LabelId},
    parser::{Encoding, ErrorReporter, FIRST_USER_TYPE_ID, UserTypeRegistry, parse_ast},
};

fn compile(source: &str) -> Executable {
    compile_sources(&[("dead-code.pps", source)])
}

fn compile_sources(sources: &[(&str, &str)]) -> Executable {
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let workspace = Workspace::default();
    let asts = sources
        .iter()
        .map(|(name, source)| parse_ast(PathBuf::from(name), errors.clone(), source, &registry, Encoding::Utf8, &workspace))
        .collect::<Vec<_>>();
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&asts.iter().collect::<Vec<_>>());
    assert!(!errors.lock().unwrap().has_errors());
    compiler.create_executable().unwrap()
}

fn routine_names(executable: &Executable) -> Vec<String> {
    executable
        .variable_table
        .get_entries()
        .iter()
        .filter(|entry| matches!(entry.entry_type, EntryType::Function | EntryType::Procedure))
        .map(|entry| entry.get_name().to_string())
        .collect()
}

#[test]
fn resolved_hir_keeps_typed_ids_until_ppe_lowering() {
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let workspace = Workspace::default();
    let source = "INTEGER marker\nGOTO Start\n:Other\nmarker = 1\n:Start\nRun()\nPROCEDURE Run()\nENDPROC\n";
    let ast = parse_ast(PathBuf::from("hir.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    assert!(!errors.lock().unwrap().has_errors());

    assert!(matches!(compiler.get_hir_program().commands[0], HirCommand::Goto(LabelId(0))));
    let routine_id = compiler
        .get_hir_program()
        .commands
        .iter()
        .find_map(|command| match command {
            HirCommand::ProcedureCall(routine, arguments) if arguments.is_empty() => Some(*routine),
            _ => None,
        })
        .unwrap();
    assert!(
        compiler
            .get_script()
            .statements
            .iter()
            .any(|statement| matches!(statement.command, PPECommand::ProcedureCall(id, ref arguments) if id == routine_id.0 && arguments.is_empty()))
    );
    assert!(matches!(compiler.get_script().statements[0].command, PPECommand::Goto(offset) if offset > 0));
}

#[test]
fn only_routines_reachable_from_main_are_emitted() {
    let executable = compile(
        r#";$LANGVERSION 400
INTEGER deadGlobal
Live()

PROCEDURE Live()
    Leaf()
ENDPROC

PROCEDURE Leaf()
    PRINTLN "live"
ENDPROC

PROCEDURE Dead()
    deadGlobal = 1
    DeadLeaf()
ENDPROC

PROCEDURE DeadLeaf()
    PRINTLN "dead"
ENDPROC
"#,
    );

    assert_eq!(vec!["Live", "Leaf"], routine_names(&executable));
    assert!(!executable.variable_table.get_entries().iter().any(|entry| entry.get_name() == "deadGlobal"));
    assert!(!executable.variable_table.get_entries().iter().any(|entry| entry.value.as_str() == Some("dead")));
}

#[test]
fn references_between_a_jump_and_its_target_do_not_keep_code_alive() {
    let executable = compile(
        r#";$LANGVERSION 400
INTEGER deadGlobal
GOTO Skip
Dead()
deadGlobal = 1
:Skip
PRINT 1

PROCEDURE Dead()
    PRINT 2
ENDPROC
"#,
    );

    assert!(routine_names(&executable).is_empty());
    assert!(!executable.variable_table.get_entries().iter().any(|entry| entry.get_name() == "deadGlobal"));
}

#[test]
fn unreachable_calls_inside_a_reachable_routine_do_not_keep_their_targets_alive() {
    let executable = compile(
        r#";$LANGVERSION 400
Entry()

PROCEDURE Entry()
    GOTO Skip
    Dead()
    :Skip
ENDPROC

PROCEDURE Dead()
    PRINT 2
ENDPROC
"#,
    );

    assert_eq!(vec!["Entry"], routine_names(&executable));
}

#[test]
fn a_label_jumped_into_from_reachable_code_keeps_its_block_alive() {
    let executable = compile(
        r#";$LANGVERSION 400
INTEGER flag
IF flag = 1 GOTO Target
GOTO Skip
WHILE flag = 3 DO
    :Target
    Alive()
ENDWHILE
:Skip
PRINT 2

PROCEDURE Alive()
    PRINT 3
ENDPROC
"#,
    );

    // The optimizer flattens blocks before dropping code, so it keeps this call - liveness must too.
    assert_eq!(vec!["Alive"], routine_names(&executable));
}

#[test]
fn a_constant_false_branch_does_not_keep_its_callee_alive() {
    let executable = compile(
        r#";$LANGVERSION 400
IF 1 = 0 Dead()
PRINT 1

PROCEDURE Dead()
    PRINT 2
ENDPROC
"#,
    );

    assert!(routine_names(&executable).is_empty());
}

#[test]
fn a_constant_true_branch_keeps_its_callee_alive() {
    let executable = compile(
        r#";$LANGVERSION 400
IF 1 = 1 Live()

PROCEDURE Live()
    PRINT 1
ENDPROC
"#,
    );

    assert_eq!(vec!["Live"], routine_names(&executable));
}

#[test]
fn recursive_reachable_routines_are_emitted() {
    let executable = compile(
        r#";$LANGVERSION 400
Recurse(3)

PROCEDURE Recurse(INTEGER value)
    IF value > 0 Recurse(value - 1)
ENDPROC
"#,
    );

    assert_eq!(vec!["Recurse"], routine_names(&executable));
}

#[test]
fn routine_references_keep_callback_targets_reachable() {
    let executable = compile(
        r#";$LANGVERSION 400
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
"#,
    );

    assert_eq!(vec!["Relay", "Invoke", "PrintValue"], routine_names(&executable));
}

#[test]
fn on_error_procedure_targets_are_reachable() {
    let executable = compile(
        r#";$LANGVERSION 400
ON ERROR Handler
PRINT 1

PROCEDURE Handler()
    PRINT 2
ENDPROC
"#,
    );

    assert_eq!(vec!["Handler"], routine_names(&executable));
}

#[test]
fn unused_locals_are_not_emitted() {
    let executable = compile(
        r#";$LANGVERSION 400
PRINT Used()

FUNCTION Used() INTEGER
    INTEGER kept = 1
    INTEGER discarded
    RETURN kept
ENDFUNC
"#,
    );

    let names: Vec<_> = executable.variable_table.get_entries().iter().map(|entry| entry.get_name().as_str()).collect();
    assert!(names.contains(&"kept"));
    assert!(!names.contains(&"discarded"));
}

#[test]
fn unused_record_types_are_pruned_and_retained_layouts_stay_complete() {
    let executable = compile(
        r#";$LANGVERSION 400
TYPE Dead
    INTEGER Gone
ENDTYPE

TYPE Inner
    INTEGER Number
    STRING SharedField
ENDTYPE

TYPE Outer
    Inner Value
    INTEGER FieldNotReadByThisPpe
ENDTYPE

Outer item
item.Value.Number = 7
PRINT item.Value.Number
"#,
    );

    assert_eq!(2, executable.user_types.len());
    assert_eq!(2, executable.user_types[0].len());
    assert_eq!(2, executable.user_types[1].len());
    assert_eq!(VariableType::UserData(FIRST_USER_TYPE_ID as u8), executable.user_types[1][0].variable_type);
    assert_eq!(VariableType::Integer, executable.user_types[1][1].variable_type);

    let item = executable.variable_table.get_entries().iter().find(|entry| entry.get_name() == "item").unwrap();
    assert_eq!(VariableType::UserData((FIRST_USER_TYPE_ID + 1) as u8), item.header.variable_type);

    let mut bytes = executable.to_buffer().unwrap();
    let loaded = Executable::from_buffer(&mut bytes, false).unwrap();
    assert_eq!(executable.user_types, loaded.user_types);
    assert!(
        loaded
            .variable_table
            .get_entries()
            .iter()
            .any(|entry| entry.header.variable_type == item.header.variable_type)
    );
}

#[test]
fn a_program_without_record_values_emits_no_record_table() {
    let executable = compile(
        r#";$LANGVERSION 400
TYPE Unused
    INTEGER StillDeclared
ENDTYPE
PRINT 1
"#,
    );

    assert!(executable.user_types.is_empty());
}

#[test]
fn a_record_used_only_as_a_whole_keeps_every_field() {
    let executable = compile(
        r#";$LANGVERSION 400
TYPE Packet
    INTEGER Header
    STRING Payload(4)
    BOOLEAN Flag(2, 3)
ENDTYPE

Packet source
Packet target
target = source
"#,
    );

    // Record layouts are positional and may be shared with other PPEs, so no field may ever be pruned.
    assert_eq!(1, executable.user_types.len());
    assert_eq!(
        vec![
            RecordField {
                variable_type: VariableType::Integer,
                dim: 0,
                vector_size: 0,
                matrix_size: 0,
                cube_size: 0,
            },
            RecordField {
                variable_type: VariableType::UnboundedString,
                dim: 1,
                vector_size: 4,
                matrix_size: 0,
                cube_size: 0,
            },
            RecordField {
                variable_type: VariableType::Boolean,
                dim: 2,
                vector_size: 2,
                matrix_size: 3,
                cube_size: 0,
            },
        ],
        executable.user_types[0]
    );
}

#[test]
fn a_record_passed_only_as_a_whole_keeps_every_field() {
    let executable = compile(
        r#";$LANGVERSION 400
TYPE Config
    INTEGER Port
    STRING Host
    BOOLEAN Secure
ENDTYPE

Config settings
Apply(settings)

PROCEDURE Apply(Config value)
    PRINT 1
ENDPROC
"#,
    );

    assert_eq!(1, executable.user_types.len());
    assert_eq!(
        vec![VariableType::Integer, VariableType::UnboundedString, VariableType::Boolean],
        executable.user_types[0].iter().map(|field| field.variable_type).collect::<Vec<_>>()
    );
}

#[test]
fn uncalled_module_exports_are_not_emitted() {
    let executable = compile_sources(&[
        (
            "tools.pps",
            r#"MODULE Tools
PROCEDURE Used()
    PRINTLN "used"
ENDPROC
PROCEDURE NeverCalled()
    PRINTLN "never"
ENDPROC
ENDMODULE
"#,
        ),
        ("main.pps", "IMPORT Tools AS T\nT.Used()\n"),
    ]);

    // Modules are never compiled standalone, so an export nobody calls is dead code here.
    assert_eq!(vec!["__M0_Used"], routine_names(&executable));
    assert!(
        !executable
            .variable_table
            .get_entries()
            .iter()
            .any(|entry| entry.value.as_str() == Some("never"))
    );
}

#[test]
fn module_internal_call_chains_stay_reachable_through_an_import() {
    let executable = compile_sources(&[
        (
            "tools.pps",
            r#"MODULE Tools
PROCEDURE Entry()
    Helper()
ENDPROC
PRIVATE
PROCEDURE Helper()
    PRINTLN "helper"
ENDPROC
ENDMODULE
"#,
        ),
        ("main.pps", "IMPORT Tools AS T\nT.Entry()\n"),
    ]);

    assert_eq!(vec!["__M0_Entry", "__M0_Helper"], routine_names(&executable));
}
