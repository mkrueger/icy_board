use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    ast::AstNode,
    compiler::{CompilationErrorType, PPECompiler, lower_modules, workspace::Workspace},
    executable::{EntryType, Executable, PPECommand, PPEExpr, PPEScript, VariableType},
    icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState, user_base::User},
    parser::{Encoding, ErrorReporter, ParserErrorType, UserTypeRegistry, parse_ast},
    vm::io::DiskIO,
};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};

const TARGETS: [(u16, u16); 4] = [(340, 340), (340, 400), (350, 350), (350, 400)];
const KIND_PROC: &str = include_str!("../../../compat/declare/kind_proc.pps");
const KIND_PROC_VAR: &str = include_str!("../../../compat/declare/kind_proc_var.pps");
const KIND_FUNC: &str = include_str!("../../../compat/declare/kind_func.pps");

fn compile(sources: &[(&str, &str)], language: u16, runtime: u16) -> Result<Executable, Vec<String>> {
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let registry = UserTypeRegistry::icy_board_registry();
    let mut workspace = Workspace::default();
    workspace.set_default_language_version(Some(language));
    workspace.package.runtime = Some(runtime);
    let asts: Vec<_> = sources
        .iter()
        .map(|(name, source)| parse_ast(PathBuf::from(name), errors.clone(), source, &registry, Encoding::Utf8, &workspace))
        .collect();
    if !errors.lock().unwrap().has_errors() {
        let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
        compiler.compile(&asts.iter().collect::<Vec<_>>());
        if !errors.lock().unwrap().has_errors() {
            let executable = compiler.create_executable().unwrap();
            let mut bytes = executable.to_buffer().unwrap();
            let executable = Executable::from_buffer(&mut bytes, false).unwrap();
            let script = PPEScript::from_ppe_file(&executable).expect("emitted commands must decode, without opcode-zero corruption");
            assert!(script.bugged_offsets.is_empty(), "{:?}", script.bugged_offsets);
            assert_eq!(executable.script_buffer, script.serialize());
            for statement in &script.statements {
                assert_ne!(0, executable.script_buffer[statement.span.start], "{:?}", statement.command);
            }
            return Ok(executable);
        }
    }
    Err(errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect())
}

fn accepted(source: &str, language: u16, runtime: u16) -> Executable {
    compile(&[("legacy_kind.pps", source)], language, runtime).unwrap_or_else(|errors| panic!("language={language}, runtime={runtime}: {errors:?}\n{source}"))
}

fn rejected(source: &str, language: u16, runtime: u16) -> Vec<String> {
    match compile(&[("legacy_kind.pps", source)], language, runtime) {
        Ok(_) => panic!("unexpected success: language={language}, runtime={runtime}\n{source}"),
        Err(errors) => errors,
    }
}

async fn run(executable: &Executable) -> String {
    let directory = tempfile::tempdir().unwrap();
    let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
    let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
    let nodes = bbs.lock().await.open_connections.clone();
    let (mut peer, connection) = ChannelConnection::create_pair();
    let mut board = IcyBoard::new();
    board.root_path = directory.path().to_path_buf();
    board.default_display_text = icy_board_engine::icy_board::icb_text::DEFAULT_DISPLAY_TEXT.clone();
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
            let result = icy_board_engine::vm::run(&PathBuf::from("legacy_kind.ppe"), executable, &mut io, &mut state).await;
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
async fn oracle_procedure_declaration_emits_procedure_without_function_result() {
    for (language, runtime) in TARGETS {
        let executable = accepted(KIND_PROC, language, runtime);
        let entries = executable.variable_table.get_entries();
        let procedures: Vec<_> = entries.iter().filter(|entry| entry.header.variable_type == VariableType::Procedure).collect();
        assert_eq!(1, procedures.len());
        assert!(entries.iter().all(|entry| entry.header.variable_type != VariableType::Function));
        assert!(entries.iter().all(|entry| entry.get_type() != EntryType::FunctionResult));
        let id = procedures[0].header.id as usize;
        let script = PPEScript::from_ppe_file(&executable).unwrap();
        assert!(
            script
                .statements
                .iter()
                .any(|statement| matches!(statement.command, PPECommand::ProcedureCall(target, _) if target == id))
        );
        assert!(
            script
                .statements
                .iter()
                .any(|statement| matches!(&statement.command, PPECommand::Let(target, _) if **target == PPEExpr::Value(id)))
        );
        assert!(script.statements.iter().any(|statement| statement.command == PPECommand::EndProc));
        assert!(script.statements.iter().all(|statement| statement.command != PPECommand::EndFunc));
        assert_eq!("", run(&executable).await);
    }
}

#[tokio::test]
async fn implementation_controls_var_modes_even_with_function_keyword() {
    for (language, runtime) in TARGETS {
        for declared_var in [false, true] {
            for implemented_var in [false, true] {
                let source = KIND_PROC_VAR
                    .replace(
                        "DECLARE PROCEDURE Work(INTEGER value)",
                        &format!("DECLARE PROCEDURE Work({}INTEGER value)", if declared_var { "VAR " } else { "" }),
                    )
                    .replace(
                        "FUNCTION Work(VAR INTEGER value)",
                        &format!("FUNCTION Work({}INTEGER value)", if implemented_var { "VAR " } else { "" }),
                    );
                let executable = accepted(&source, language, runtime);
                let procedure = executable
                    .variable_table
                    .get_entries()
                    .iter()
                    .find(|entry| entry.header.variable_type == VariableType::Procedure)
                    .unwrap();
                assert_eq!(u16::from(implemented_var), unsafe { procedure.value.data.procedure_value.pass_flags });
                assert_eq!(if implemented_var { "9\n" } else { "1\n" }, run(&executable).await);
            }
        }
    }
}

#[tokio::test]
async fn scalar_routine_writes_evaluate_rhs_and_do_not_corrupt_repeated_calls() {
    let source = r#"
DECLARE PROCEDURE Work()
DECLARE FUNCTION Side() INTEGER
INTEGER calls
Side = 123
Work()
Work()
PRINTLN calls
END
FUNCTION Work() INTEGER
Work = Side()
RETURN
PRINTLN "unreachable"
ENDFUNC
FUNCTION Side() INTEGER
calls = calls + 1
Side = 7
ENDFUNC
"#;
    for (language, runtime) in TARGETS {
        assert_eq!("2\n", run(&accepted(source, language, runtime)).await);
        let return_expression = source.replace("Work = Side()\nRETURN", "RETURN Side()");
        assert_eq!("2\n", run(&accepted(&return_expression, language, runtime)).await);
    }
    // The descriptor protection also applies to genuine runtime/language 400 routines.
    let strict = source
        .replace("FUNCTION Work() INTEGER", "PROCEDURE Work()")
        .replace("ENDFUNC\nFUNCTION Side", "ENDPROC\nFUNCTION Side")
        .replace("\nEND\n", "\nEXIT\n");
    assert_eq!("2\n", run(&accepted(&strict, 400, 400)).await);
}

#[test]
fn reverse_mismatch_strict400_and_parameter_count_mismatch_remain_errors() {
    for (language, runtime) in TARGETS {
        assert!(rejected(KIND_FUNC, language, runtime).contains(&CompilationErrorType::ProcedureUsedAsFunction.to_string()));
        let source = KIND_PROC
            .replace("DECLARE PROCEDURE Work()", "DECLARE PROCEDURE Work(INTEGER value)")
            .replace("Work()\nEND", "Work(1)\nEND");
        assert!(rejected(&source, language, runtime).iter().any(|error| error.contains("parameters not match")));
    }
    for source in [KIND_PROC, KIND_FUNC] {
        let strict = source.replace("\nEND\n", "\nEXIT\n");
        assert!(rejected(&strict, 400, 400).contains(&CompilationErrorType::ProcedureUsedAsFunction.to_string()));
    }
    assert!(rejected(KIND_PROC_VAR, 400, 400).contains(&ParserErrorType::VarNotAllowedInFunctions.to_string()));
}

#[test]
fn genuine_function_var_headers_and_declarations_still_error() {
    let source = "DECLARE FUNCTION Work(INTEGER value) INTEGER\nPRINTLN Work(1)\nEND\nFUNCTION Work(VAR INTEGER value) INTEGER\nWork = value\nENDFUNC\n";
    for (language, runtime) in TARGETS.into_iter().chain([(400, 400)]) {
        assert!(rejected(source, language, runtime).contains(&ParserErrorType::VarNotAllowedInFunctions.to_string()));
        let declaration_var = source
            .replace("DECLARE FUNCTION Work(INTEGER", "DECLARE FUNCTION Work(VAR INTEGER")
            .replace("FUNCTION Work(VAR INTEGER value) INTEGER\nWork", "FUNCTION Work(INTEGER value) INTEGER\nWork");
        assert!(rejected(&declaration_var, language, runtime).contains(&ParserErrorType::VarNotAllowedInFunctions.to_string()));
        if language >= 350 {
            let implicit = source.lines().skip(1).collect::<Vec<_>>().join("\n");
            assert!(rejected(&implicit, language, runtime).contains(&ParserErrorType::VarNotAllowedInFunctions.to_string()));
        }
    }
}

#[tokio::test]
async fn implicit_legacy_function_is_not_normalized_from_a_call() {
    for runtime in [350, 400] {
        let executable = accepted("PRINTLN Work()\nEND\nFUNCTION Work() INTEGER\nWork = 7\nENDFUNC\n", 350, runtime);
        assert!(
            executable
                .variable_table
                .get_entries()
                .iter()
                .any(|entry| entry.header.variable_type == VariableType::Function)
        );
        assert_eq!("7\n", run(&executable).await);
    }
}

#[test]
fn shared_lowering_preserves_source_spans_documentation_and_original_ast() {
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let registry = UserTypeRegistry::icy_board_registry();
    let mut workspace = Workspace::default();
    workspace.set_default_language_version(Some(340));
    let mut ast = parse_ast(
        PathBuf::from("legacy_kind.pps"),
        errors.clone(),
        KIND_PROC_VAR,
        &registry,
        Encoding::Utf8,
        &workspace,
    );
    let function = ast
        .nodes
        .iter_mut()
        .find_map(|node| if let AstNode::Function(function) = node { Some(function) } else { None })
        .unwrap();
    function.set_documentation("Legacy procedure documentation".to_string());
    let function = function.clone();
    assert!(function.get_parameters()[0].is_var());
    let lowered = lower_modules(&[&ast], errors.clone(), &registry);
    assert!(!errors.lock().unwrap().has_errors());
    let procedure = lowered[0]
        .nodes
        .iter()
        .find_map(|node| if let AstNode::Procedure(procedure) = node { Some(procedure) } else { None })
        .unwrap();
    assert_eq!(function.get_identifier_token(), procedure.get_identifier_token());
    assert_eq!(function.get_function_token().span, procedure.get_procedure_token().span);
    assert_eq!(function.get_endfunc_token().span, procedure.get_endproc_token().span);
    assert_eq!(function.get_documentation(), procedure.get_documentation());
    assert!(procedure.get_parameters()[0].is_var());
    assert!(ast.nodes.iter().any(|node| matches!(node, AstNode::Function(_))));
    assert_eq!(lowered[0].nodes, lower_modules(&[&lowered[0]], errors, &registry)[0].nodes);
}

#[tokio::test]
async fn module_qualification_keeps_unrelated_function_kind_and_initializer_checks() {
    let sources = [
        ("main.pps", "IMPORT Legacy AS L\nIMPORT Genuine AS G\nL.Work()\nPRINTLN G.Work()\n"),
        (
            "legacy.pps",
            "MODULE Legacy\nDECLARE PROCEDURE Work()\nFUNCTION Work() INTEGER\nWork = 7\nENDFUNC\nENDMODULE\n",
        ),
        ("genuine.pps", "MODULE Genuine\nFUNCTION Work() INTEGER\nWork = 9\nENDFUNC\nENDMODULE\n"),
    ];
    let executable = compile(&sources, 350, 400).unwrap_or_else(|errors| panic!("{errors:?}"));
    assert_eq!("9\n", run(&executable).await);
    let invalid = sources[1].1.replace("ENDFUNC\nENDMODULE", "ENDFUNC\nINTEGER value = Work()\nENDMODULE");
    let errors = match compile(&[sources[0], ("legacy.pps", &invalid), sources[2]], 350, 400) {
        Ok(_) => panic!("runtime module initializer was accepted"),
        Err(errors) => errors,
    };
    assert!(errors.iter().any(|error| error.contains("must be constant")), "{errors:?}");
}

#[tokio::test]
async fn normalized_procedure_return_expression_has_no_result_slot() {
    for (language, runtime) in TARGETS {
        let source = KIND_PROC.replace("Work = 7", "RETURN 7");
        let executable = accepted(&source, language, runtime);
        assert!(
            executable
                .variable_table
                .get_entries()
                .iter()
                .all(|entry| entry.get_type() != EntryType::FunctionResult)
        );
        assert_eq!("", run(&executable).await);
    }
}

#[tokio::test]
async fn same_kind_routine_reference_assignments_remain_valid() {
    let source = r#"
First = 0
First()
Placeholder = 0
PRINTLN Placeholder()
Second()
PRINTLN Answer()
PROCEDURE First()
PRINT "first"
ENDPROC
PROCEDURE Second()
PRINT "second"
ENDPROC
FUNCTION Placeholder() INTEGER
RETURN 1
ENDFUNC
FUNCTION Answer() INTEGER
RETURN 42
ENDFUNC
"#;
    let mut executable = accepted(source, 400, 400);
    let mut script = PPEScript::from_ppe_file(&executable).unwrap();
    let procedures: Vec<_> = script
        .statements
        .iter()
        .filter_map(|statement| match statement.command {
            PPECommand::ProcedureCall(id, _) => Some(id),
            _ => None,
        })
        .collect();
    let functions: Vec<_> = script
        .statements
        .iter()
        .filter_map(|statement| match &statement.command {
            PPECommand::PredefinedCall(_, arguments) => arguments.iter().find_map(|argument| match argument {
                PPEExpr::FunctionCall(id, _) => Some(*id),
                _ => None,
            }),
            _ => None,
        })
        .collect();
    assert_eq!(2, procedures.len());
    assert_eq!(2, functions.len());
    let mut assignments = [(procedures[0], procedures[1]), (functions[0], functions[1])].into_iter();
    // Exercise the VM operation directly: source-level callback assignment has
    // separate semantic restrictions. Both RHS encodings occupy two words, so
    // replacing them preserves every compiled routine and jump offset.
    for statement in &mut script.statements {
        if let PPECommand::Let(target, value) = &mut statement.command
            && let Some((destination, source)) = assignments.next()
        {
            // Function-name source assignments can resolve to a result slot;
            // explicitly address the descriptor for this bytecode-level test.
            **target = PPEExpr::Value(destination);
            **value = PPEExpr::RoutineReference(source);
        }
        assert_eq!(statement.span.len(), statement.command.get_size());
    }
    assert!(assignments.next().is_none());
    executable.script_buffer = script.serialize();
    let mut bytes = executable.to_buffer().unwrap();
    let executable = Executable::from_buffer(&mut bytes, false).unwrap();
    PPEScript::from_ppe_file(&executable).unwrap();
    assert_eq!("second42\nsecond42\n", run(&executable).await);
}
