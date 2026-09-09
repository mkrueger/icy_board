use std::{path::PathBuf, sync::Arc};

use crate::{
    compiler::{PPECompiler, workspace::Workspace},
    executable::{EntryType, Executable, PPECommand, PPEExpr, ProcedureValue, TableEntry, VarHeader, VariableTable, VariableType, VariableValue},
    icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState, user_base::User},
    parser::{Encoding, ErrorReporter, parse_ast},
    vm::{ReturnAddress, VirtualMachine, io::DiskIO},
};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};

const LEGACY_TARGETS: [(u16, u16); 4] = [(340, 340), (340, 400), (350, 350), (350, 400)];

fn compile_legacy(source: &str, language: u16, runtime: u16) -> Executable {
    let errors = Arc::new(std::sync::Mutex::new(ErrorReporter::default()));
    let registry = crate::parser::icy_board_registry();
    let mut workspace = Workspace::default();
    workspace.hard_coded_files = Some(vec![PathBuf::from("legacy.pps")]);
    workspace.package.runtime = Some(runtime);
    workspace.set_default_language_version(Some(language));
    let ast = parse_ast(PathBuf::from("legacy.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    let diagnostics = errors.lock().unwrap();
    assert!(
        !diagnostics.has_errors(),
        "language={language}, runtime={runtime}: {:?}",
        diagnostics.errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>()
    );
    drop(diagnostics);
    let mut bytes = compiler.create_executable().unwrap().to_buffer().unwrap();
    Executable::from_buffer(&mut bytes, false).unwrap()
}

async fn run_legacy(executable: &Executable) -> String {
    crate::executable::PPEScript::from_ppe_file(executable).expect("legacy executable must decode");
    let directory = tempfile::tempdir().unwrap();
    let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (mut peer, connection) = ChannelConnection::create_pair();
    let mut board = IcyBoard::new();
    board.root_path = directory.path().to_path_buf();
    board.default_display_text = crate::icy_board::icb_text::DEFAULT_DISPLAY_TEXT.clone();
    let user = User {
        name: "SYSOP".to_string(),
        security_level: 255,
        ..Default::default()
    };
    board.users.new_user(user.clone());
    let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), nodes, node, Box::new(connection)).await;
    state.session.current_user = Some(user);
    state.session.cur_user_id = 0;
    let (result, bytes) = tokio::join!(
        async {
            let mut io = DiskIO::new(directory.path().to_str().unwrap(), None);
            let result = crate::vm::run(&PathBuf::from("legacy.ppe"), executable, &mut io, &mut state).await;
            drop(state);
            result
        },
        async {
            let mut bytes = Vec::new();
            let mut buffer = [0; 1024];
            while let Ok(size) = peer.read(&mut buffer).await {
                if size == 0 {
                    break;
                }
                bytes.extend_from_slice(&buffer[..size]);
            }
            bytes
        }
    );
    result.unwrap();
    String::from_utf8(bytes).unwrap().replace("\r\n", "\n")
}

#[tokio::test]
async fn original_compiler_probes_keep_implementation_rank_and_bound_not_declaration_dimensions() {
    // PPLC 3.40 emits dim=1/bound=3 for both probes and accepts Work(1).
    for source in [
        include_str!("../../../../../compat/declare/dim_bound_index.pps"),
        include_str!("../../../../../compat/declare/dim_scalar_index.pps"),
    ] {
        for (language, runtime) in LEGACY_TARGETS {
            let executable = compile_legacy(source, language, runtime);
            let parameters: Vec<_> = executable
                .variable_table
                .get_entries()
                .iter()
                .filter(|entry| entry.get_type() == EntryType::Parameter)
                .collect();
            assert_eq!(1, parameters.len());
            let header = &parameters[0].header;
            assert_eq!(
                (1, 3, 0, 0, 0),
                (header.dim, header.vector_size, header.matrix_size, header.cube_size, header.flags)
            );
            assert_eq!("1\n", run_legacy(&executable).await);
        }
    }
}

#[tokio::test]
async fn value_parameter_tails_persist_between_calls_but_local_arrays_reset() {
    let source = r#"
Work(7)
Work(8)
PROCEDURE Work(INTEGER values(3))
INTEGER scratch(1)
PRINT values(0), ":", values(3), ":", scratch(1), ";"
values(3) = values(3) + 1
scratch(1) = 99
ENDPROC
"#;
    for (language, runtime) in LEGACY_TARGETS {
        assert_eq!("7:0:0;8:1:0;", run_legacy(&compile_legacy(source, language, runtime)).await);
    }
}

#[tokio::test]
async fn recursive_value_parameters_restore_only_element_zero_and_share_tail_changes() {
    let source = r#"
Work(2)
Work(0)
PROCEDURE Work(INTEGER values(3))
values(3) = values(3) + 1
IF (values(0) > 0) THEN
    Work(values(0) - 1)
ENDIF
PRINT values(0), ":", values(3), ";"
ENDPROC
"#;
    for (language, runtime) in LEGACY_TARGETS {
        assert_eq!("0:3;1:3;2:3;0:4;", run_legacy(&compile_legacy(source, language, runtime)).await);
    }
}

#[tokio::test]
async fn single_var_parameter_copies_only_element_zero_to_scalar_and_indexed_callers() {
    let source = r#"
INTEGER value, data(1)
value = 7
data(0) = 8
data(1) = 9
Work(value)
Work(data(1))
PRINT value, ":", data(0), ":", data(1)
PROCEDURE Work(VAR INTEGER values(3))
values(3) = values(3) + 1
values(0) = values(0) + values(3)
ENDPROC
"#;
    for (language, runtime) in LEGACY_TARGETS {
        assert_eq!("8:8:11", run_legacy(&compile_legacy(source, language, runtime)).await);
    }
}

#[tokio::test]
async fn multiple_var_parameters_copy_out_scalars_and_leave_caller_array_tails_alone() {
    let source = r#"
INTEGER first, data(1)
first = 7
data(0) = 8
data(1) = 9
Work(first, data(1))
PRINT first, ":", data(0), ":", data(1)
PROCEDURE Work(VAR INTEGER firstValues(3), VAR INTEGER secondValues(2))
firstValues(0) = firstValues(0) + 10
secondValues(0) = secondValues(0) + 20
firstValues(3) = 99
secondValues(2) = 88
ENDPROC
"#;
    for (language, runtime) in LEGACY_TARGETS {
        assert_eq!("17:8:29", run_legacy(&compile_legacy(source, language, runtime)).await);
    }
}

#[tokio::test]
async fn recursive_var_parameters_restore_outer_zero_before_copyout_and_preserve_tail() {
    let source = r#"
INTEGER value
Work(value, 2)
PRINT value, "|"
Work(value, 1)
PRINT value
PROCEDURE Work(VAR INTEGER values(3), INTEGER depth)
values(0) = values(0) + depth
values(3) = values(3) + 1
IF (depth > 0) THEN
    Work(values(0), depth - 1)
ENDIF
values(0) = values(0) + 10
PRINT values(0), ":", values(3), ";"
ENDPROC
"#;
    for (language, runtime) in LEGACY_TARGETS {
        assert_eq!("13:3;23:3;33:3;33|44:5;54:5;54", run_legacy(&compile_legacy(source, language, runtime)).await);
    }
}

#[tokio::test]
async fn functions_accept_scalar_arguments_and_keep_parameter_tails_across_recursion() {
    let source = r#"
PRINT Read(2), ":", Read(0)
FUNCTION Read(INTEGER values(3)) INTEGER
INTEGER nested
values(3) = values(3) + 1
IF (values(0) > 0) THEN
    nested = Read(values(0) - 1)
ENDIF
Read = values(0) + values(3)
ENDFUNC
"#;
    for (language, runtime) in LEGACY_TARGETS {
        assert_eq!("5:4", run_legacy(&compile_legacy(source, language, runtime)).await);
    }
}

#[tokio::test]
async fn string_var_parameters_preserve_tail_storage_and_copy_out_only_the_first_string() {
    let source = r#"
STRING value
value = "a"
Work(value)
PRINT value, "|"
Work(value)
PRINT value
PROCEDURE Work(VAR STRING values(3))
values(3) = values(3) + "x"
values(0) = values(0) + values(3)
ENDPROC
"#;
    for (language, runtime) in LEGACY_TARGETS {
        assert_eq!("ax|axxx", run_legacy(&compile_legacy(source, language, runtime)).await);
    }
}

#[tokio::test]
async fn legacy_bytecode_and_vm_supplied_values_save_only_zero_even_with_static_flag() {
    let directory = tempfile::tempdir().unwrap();
    let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (_peer, connection) = ChannelConnection::create_pair();
    let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(IcyBoard::new())), nodes, node, Box::new(connection)).await;
    let registry = crate::parser::icy_board_registry();
    let mut io = DiskIO::new(directory.path().to_str().unwrap(), None);

    // Original PPEs need no new marker. Unknown bit 0x04 must not activate the
    // new convention in old runtimes, and STATIC never exempts legacy params.
    for (runtime, flags) in [(300, 0), (340, 0), (350, 0), (400, 0), (340, 0x04), (340, 0x01), (400, 0x01)] {
        let mut table = VariableTable::default();
        table.set_version(runtime);
        table.push(TableEntry::new(
            "work",
            VarHeader {
                id: 1,
                variable_type: VariableType::Procedure,
                ..Default::default()
            },
            VariableValue::new_procedure(ProcedureValue {
                parameters: 1,
                first_var_id: 1,
                pass_flags: 1,
                ..Default::default()
            }),
            EntryType::Procedure,
        ));
        for (id, kind) in [(2, EntryType::Parameter), (3, EntryType::Variable)] {
            table.push(TableEntry::new(
                "values",
                VarHeader {
                    id,
                    dim: 1,
                    vector_size: 3,
                    variable_type: VariableType::Integer,
                    flags: if id == 2 { flags } else { 0 },
                    ..Default::default()
                },
                VariableValue::new_int(0),
                kind,
            ));
        }
        let mut bytes = Vec::new();
        table.serialize(&mut bytes).unwrap();
        let (_, table) = VariableTable::deserialize(runtime, &mut bytes).unwrap();
        let mut vm = VirtualMachine::new(PathBuf::from("legacy.ppe"), &registry, &mut io, &mut state);
        vm.variable_table = table;
        for (id, zero, tail) in [(2, 11, 12), (3, 21, 22)] {
            vm.variable_table
                .get_value_mut(id)
                .set_array_value(0, 0, 0, VariableValue::new_int(zero))
                .unwrap();
            vm.variable_table
                .get_value_mut(id)
                .set_array_value(3, 0, 0, VariableValue::new_int(tail))
                .unwrap();
        }
        vm.prepare_call(0, 1, 2, &[PPEExpr::Value(3)], 1).await.unwrap();
        assert_eq!(21, vm.variable_table.get_value(2).get_array_value(0, 0, 0).as_int());
        assert_eq!(12, vm.variable_table.get_value(2).get_array_value(3, 0, 0).as_int());
        assert_eq!(0, vm.call_local_value_stack[0].get_dimensions());
        assert_eq!(11, vm.call_local_value_stack[0].as_int());
        vm.return_addresses.push(ReturnAddress::func_call(0, 1));

        // Exercise the synchronous VM-provided argument path on a nested frame.
        vm.prepare_call_with_values(0, 1, 2, vec![VariableValue::new_int(31)]).unwrap();
        vm.variable_table.get_value_mut(2).set_array_value(3, 0, 0, VariableValue::new_int(99)).unwrap();
        vm.write_back_stack.push(PPEExpr::Value(3));
        vm.return_addresses.push(ReturnAddress::func_call(0, 1));
        vm.execute_statement(&PPECommand::EndProc).await.unwrap();
        assert_eq!(21, vm.variable_table.get_value(2).get_array_value(0, 0, 0).as_int());
        assert_eq!(99, vm.variable_table.get_value(2).get_array_value(3, 0, 0).as_int());
        assert_eq!(31, vm.variable_table.get_value(3).get_array_value(0, 0, 0).as_int());
        assert_eq!(22, vm.variable_table.get_value(3).get_array_value(3, 0, 0).as_int());

        vm.execute_statement(&PPECommand::EndProc).await.unwrap();
        assert_eq!(11, vm.variable_table.get_value(2).get_array_value(0, 0, 0).as_int());
        assert_eq!(99, vm.variable_table.get_value(2).get_array_value(3, 0, 0).as_int());
        assert_eq!(21, vm.variable_table.get_value(3).get_array_value(0, 0, 0).as_int());
        assert_eq!(22, vm.variable_table.get_value(3).get_array_value(3, 0, 0).as_int());
        assert!(vm.call_local_value_stack.is_empty());
        assert!(vm.write_back_stack.is_empty());
        assert!(vm.return_addresses.is_empty());
    }
}
