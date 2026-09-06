//! Semantic PPE -> ppld source -> PPE regressions, not source-text goldens.
//!
//! Both sides are serialized and read back before execution/decompilation. The
//! runtime-only instrumentation below meters *every* command, including commands
//! reached by the recursive `run()` used for function expressions. No timeout,
//! scheduler cooperation, channel reader thread, or production VM change is used.

use std::{
    path::PathBuf,
    sync::{
        Arc, Mutex,
        atomic::{AtomicUsize, Ordering},
    },
};

use async_trait::async_trait;
use icy_net::{Connection, ConnectionType};
use unicase::Ascii;

use crate::{
    Res,
    ast::output_visitor::OutputVisitor,
    compiler::{
        PPECompiler,
        user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
        workspace::Workspace,
    },
    decompiler::decompile,
    executable::{EntryType, Executable, PPECommand, PPEExpr, PPEScript, PPEStatement, TableEntry, VarHeader, VariableType, VariableValue},
    icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    vm::{PplError, VirtualMachine, io::DiskIO},
};

const INSTRUCTION_LIMIT: usize = 4096;
const BUDGET_ERROR: &str = "decompiler roundtrip instruction budget exhausted";
// Only registered in the execution harness, never compiled or serialized.
const METER_TYPE: usize = 99;

struct InstructionMeter {
    executed: Arc<AtomicUsize>,
    limit: usize,
}

impl UserData for InstructionMeter {
    const TYPE_NAME: &'static str = "RoundtripInstructionMeter";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_property(Ascii::new("Tick".to_string()), VariableType::Integer, false);
    }
}

#[async_trait(?Send)]
impl UserDataValue for InstructionMeter {
    fn get_property_value(&self, _vm: &VirtualMachine, _name: &Ascii<String>) -> Res<VariableValue> {
        if self.executed.fetch_add(1, Ordering::Relaxed) >= self.limit {
            return Err(std::io::Error::other(BUDGET_ERROR).into());
        }
        Ok(VariableValue::new_int(0))
    }

    async fn set_property_value(&self, _vm: &mut VirtualMachine<'_>, _name: &Ascii<String>, _value: VariableValue) -> Res<()> {
        unreachable!("the meter is read-only")
    }

    async fn call_function(&self, _vm: &mut VirtualMachine<'_>, _name: &Ascii<String>, _arguments: &[VariableValue]) -> Res<VariableValue> {
        unreachable!("the meter has no functions")
    }

    async fn call_method(&mut self, _vm: &mut VirtualMachine<'_>, _name: &Ascii<String>, _arguments: &[VariableValue]) -> Res<()> {
        unreachable!("the meter has no methods")
    }
}

struct OutputSink(Arc<Mutex<Vec<u8>>>);

#[async_trait]
impl Connection for OutputSink {
    fn get_connection_type(&self) -> ConnectionType {
        ConnectionType::Channel
    }

    async fn read(&mut self, _buffer: &mut [u8]) -> icy_net::Result<usize> {
        Ok(0)
    }

    async fn try_read(&mut self, _buffer: &mut [u8]) -> icy_net::Result<usize> {
        Ok(0)
    }

    async fn send(&mut self, bytes: &[u8]) -> icy_net::Result<()> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(())
    }
}

fn compile(source: &str, optimize: bool) -> Executable {
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let registry = UserTypeRegistry::icy_board_registry();
    let mut workspace = Workspace::default();
    workspace.hard_coded_files = Some(vec![PathBuf::from("roundtrip.pps")]);
    workspace.package.runtime = Some(400);
    workspace.set_default_language_version(Some(400));
    let ast = parse_ast(PathBuf::from("roundtrip.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone()).with_optimization(optimize);
    compiler.compile(&[&ast]);
    let reporter = errors.lock().unwrap();
    assert!(
        !reporter.has_errors(),
        "compile failed (optimize={optimize}):\n{}\n{source}",
        reporter.errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>().join("\n")
    );
    drop(reporter);
    let executable = compiler.create_executable().expect("create PPE");
    reload(&executable)
}

fn reload(executable: &Executable) -> Executable {
    Executable::from_buffer(&mut executable.to_buffer().expect("serialize PPE"), false).expect("read serialized PPE")
}

#[derive(Debug, PartialEq, Eq)]
struct Outcome {
    result: Result<(), String>,
    output: String,
    aborted: bool,
    last_error: PplError,
    error_pending: bool,
    // Names/variable IDs change during decompilation; observable values are
    // printed by each fixture instead of comparing unrelated table slots.
    frames: [usize; 5],
}

fn execute(executable: &Executable, limit: usize) -> (Outcome, usize) {
    let executable = reload(executable);
    let mut script = PPEScript::from_ppe_file(&executable).expect("decode commands");
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let mut board = IcyBoard::new();
        board.root_path = directory.path().to_path_buf();
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let output = Arc::new(Mutex::new(Vec::new()));
        let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), nodes, node, Box::new(OutputSink(output.clone()))).await;
        // Compare program output, not locale-dependent interactive MORE prompts.
        state.session.disp_options.force_non_stop();
        let mut io = DiskIO::new(directory.path().to_str().unwrap(), None);
        let mut registry = UserTypeRegistry::icy_board_registry();
        registry.register::<InstructionMeter>(METER_TYPE);
        let mut vm = VirtualMachine::new(PathBuf::from("roundtrip.ppe"), &registry, &mut io, &mut state);
        vm.variable_table = executable.variable_table;
        vm.user_types = executable.user_types;
        let executed = Arc::new(AtomicUsize::new(0));
        let meter_id = vm.variable_table.get_entries().len() + 1;
        vm.variable_table.push(TableEntry::new(
            "test-only instruction meter",
            VarHeader {
                id: meter_id,
                variable_type: VariableType::UserData(METER_TYPE as u8),
                ..Default::default()
            },
            user_data_value(
                InstructionMeter {
                    executed: executed.clone(),
                    limit,
                },
                METER_TYPE,
            ),
            EntryType::Constant,
        ));

        // Keep original byte offsets, but map each entry/branch target to its
        // meter command. Return addresses are command indices and are managed by
        // the real VM. Duplicate spans keep FOREACH's next-statement body_start
        // and byte-offset cleanup ranges intact. Nothing here is fed to ppld.
        let original = std::mem::take(&mut script.statements);
        for statement in original {
            vm.label_table.insert(statement.span.start * 2, script.statements.len());
            script.statements.push(PPEStatement {
                span: statement.span.clone(),
                command: PPECommand::MemberCall(Box::new(PPEExpr::Member(Box::new(PPEExpr::Value(meter_id)), 0))),
            });
            script.statements.push(statement);
        }
        vm.commands = script.statements.iter().map(|statement| statement.command.clone()).collect::<Vec<_>>().into();
        vm.script = script;

        // Exact production dispatch, including check_error_trap and recursive
        // function execution; the meter fails before command limit + 1 executes.
        // These fixtures use only finite arithmetic/output builtins: this is an
        // instruction bound, not a deadline for arbitrary blocking host I/O.
        let result = vm.run().await.map_err(|error| error.to_string());
        let outcome = Outcome {
            result,
            output: String::from_utf8(output.lock().unwrap().clone()).unwrap().replace("\r\n", "\n"),
            aborted: vm.aborted,
            last_error: vm.last_error.clone(),
            error_pending: vm.error_pending,
            frames: [
                vm.return_addresses.len(),
                vm.call_local_value_stack.len(),
                vm.write_back_stack.len(),
                vm.push_pop_stack.len(),
                vm.foreach_stack.len(),
            ],
        };
        (outcome, executed.load(Ordering::Relaxed))
    })
}

fn assert_roundtrip(source: &str, expected: &str) {
    let mut failures = Vec::new();
    for optimize in [false, true] {
        let original = compile(source, optimize);
        let (baseline, _) = execute(&original, INSTRUCTION_LIMIT);
        if baseline.result.is_err() || baseline.output != expected || baseline.frames != [0; 5] || baseline.aborted || baseline.error_pending {
            failures.push(format!("original optimize={optimize}: {baseline:?}; expected output={expected:?}"));
        }

        for raw in [false, true] {
            let (ast, issues) = decompile(reload(&original), raw, 400).expect("decompile serialized PPE");
            assert!(
                issues.is_empty(),
                "decompiler reported {} issues (raw={raw}, optimize={optimize})",
                issues.len()
            );
            let mut output = OutputVisitor::default();
            output.version = 400;
            ast.visit(&mut output);
            for reoptimize in [false, true] {
                let rebuilt = compile(&output.output, reoptimize);
                let (actual, _) = execute(&rebuilt, INSTRUCTION_LIMIT);
                if actual != baseline || actual.output != expected {
                    failures.push(format!(
                        "raw={raw}, original optimize={optimize}, rebuilt optimize={reoptimize}\nactual={actual:?}\nbaseline={baseline:?}\nexpected output={expected:?}\nppld:\n{}",
                        output.output
                    ));
                }
            }
        }
    }
    // Exercise BOTH raw modes and all optimization combinations even when an
    // earlier variant regresses, rather than hiding raw=true behind raw=false.
    assert!(failures.is_empty(), "Original:\n{source}\n{}", failures.join("\n\n"));
}

#[test]
fn legacy_ppe_fixtures_keep_output_in_legacy_and_current_source() {
    // Existing PCBoard-era fixtures supply independent bytecode patterns; no
    // DOS tooling is required to execute the regression suite.
    let bottles = (1..=100)
        .rev()
        .map(|count| {
            format!(
                "{count} Bottle(s) of beer on the wall, {count} bottle(s) of beer\nTake one down and pass it around,\n{} bottle(s) of beer on the wall\n",
                count - 1,
            )
        })
        .collect::<String>();
    for (bytes, expected) in [
        (include_bytes!("../../../../ppld/test_data/newline_stmt.ppe").as_slice(), "\n\n".to_string()),
        (include_bytes!("../../../../ppld/test_data/99bottles.ppe").as_slice(), bottles),
    ] {
        let original = Executable::from_buffer(&mut bytes.to_vec(), false).unwrap();
        let (baseline, _) = execute(&original, INSTRUCTION_LIMIT);
        assert_eq!(baseline.result, Ok(()));
        assert_eq!(baseline.output, expected);
        for language in [340, 400] {
            for raw in [false, true] {
                let (ast, issues) = decompile(original.clone(), raw, language).unwrap();
                assert!(issues.is_empty());
                let mut output = OutputVisitor::default();
                output.version = language;
                ast.visit(&mut output);
                let rebuilt = compile(&output.output, true);
                let (actual, _) = execute(&rebuilt, INSTRUCTION_LIMIT);
                assert_eq!(actual, baseline, "language={language}, raw={raw}\n{}", output.output);
            }
        }
    }
}

#[test]
fn instruction_budget_stops_non_yielding_top_level_and_routine_loops() {
    for source in [
        ":Spin\nGOTO Spin\n",
        "Spin()\nPROCEDURE Spin()\n:Again\nGOTO Again\nENDPROC\n",
        "INTEGER result\nresult = Spin()\nPRINT result\nFUNCTION Spin() INTEGER\n:Again\nGOTO Again\nENDFUNC\n",
    ] {
        let (outcome, executed) = execute(&compile(source, false), 32);
        assert_eq!(outcome.result, Err(BUDGET_ERROR.to_string()), "{source}");
        assert_eq!(executed, 33, "must stop before the 33rd source command");
        assert!(outcome.output.is_empty());
    }
}

#[test]
fn right_nested_subtraction_and_division_preserve_parentheses() {
    assert_roundtrip(
        "INTEGER a, b, c\na=20\nb=8\nc=3\nPRINTLN a-(b-c)\na=120\nb=6\nc=2\nPRINTLN a/(b/c)\nPRINTLN a/(b*c)\n",
        "15\n40\n10\n",
    );
}

#[test]
fn powers_and_same_priority_modulo_operands_keep_grouping() {
    assert_roundtrip(
        "INTEGER a, b, c\na=2\nb=3\nc=2\nPRINTLN a^(b^c)\nPRINTLN (a^b)^c\na=20\nb=8\nc=3\nPRINTLN a%(b%c)\nPRINTLN (a%b)%c\nPRINTLN a%(b*c)\nPRINTLN a*(b%c)\n",
        "512\n64\n0\n1\n20\n40\n",
    );
}

#[test]
fn fractional_factor_is_not_folded_as_integer_zero() {
    assert_roundtrip("INTEGER a, half\na=6\nhalf=0.5*a\nPRINTLN half\nPRINTLN (0.5*a)=3\n", "3\n1\n");
}

#[test]
fn arithmetic_conditions_and_mixed_boolean_operators_keep_grouping() {
    assert_roundtrip(
        r#"INTEGER a = 1, b = -1, c = 1
IF a + b PRINT "wrong;"
IF a - b PRINT "yes;"
PRINTLN (a & b) = c
PRINTLN a & (b | c)
PRINTLN !(a + b)
"#,
        "yes;1\n1\n1\n",
    );
}

#[test]
fn boolean_conditions_preserve_eager_calls_and_value_coercions() {
    assert_roundtrip(
        r#"INTEGER calls, value = 7
PRINTLN TRUE & value, ":", FALSE | value
IF FALSE & Probe() PRINT "wrong;"
IF TRUE | Probe() PRINT "yes;"
PRINTLN ":", calls
FUNCTION Probe() INTEGER
    calls = calls + 1
    PRINT "probe;"
    RETURN 7
ENDFUNC
"#,
        "1:1\nprobe;probe;yes;:2\n",
    );
}

#[test]
fn zero_times_function_keeps_call_output_and_counter() {
    assert_roundtrip(
        r#"INTEGER calls, result
result = 0 * Probe()
PRINTLN "value=", result, ",calls=", calls
FUNCTION Probe() INTEGER
    calls = calls + 1
    PRINT "probe;"
    RETURN 7
ENDFUNC
"#,
        "probe;value=0,calls=1\n",
    );
}

#[test]
fn nested_foreach_inside_while_preserves_goto_to_outer_exit() {
    assert_roundtrip(
        r#"INTEGER values[] = {1, 2}
INTEGER weights[] = {10, 20}
INTEGER round, item, weight, visits, total
WHILE round < 3 DO
    round = round + 1
    FOREACH item IN values
        FOREACH weight IN weights
            visits = visits + 1
            total = total + item + weight
            IF item = 2 GOTO OuterExit
        NEXT
    NEXT
ENDWHILE
:OuterExit
PRINTLN round, ":", visits, ":", total
FOREACH item IN values
    PRINT item
NEXT
PRINTLN ":done"
"#,
        "1:3:44\n12:done\n",
    );
}

#[test]
fn for_shaped_head_jump_skips_increment() {
    assert_roundtrip(
        r#"INTEGER i, guard, total
i = 1
:Head
IF i > 3 GOTO Done
guard = guard + 1
IF guard > 8 GOTO Done
total = total + i
IF guard = 1 GOTO Head
i = i + 1
GOTO Head
:Done
PRINTLN i, ":", guard, ":", total
"#,
        "4:4:7\n",
    );
}

#[test]
fn for_shaped_mismatched_initializer_test_and_increment_stay_distinct() {
    assert_roundtrip(
        r#"INTEGER i, j, guard, total
i = 1
j = 7
:Head
IF i > 3 GOTO Done
guard = guard + 1
IF guard > 4 GOTO Done
total = total + j
j = i + 1
GOTO Head
:Done
PRINTLN i, ":", j, ":", guard, ":", total
"#,
        "1:2:5:13\n",
    );
}

#[test]
fn positive_and_negative_for_with_nested_break_continue() {
    assert_roundtrip(
        r#"INTEGER i, j, total, guard
FOR i = 1 TO 3
    IF i = 2 CONTINUE
    FOR j = 3 TO 1 STEP -1
        guard = guard + 1
        IF guard > 20 GOTO Done
        IF j = 2 CONTINUE
        total = total + i * 10 + j
        IF i = 3 BREAK
    NEXT
NEXT
:Done
PRINTLN i, ":", j, ":", guard, ":", total
FOR i = 5 TO 1 STEP -2
    PRINT i
NEXT
PRINTLN ":", i
"#,
        "4:3:4:57\n531:-1\n",
    );
}

#[test]
fn functions_procedures_and_gosub_keep_effect_order_and_return_values() {
    assert_roundtrip(
        r#"INTEGER calls, total
total = Twice(3) - (Twice(2) - Twice(1))
Add(total)
GOSUB Finish
PRINTLN "total=", total, ",calls=", calls
EXIT
:Finish
PRINT "G;"
total = total + 1
RETURN
FUNCTION Twice(INTEGER value) INTEGER
    calls = calls + 1
    PRINT "F", value, ";"
    RETURN value * 2
ENDFUNC
PROCEDURE Add(VAR INTEGER value)
    PRINT "P;"
    value = value + Twice(4)
ENDPROC
"#,
        "F3;F2;F1;P;F4;G;total=13,calls=4\n",
    );
}
