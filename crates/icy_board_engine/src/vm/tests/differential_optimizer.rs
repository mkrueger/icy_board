use std::{
    collections::BTreeMap,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_net::{Connection, ConnectionType, channel::ChannelConnection};

use crate::{
    compiler::{PPECompiler, workspace::Workspace},
    executable::{EntryType, PPEScript},
    icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    vm::{PplError, VirtualMachine, io::DiskIO},
};

#[derive(Debug, PartialEq, Eq)]
struct ExecutionSnapshot {
    result: Result<(), String>,
    output: Vec<u8>,
    variables: BTreeMap<String, String>,
    last_error: PplError,
    error_pending: bool,
    return_addresses: usize,
    call_locals: usize,
    write_backs: usize,
    push_pop_values: usize,
    file_effect: Option<Vec<u8>>,
}

fn compile(source: &str, optimize: bool) -> crate::executable::Executable {
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let registry = UserTypeRegistry::icy_board_registry();
    let mut workspace = Workspace::default();
    workspace.hard_coded_files = Some(vec![PathBuf::from("differential.pps")]);
    let ast = parse_ast(PathBuf::from("differential.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone()).with_optimization(optimize);
    compiler.compile(&[&ast]);
    let reporter = errors.lock().unwrap();
    assert!(
        !reporter.has_errors(),
        "source failed to compile with optimize={optimize}:\n{}",
        reporter.errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>().join("\n")
    );
    drop(reporter);
    compiler.create_executable().unwrap()
}

fn execute(source: &str, optimize: bool) -> ExecutionSnapshot {
    let executable = compile(source, optimize);
    let root = super::scratch_dir(if optimize { "optimizer-on" } else { "optimizer-off" });
    let snapshot = tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let mut board = IcyBoard::new();
        board.root_path = root.clone();
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let node_state = bbs.lock().await.open_connections.clone();
        let (mut peer, connection) = ChannelConnection::create_pair();
        let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), node_state, node, Box::new(connection)).await;

        // Drained on its own thread so a program that outgrows one channel buffer cannot block the VM.
        let collected = Arc::new(Mutex::new(Vec::new()));
        let sink = collected.clone();
        let reader = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
                let mut buffer = [0; 1024];
                while let Ok(size) = peer.read(&mut buffer).await {
                    if size == 0 {
                        break;
                    }
                    sink.lock().unwrap().extend_from_slice(&buffer[..size]);
                }
            });
        });

        let mut io = DiskIO::new(root.to_str().unwrap(), None);
        let registry = UserTypeRegistry::icy_board_registry();
        let script = PPEScript::from_ppe_file(&executable).unwrap();
        let commands = script.statements.iter().map(|statement| statement.command.clone()).collect::<Vec<_>>().into();
        let mut vm = VirtualMachine::new(PathBuf::from("differential.ppe"), &registry, &mut io, &mut state);
        vm.script = script;
        vm.commands = commands;
        vm.variable_table = executable.variable_table;
        vm.user_types = executable.user_types;
        vm.label_table = vm
            .script
            .statements
            .iter()
            .enumerate()
            .map(|(index, statement)| (statement.span.start * 2, index))
            .collect();

        let result = vm.run().await.map_err(|error| error.to_string());
        let variables = vm
            .variable_table
            .get_entries()
            .iter()
            .filter(|entry| entry.entry_type == EntryType::Variable)
            .map(|entry| (entry.get_name().clone(), format!("{:?}", entry.value)))
            .collect();
        let snapshot = (
            result,
            variables,
            vm.last_error.clone(),
            vm.error_pending,
            vm.return_addresses.len(),
            vm.call_local_value_stack.len(),
            vm.write_back_stack.len(),
            vm.push_pop_stack.len(),
        );
        drop(vm);
        drop(state);
        reader.join().unwrap();

        let output = collected.lock().unwrap().clone();
        (snapshot, output)
    });

    let file_effect = std::fs::read(root.join("effect.txt")).ok();
    let _ = std::fs::remove_dir_all(&root);
    let ((result, variables, last_error, error_pending, return_addresses, call_locals, write_backs, push_pop_values), output) = snapshot;
    ExecutionSnapshot {
        result,
        output,
        variables,
        last_error,
        error_pending,
        return_addresses,
        call_locals,
        write_backs,
        push_pop_values,
        file_effect,
    }
}

fn assert_equivalent(source: &str) -> ExecutionSnapshot {
    let optimized = execute(source, true);
    let unoptimized = execute(source, false);

    assert_eq!(unoptimized.result, optimized.result, "result differs for:\n{source}");
    assert_eq!(unoptimized.output, optimized.output, "output differs for:\n{source}");
    assert_eq!(unoptimized.last_error, optimized.last_error, "error differs for:\n{source}");
    assert_eq!(unoptimized.error_pending, optimized.error_pending, "pending error differs for:\n{source}");
    assert_eq!(unoptimized.return_addresses, optimized.return_addresses, "return stack differs for:\n{source}");
    assert_eq!(unoptimized.call_locals, optimized.call_locals, "call locals differ for:\n{source}");
    assert_eq!(unoptimized.write_backs, optimized.write_backs, "write backs differ for:\n{source}");
    assert_eq!(unoptimized.push_pop_values, optimized.push_pop_values, "push/pop stack differs for:\n{source}");
    assert_eq!(unoptimized.file_effect, optimized.file_effect, "file effect differs for:\n{source}");

    // Dropping a variable nothing can observe is the optimizer's job, but every variable it
    // keeps has to hold the value the unoptimized program produced.
    for (name, value) in &optimized.variables {
        assert_eq!(unoptimized.variables.get(name), Some(value), "variable {name} differs for:\n{source}");
    }

    optimized
}

#[test]
fn generated_arithmetic_and_branches_are_equivalent() {
    for left in -3..=3 {
        for right in -3..=3 {
            let source = format!(
                r#";$LANGVERSION 400
INTEGER result
result = ({left} + {right}) * ({left} - {right})
IF result < 0 GOTO Negative
PRINTLN "nonnegative:", result
GOTO Done
:Negative
PRINTLN "negative:", result
:Done
"#,
            );
            assert_equivalent(&source);
        }
    }
}

#[test]
fn generated_label_and_routine_call_shapes_are_equivalent() {
    for take_first in [false, true] {
        for use_gosub in [false, true] {
            let branch = i32::from(take_first);
            let (prefix, call, routine) = if use_gosub {
                ("GOTO Main\n:Worker\nresult = result + 7\nRETURN\n:Main\n", "GOSUB Worker", "")
            } else {
                ("", "CallWorker()", "PROCEDURE CallWorker()\n    result = result + 7\nENDPROC\n")
            };
            let source = format!(
                r";$LANGVERSION 400
INTEGER result
{prefix}
IF {branch} = 1 GOTO First
GOTO Second
:First
result = 10
GOTO Join
:Second
result = 20
:Join
{call}
PRINTLN result
{routine}",
            );
            assert_equivalent(&source);
        }
    }
}

#[test]
fn the_two_modes_emit_different_code_before_they_are_compared() {
    let source = r#"GOTO Done
PRINTLN "unreachable"
:Done
PRINTLN "done"
"#;
    let optimized = compile(source, true);
    let unoptimized = compile(source, false);

    assert!(optimized.script_buffer.len() < unoptimized.script_buffer.len());
    assert_equivalent(source);
}

#[test]
fn every_statement_optimizer_transformation_preserves_behavior() {
    for source in [
        "IF 1 = 0 PRINTLN \"never\"\nPRINTLN \"constant branch\"\n",
        "GOTO Next\n:Next\nPRINTLN \"adjacent label\"\n",
        "GOTO Live\nPRINTLN \"unreachable\"\n:Live\nPRINTLN \"reachable\"\n",
        "PRINTLN (2 + 3) * (4 - 1)\n",
    ] {
        assert_equivalent(source);
    }
}

#[test]
fn eliminating_a_variable_nothing_can_observe_is_not_a_difference() {
    let source = ";$LANGVERSION 400\nINTEGER x\nGOTO Skip\nx = 1\n:Skip\nPRINTLN \"done\"\n";
    let optimized = execute(source, true);
    let unoptimized = execute(source, false);

    assert!(!optimized.variables.contains_key("x"));
    assert!(unoptimized.variables.contains_key("x"));
    assert_equivalent(source);
}

#[test]
fn output_variables_errors_frames_and_file_effects_are_equivalent() {
    let source = r#";$LANGVERSION 400
INTEGER total, handled
STRING text

total = Sum(6)
text = "value=" + STRING(total)
ON ERROR GOSUB Failed
Terminal.LoadFont(43, "missing.fnt")
ON ERROR OFF
FCREATE 1, "effect.txt", O_WR, S_DN
FPUTLN 1, text
FCLOSE 1
PRINTLN text, " handled=", handled
EXIT

:Failed
handled = 1
RETURN

FUNCTION Sum(INTEGER value) INTEGER
    IF value <= 0 RETURN 0
    RETURN value + Sum(value - 1)
ENDFUNC
"#;

    let snapshot = assert_equivalent(source);
    assert_eq!(Ok(()), snapshot.result);
    assert_eq!(0, snapshot.return_addresses);
    assert_eq!(0, snapshot.call_locals);
    assert_eq!(0, snapshot.write_backs);
    assert_eq!(0, snapshot.push_pop_values);
    assert_eq!(b"value=21 handled=1\r\n", snapshot.output.as_slice());
    assert!(snapshot.variables.contains_key("total"));
    assert!(snapshot.variables.contains_key("handled"));
    assert!(snapshot.variables.contains_key("text"));
    assert_eq!(Some(b"\xEF\xBB\xBFvalue=21\n".to_vec()), snapshot.file_effect);
}
