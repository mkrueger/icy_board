use std::path::PathBuf;

use super::*;
use crate::{
    ast::{BinOp, FunctionCallExpression, IdentifierExpression, UnaryOp},
    executable::{FuncOpCode, PPEExpr},
    hir::{MemberId, UserTypeId},
    parser::{Encoding, parse_ast},
};

fn compile_source(source: &str) -> (PPECompiler, Arc<Mutex<ErrorReporter>>) {
    let workspace = Workspace::default();
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(
        PathBuf::from("hir_validation.pps"),
        errors.clone(),
        source,
        &registry,
        Encoding::Utf8,
        &workspace,
    );
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    (compiler, errors)
}

fn valid_compiler(source: &str) -> PPECompiler {
    let (compiler, errors) = compile_source(source);
    let messages: Vec<_> = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect();
    assert!(messages.is_empty(), "{messages:?}");
    compiler
}

fn reject(command: HirCommand, reason: &str) {
    let program = HirProgram {
        commands: vec![HirCommand::End, command],
    };
    let error = program.validate(2, 1).unwrap_err();
    assert_eq!(error.command_index, 1);
    assert!(error.reason.contains(reason), "{error}");
}

#[test]
fn hir_validation_walks_every_expression_child() {
    let invalid = HirExpr::Invalid;
    let valid = HirExpr::variable(1);
    for expression in [
        invalid.clone(),
        HirExpr::RecordLiteral(UserTypeId(100), vec![(MemberId(0), invalid.clone())]),
        HirExpr::member(invalid.clone(), 0),
        HirExpr::indexed_member(invalid.clone(), 0, vec![valid.clone()]),
        HirExpr::indexed_member(valid.clone(), 0, vec![invalid.clone()]),
        HirExpr::Unary(UnaryOp::Minus, Box::new(invalid.clone())),
        HirExpr::Binary(BinOp::Add, Box::new(invalid.clone()), Box::new(valid.clone())),
        HirExpr::Binary(BinOp::Add, Box::new(valid.clone()), Box::new(invalid.clone())),
        HirExpr::dim(1, vec![invalid.clone()]),
        HirExpr::predefined(FuncOpCode::LEN, vec![invalid.clone()]),
        HirExpr::function(1, vec![invalid.clone()]),
        HirExpr::member_call(invalid.clone(), vec![valid.clone()], 0),
        HirExpr::member_call(valid, vec![invalid], 0),
    ] {
        reject(HirCommand::MemberCall(expression), "HirExpr::Invalid");
    }
}

#[test]
fn hir_validation_walks_every_command_expression() {
    for command in [
        HirCommand::ConditionalGoto(HirExpr::Invalid, LabelId(0)),
        HirCommand::Let(HirExpr::Invalid, HirExpr::constant(1)),
        HirCommand::Let(HirExpr::variable(1), HirExpr::Invalid),
        HirCommand::MemberCall(HirExpr::Invalid),
        HirCommand::PredefinedCall(OpCode::PRINT, vec![HirExpr::constant(1), HirExpr::Invalid]),
        HirCommand::ProcedureCall(RoutineId(1), vec![HirExpr::Invalid]),
        HirCommand::ForEach(VariableId(1), HirExpr::Invalid, LabelId(0)),
    ] {
        reject(command, "HirExpr::Invalid");
    }
}

#[test]
fn hir_validation_rejects_zero_and_out_of_range_storage_ids() {
    for id in [0, 3, usize::MAX] {
        for expression in [HirExpr::variable(id), HirExpr::dim(id, Vec::new())] {
            reject(HirCommand::MemberCall(expression), &format!("VariableId({id})"));
        }
        reject(HirCommand::MemberCall(HirExpr::constant(id)), &format!("ConstantId({id})"));
        for expression in [HirExpr::routine_reference(id), HirExpr::function(id, Vec::new())] {
            reject(HirCommand::MemberCall(expression), &format!("RoutineId({id})"));
        }
        reject(HirCommand::ProcedureCall(RoutineId(id), Vec::new()), &format!("RoutineId({id})"));
        reject(HirCommand::OnError(HirErrorTarget::Procedure(RoutineId(id))), &format!("RoutineId({id})"));
        reject(
            HirCommand::ForEach(VariableId(id), HirExpr::variable(1), LabelId(0)),
            &format!("VariableId({id})"),
        );
    }
    assert!(
        HirProgram {
            commands: vec![HirCommand::MemberCall(HirExpr::variable(1))]
        }
        .validate(0, 0)
        .is_err()
    );
}

#[test]
fn hir_validation_checks_label_bounds_but_not_code_offsets() {
    for id in [1, usize::MAX] {
        for command in [
            HirCommand::Goto(LabelId(id)),
            HirCommand::Gosub(LabelId(id)),
            HirCommand::OnError(HirErrorTarget::Goto(LabelId(id))),
            HirCommand::OnError(HirErrorTarget::Gosub(LabelId(id))),
            HirCommand::ConditionalGoto(HirExpr::constant(1), LabelId(id)),
            HirCommand::ForEach(VariableId(1), HirExpr::variable(2), LabelId(id)),
        ] {
            reject(command, &format!("LabelId({id})"));
        }
    }
    let program = HirProgram {
        commands: vec![HirCommand::Goto(LabelId(0))],
    };
    assert!(program.validate(0, 0).is_err());
    assert!(program.validate(0, 1).is_ok());
    // These are byte offsets, NOT label-table indices or storage IDs.
    for offset in [0, 12, usize::MAX] {
        HirProgram {
            commands: vec![HirCommand::NextForEach(CodeOffset(offset))],
        }
        .validate(0, 0)
        .unwrap();
    }
}

#[test]
fn hir_validation_accepts_boundaries_and_empty_operand_lists() {
    HirProgram::default().validate(0, 0).unwrap();
    HirProgram {
        commands: vec![
            HirCommand::End,
            HirCommand::EndFunction,
            HirCommand::EndProcedure,
            HirCommand::Return,
            HirCommand::Goto(LabelId(0)),
            HirCommand::Gosub(LabelId(0)),
            HirCommand::OnError(HirErrorTarget::Off),
            HirCommand::OnError(HirErrorTarget::Goto(LabelId(0))),
            HirCommand::OnError(HirErrorTarget::Gosub(LabelId(0))),
            HirCommand::OnError(HirErrorTarget::Procedure(RoutineId(2))),
            HirCommand::ConditionalGoto(HirExpr::constant(2), LabelId(0)),
            HirCommand::Let(HirExpr::variable(1), HirExpr::constant(2)),
            HirCommand::MemberCall(HirExpr::member_call(HirExpr::variable(2), Vec::new(), 0)),
            HirCommand::PredefinedCall(OpCode::PRINTLN, Vec::new()),
            HirCommand::ProcedureCall(RoutineId(2), Vec::new()),
            HirCommand::ForEach(VariableId(2), HirExpr::variable(1), LabelId(0)),
            HirCommand::NextForEach(CodeOffset(0)),
            HirCommand::PredefinedCall(
                OpCode::PRINT,
                vec![
                    HirExpr::routine_reference(2),
                    HirExpr::function(2, Vec::new()),
                    HirExpr::predefined(FuncOpCode::TIME, Vec::new()),
                    HirExpr::dim(2, Vec::new()),
                    HirExpr::RecordLiteral(UserTypeId(100), Vec::new()),
                    HirExpr::indexed_member(HirExpr::variable(2), 0, Vec::new()),
                ],
            ),
        ],
    }
    .validate(2, 1)
    .unwrap();
}

#[test]
fn hir_validation_guards_output_even_after_invalid_was_lowered_to_value_zero() {
    let mut compiler = valid_compiler("PRINT 1\n");
    let command_index = compiler.get_hir_program().commands.len();
    compiler.add_hir_command(HirCommand::PredefinedCall(OpCode::PRINT, vec![HirExpr::Invalid]));
    let PPECommand::PredefinedCall(_, arguments) = &compiler.get_script().statements[command_index].command else {
        panic!("expected PRINT");
    };
    assert_eq!(arguments, &[PPEExpr::Value(0)]);
    for _ in 0..2 {
        let Err(CompilationErrorType::InvalidLoweredProgram { command_index: actual, reason }) = compiler.create_executable() else {
            panic!("invalid HIR must prevent output");
        };
        assert_eq!(actual, command_index);
        assert!(reason.contains("HirExpr::Invalid"));
    }
    assert!(compiler.semantic_visitor.errors.lock().unwrap().errors.is_empty());
}

#[test]
fn hir_validation_rejects_missing_call_metadata() {
    let mut compiler = valid_compiler("PRINT 1\n");
    let call = Expression::FunctionCall(FunctionCallExpression::empty(
        Expression::Identifier(IdentifierExpression::empty(unicase::Ascii::new("TIME".to_string()))),
        Vec::new(),
    ));
    // A parse-assigned call without its semantic annotation used to serialize as Value(0).
    let expression = compiler.resolve_expr(&call);
    assert_eq!(expression, HirExpr::Invalid);
    compiler.add_hir_command(HirCommand::PredefinedCall(OpCode::PRINT, vec![expression]));
    assert!(matches!(compiler.create_executable(), Err(CompilationErrorType::InvalidLoweredProgram { .. })));
}

#[test]
fn hir_validation_output_guard_uses_actual_table_sizes() {
    let mut compiler = valid_compiler("PRINT 1\n");
    let id = compiler.lookup_table.variable_table.len() + 1;
    compiler.add_hir_command(HirCommand::ProcedureCall(RoutineId(id), Vec::new()));
    assert!(
        matches!(compiler.create_executable(), Err(CompilationErrorType::InvalidLoweredProgram { reason, .. }) if reason.contains(&format!("RoutineId({id})")))
    );

    let mut compiler = valid_compiler("PRINT 1\n");
    compiler.add_hir_command(HirCommand::Goto(LabelId(compiler.label_table.len())));
    assert!(matches!(compiler.create_executable(), Err(CompilationErrorType::InvalidLoweredProgram { reason, .. }) if reason.contains("LabelId")));
}

#[test]
fn hir_validation_source_errors_block_empty_output_without_duplicate_diagnostics() {
    let (compiler, errors) = compile_source("PRINT missingValue\n");
    assert!(compiler.get_hir_program().commands.is_empty());
    assert!(compiler.get_script().statements.is_empty());
    let before: Vec<_> = errors
        .lock()
        .unwrap()
        .errors
        .iter()
        .map(|error| (error.span.clone(), error.error.to_string()))
        .collect();
    assert!(!before.is_empty());
    for _ in 0..2 {
        assert!(matches!(compiler.create_executable(), Err(CompilationErrorType::SourceErrors)));
    }
    let after: Vec<_> = errors
        .lock()
        .unwrap()
        .errors
        .iter()
        .map(|error| (error.span.clone(), error.error.to_string()))
        .collect();
    assert_eq!(before, after);
}

#[test]
fn hir_validation_valid_ppe_roundtrips_and_keeps_label_zero() {
    for source in [
        "PRINT 1\n:start\nPRINT 2\nGOTO start\n",
        ":start\nPRINT 1\nGOTO start\n",
        "INTEGER item, values[1]\nFOREACH item IN values\nPRINT item\nENDFOREACH\n",
        "PRINT Count()\nFUNCTION Count() INTEGER\nCount = 1\nENDFUNC\n",
    ] {
        let compiler = valid_compiler(source);
        let executable = compiler.create_executable().unwrap();
        assert_eq!(
            compiler.get_script().statements.len(),
            executable.in_memory_script.as_ref().unwrap().statements.len()
        );
        let mut bytes = executable.to_buffer().unwrap();
        let decoded = Executable::from_buffer(&mut bytes, false).unwrap();
        let script = PPEScript::from_ppe_file(&decoded).unwrap();
        assert!(script.bugged_offsets.is_empty());
        assert_eq!(compiler.get_script().statements.len(), script.statements.len());
        assert_eq!(bytes, decoded.to_buffer().unwrap());
        if let Some(index) = compiler
            .get_hir_program()
            .commands
            .iter()
            .position(|command| matches!(command, HirCommand::Goto(LabelId(0))))
        {
            let offset = compiler.label_table[0].offset.unwrap() * 2;
            assert_eq!(compiler.get_script().statements[index].command, PPECommand::Goto(offset));
            assert_eq!(compiler.get_hir_program().commands[index], HirCommand::Goto(LabelId(0)));
        }
    }
}
