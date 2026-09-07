//! Backend regressions for typed constants after source semantic analysis.
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    compiler::{PPECompiler, workspace::Workspace},
    executable::{Executable, FuncOpCode, OpCode},
    hir::{HirCommand, HirExpr},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
};

fn compile(source: &str) -> PPECompiler {
    let workspace = Workspace::default();
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(PathBuf::from("fractal.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&[&ast]);
    let messages = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>();
    assert!(messages.is_empty(), "{messages:?}");
    compiler
}

fn valid(expression: &HirExpr) -> bool {
    match expression {
        HirExpr::Invalid => false,
        HirExpr::Variable(id) => id.0 != 0,
        HirExpr::Constant(id) => id.0 != 0,
        HirExpr::RoutineReference(id) => id.0 != 0,
        HirExpr::RecordLiteral(_, fields) => fields.iter().all(|(_, expression)| valid(expression)),
        HirExpr::Member(base, _) | HirExpr::Unary(_, base) => valid(base),
        HirExpr::Binary(_, left, right) => valid(left) && valid(right),
        HirExpr::Dim(id, arguments) => id.0 != 0 && arguments.iter().all(valid),
        HirExpr::FunctionCall(id, arguments) => id.0 != 0 && arguments.iter().all(valid),
        HirExpr::PredefinedCall(_, arguments) => arguments.iter().all(valid),
        HirExpr::IndexedMember(base, _, arguments) | HirExpr::MemberCall(base, arguments, _) => valid(base) && arguments.iter().all(valid),
    }
}

fn assert_valid_hir(compiler: &PPECompiler) {
    for (index, command) in compiler.get_hir_program().commands.iter().enumerate() {
        let is_valid = match command {
            HirCommand::Let(target, value) => valid(target) && valid(value),
            HirCommand::ConditionalGoto(condition, _) | HirCommand::MemberCall(condition) | HirCommand::ForEach(_, condition, _) => valid(condition),
            HirCommand::PredefinedCall(_, arguments) | HirCommand::ProcedureCall(_, arguments) => arguments.iter().all(valid),
            _ => true,
        };
        assert!(is_valid, "command {index}: {command:?}");
    }
}

#[test]
fn fractal_compiles_without_invalid_expressions() {
    let source = include_str!("../../../ppe/fractal/src/fractal.pps");
    for source in [source.to_string(), source.replace("WHILE TRUE DO", "WHILE FALSE DO")] {
        let compiler = compile(&source);
        assert_valid_hir(&compiler);
        let executable = compiler.create_executable().unwrap();
        let mut bytes = executable.to_buffer().unwrap();
        Executable::from_buffer(&mut bytes, false).unwrap();
    }
}

#[test]
fn double_constants_keep_precision_and_local_shadowing() {
    let compiler = compile(
        "CONST DOUBLE coordinate = -0.743643887\nPRINT coordinate\nShow()\nEXIT\n\
         PROCEDURE Show()\nCONST DOUBLE coordinate = 0.131825904\nPRINT coordinate\nENDPROC\n",
    );
    assert_valid_hir(&compiler);
    let executable = compiler.create_executable().unwrap();
    let arguments: Vec<_> = compiler
        .get_hir_program()
        .commands
        .iter()
        .filter_map(|command| {
            if let HirCommand::PredefinedCall(OpCode::PRINT, arguments) = command {
                Some(arguments)
            } else {
                None
            }
        })
        .collect();
    assert_eq!(arguments.len(), 2);
    for (arguments, expected) in arguments.into_iter().zip([-0.743643887_f64, 0.131825904_f64]) {
        let [HirExpr::PredefinedCall(FuncOpCode::TODREAL, operands)] = arguments.as_slice() else {
            panic!("DOUBLE must use the precise string conversion: {arguments:?}")
        };
        let [HirExpr::Constant(id)] = operands.as_slice() else {
            panic!("DOUBLE conversion must have a literal operand: {operands:?}")
        };
        let text = executable.variable_table.get_var_entry(id.0).value.as_string();
        assert_eq!(text.parse::<f64>().unwrap(), expected);
    }
}
