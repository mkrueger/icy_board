//! Exercise the public checked-source boundary, not a hand-built/lowered AST.
use std::{
    ops::Range,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    ast::{
        Ast, AstVisitor, BinaryExpression, ForStatement, FunctionCallExpression, IdentifierExpression, walk_binary_expression, walk_for_stmt,
        walk_function_call_expression,
    },
    compiler::{CompilationErrorType, PPECompiler, workspace::Workspace},
    executable::{EntryType, Executable, OpCode, VariableType},
    hir::{CallId, HirCommand, HirExpr},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast_with_predeclared_types, preparse_type_declarations},
    semantic::SemanticVisitor,
};

type Reporter = Arc<Mutex<ErrorReporter>>;

#[derive(Debug, PartialEq, Eq)]
struct Diagnostic {
    file: PathBuf,
    span: Range<usize>,
    message: String,
}

fn diagnostics(errors: &Reporter) -> Vec<Diagnostic> {
    errors
        .lock()
        .unwrap()
        .errors
        .iter()
        .map(|error| Diagnostic {
            file: error.file_name.clone(),
            span: error.span.clone(),
            message: error.error.to_string(),
        })
        .collect()
}

fn parse(sources: &[(&str, &str)]) -> (Workspace, UserTypeRegistry, Reporter, Vec<Ast>) {
    let mut workspace = Workspace::default();
    workspace.set_default_language_version(Some(400));
    workspace.package.runtime = Some(400);
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    for (file, source) in sources {
        preparse_type_declarations(PathBuf::from(file), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    }
    let asts = sources
        .iter()
        .map(|(file, source)| parse_ast_with_predeclared_types(PathBuf::from(file), errors.clone(), source, &registry, Encoding::Utf8, &workspace))
        .collect();
    assert!(diagnostics(&errors).is_empty(), "fixtures must parse: {sources:?}\n{:?}", diagnostics(&errors));
    (workspace, registry, errors, asts)
}

fn analyze(sources: &[(&str, &str)]) -> (SemanticVisitor, bool, Reporter) {
    let (workspace, registry, errors, asts) = parse(sources);
    let original = format!("{asts:?}");
    let mut visitor = SemanticVisitor::new(&workspace, errors.clone(), registry);
    let checked = visitor.analyze_sources(&asts.iter().collect::<Vec<_>>());
    assert_eq!(original, format!("{asts:?}"), "checking must preserve original source ASTs, IDs and spans");
    (visitor, checked.is_valid(), errors)
}

fn compile(sources: &[(&str, &str)]) -> (PPECompiler, Reporter) {
    let (workspace, registry, errors, asts) = parse(sources);
    let original = format!("{asts:?}");
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    // This is the public compile API: it analyzes source once and only lowers a
    // valid CheckedProgram. There is no public compile_checked entry point.
    compiler.compile(&asts.iter().collect::<Vec<_>>());
    assert_eq!(
        original,
        format!("{asts:?}"),
        "compilation must not replace the caller's ASTs with lowered trees"
    );
    (compiler, errors)
}

fn successful(sources: &[(&str, &str)]) -> (PPECompiler, Executable) {
    let (compiler, errors) = compile(sources);
    assert!(diagnostics(&errors).is_empty(), "{sources:?}\n{:?}", diagnostics(&errors));
    let executable = compiler.create_executable().unwrap_or_else(|error| panic!("{sources:?}\n{error:?}"));
    assert!(!compiler.get_hir_program().commands.is_empty(), "valid source must reach HIR emission");
    (compiler, executable)
}

fn rejected(sources: &[(&str, &str)]) -> Vec<Diagnostic> {
    let (_, valid, analysis_errors) = analyze(sources);
    let expected = diagnostics(&analysis_errors);
    assert!(!valid, "invalid source crossed the checked boundary: {sources:?}\n{expected:?}");
    assert!(!expected.is_empty(), "invalid CheckedProgram needs a source diagnostic");
    // Use a fresh reporter so comparison also detects duplicate semantic walks.
    let (compiler, errors) = compile(sources);
    assert_eq!(expected, diagnostics(&errors), "compiler and checked-source diagnostics must agree");
    assert!(compiler.get_hir_program().commands.is_empty(), "invalid source must not emit even partial HIR");
    assert!(compiler.get_script().statements.is_empty(), "invalid source must not emit PPE commands");
    assert!(matches!(compiler.create_executable(), Err(CompilationErrorType::SourceErrors)));
    assert_eq!(expected, diagnostics(&errors), "output rejection must not duplicate diagnostics");
    expected
}

#[derive(Default, Debug, PartialEq)]
struct SourceShape {
    calls: Vec<CallId>,
    operators: Vec<(u64, Range<usize>)>,
    identifiers: Vec<(String, Range<usize>)>,
    for_loops: usize,
}

impl AstVisitor<()> for SourceShape {
    fn visit_function_call_expression(&mut self, call: &FunctionCallExpression) {
        self.calls.push(CallId(call.id));
        walk_function_call_expression(self, call);
    }

    fn visit_binary_expression(&mut self, binary: &BinaryExpression) {
        self.operators.push((binary.id, binary.get_op_token().span.clone()));
        walk_binary_expression(self, binary);
    }

    fn visit_identifier_expression(&mut self, identifier: &IdentifierExpression) {
        self.identifiers
            .push((identifier.get_identifier().to_string(), identifier.get_identifier_token().span.clone()));
    }

    fn visit_for_statement(&mut self, statement: &ForStatement) {
        self.for_loops += 1;
        walk_for_stmt(self, statement);
    }
}

#[test]
fn checked_source_preserves_high_level_structure_and_parse_assigned_call_ids() {
    let source = "CONST INTEGER Limit = 1 + 2\nINTEGER counter\nFOR counter = LEN(\"a\") TO Limit\nPRINTLN counter\nNEXT\n";
    let sources = [("source.pps", source)];
    let (workspace, registry, errors, asts) = parse(&sources);
    let original = format!("{asts:?}");
    let mut before = SourceShape::default();
    asts[0].visit(&mut before);
    assert_eq!(before.for_loops, 1, "fixture must contain a parsed FOR, not GOTOs");
    assert_eq!(before.calls.len(), 1, "fixture must contain the original LEN call");
    let operator = source.find('+').unwrap();
    assert_eq!(before.operators.len(), 1, "constant declaration must retain its binary expression");
    assert_eq!(before.operators[0].1, operator..operator + 1);

    let mut visitor = SemanticVisitor::new(&workspace, errors.clone(), registry);
    let checked = visitor.analyze_sources(&[&asts[0]]);
    assert!(checked.is_valid(), "{:?}", diagnostics(&errors));
    for id in &before.calls {
        assert!(visitor.function_type_lookup.contains_key(id), "source call {id:?} lost its checked annotation");
    }
    let mut after = SourceShape::default();
    asts[0].visit(&mut after);
    assert_eq!(before, after);
    assert_eq!(original, format!("{asts:?}"));

    let (compiler, _) = successful(&sources);
    assert!(
        compiler
            .get_hir_program()
            .commands
            .iter()
            .any(|command| matches!(command, HirCommand::ConditionalGoto(_, _))),
        "the valid source FOR must actually be lowered"
    );
}

#[test]
fn invalid_dead_branch_blocks_lowering_of_the_entire_package() {
    let bad = "IF (FALSE) THEN\nPRINTLN missingValue\nENDIF\n";
    let good = "INTEGER counter\nFOR counter = 1 TO 3\nPRINTLN counter\nNEXT\n";
    for sources in [[("bad.pps", bad), ("good.pps", good)], [("good.pps", good), ("bad.pps", bad)]] {
        let errors = rejected(&sources);
        let start = bad.find("missingValue").unwrap();
        assert_eq!(
            errors,
            vec![Diagnostic {
                file: PathBuf::from("bad.pps"),
                span: start..start + "missingValue".len(),
                message: "Variable not found (missingValue)".to_string(),
            }]
        );
    }
}

#[test]
fn enum_constant_operator_errors_keep_exact_source_spans_before_folding() {
    let declarations = "ENUM Choice\n One = 1\n Two = 2\nENDENUM\nCONST Choice First = Choice.One\nCONST Choice Second = Choice.Two\n";
    for (expression, operator, message) in [
        ("First + Second", "+", "Operator + is not defined for custom types"),
        ("First < Second", "<", "Operator < is not defined for custom types"),
        ("First = 1", "=", "Can't compare Choice with Integer"),
    ] {
        // FALSE would discard this statement after optimization; substituting
        // enum constants first would instead hide its nominal type violation.
        let source = format!("{declarations}IF (FALSE) PRINTLN {expression}\n");
        let errors = rejected(&[("operators.pps", &source)]);
        let start = source.find(expression).unwrap() + expression.find(operator).unwrap();
        assert_eq!(
            errors,
            vec![Diagnostic {
                file: PathBuf::from("operators.pps"),
                span: start..start + operator.len(),
                message: message.to_string(),
            }],
            "{source}"
        );
    }
}

#[test]
fn compound_operator_diagnostic_covers_the_original_two_byte_token_once() {
    let source = "TYPE Item\n INTEGER Value\nENDTYPE\nItem a, b\nIF (FALSE) THEN\na += b\nENDIF\n";
    let errors = rejected(&[("compound.pps", source)]);
    let start = source.find("+=").unwrap();
    assert_eq!(
        errors,
        vec![Diagnostic {
            file: PathBuf::from("compound.pps"),
            span: start..start + 2,
            message: "Operator + is not defined for custom types".to_string(),
        }]
    );
}

#[test]
fn duplicate_names_are_reported_once_before_constant_substitution() {
    for declarations in ["CONST INTEGER Limit = 1 + 2\nINTEGER Limit\n", "INTEGER Limit\nCONST INTEGER Limit = 1 + 2\n"] {
        for local in [false, true] {
            let source = if local {
                format!("Show()\nPROCEDURE Show()\n{declarations}PRINTLN Limit\nENDPROC\n")
            } else {
                format!("{declarations}PRINTLN Limit\n")
            };
            let start = source.match_indices("Limit").nth(1).unwrap().0;
            let errors = rejected(&[("duplicate.pps", &source)]);
            assert_eq!(
                errors,
                vec![Diagnostic {
                    file: PathBuf::from("duplicate.pps"),
                    span: start..start + "Limit".len(),
                    message: "Variable name already used (Limit)".to_string(),
                }],
                "{source}"
            );
        }
    }
}

// Pad real source text, never fabricate AST nodes or overwrite token spans.
fn at_offset(prefix: &str, identifier_and_suffix: &str, offset: usize) -> String {
    assert!(prefix.len() <= offset);
    format!("{prefix}{}{identifier_and_suffix}", " ".repeat(offset - prefix.len()))
}

#[test]
fn module_callback_result_and_plain_variable_flags_are_isolated_at_the_same_offset() {
    const OFFSET: usize = 256;
    let result = at_offset(
        "MODULE Results\nFUNCTION Count() INTEGER\n Count = 7\n PRINTLN ",
        "Count\nENDFUNC\nENDMODULE\n",
        OFFSET,
    );
    let callback = at_offset(
        "MODULE Callbacks\nFUNCTION Target() INTEGER\n RETURN 9\nENDFUNC\nPROCEDURE Run()\n Apply(",
        "Target)\nENDPROC\nPROCEDURE Apply(FUNCTION callback() INTEGER)\n PRINTLN callback()\nENDPROC\nENDMODULE\n",
        OFFSET,
    );
    let root = at_offset(
        "IMPORT Results AS R\nIMPORT Callbacks AS C\nINTEGER Plain\nPRINTLN R.Count()\nC.Run()\nPRINTLN ",
        "Plain\n",
        OFFSET,
    );
    // Reverse the module ordering too: stale callback permission must not turn
    // the other file's function-result read into a routine reference.
    for sources in [
        [
            ("root.pps", root.as_str()),
            ("result.pps", result.as_str()),
            ("callback.pps", callback.as_str()),
        ],
        [
            ("callback.pps", callback.as_str()),
            ("result.pps", result.as_str()),
            ("root.pps", root.as_str()),
        ],
    ] {
        let (_, _, _, asts) = parse(&sources);
        for ast in &asts {
            let name = match ast.file_name.to_str().unwrap() {
                "result.pps" => "Count",
                "callback.pps" => "Target",
                _ => "Plain",
            };
            let mut shape = SourceShape::default();
            ast.visit(&mut shape);
            assert!(
                shape.identifiers.contains(&(name.to_string(), OFFSET..OFFSET + name.len())),
                "fixture must parse an actual identifier at the colliding offset: {}\n{shape:?}",
                ast.file_name.display()
            );
        }
        let (mut visitor, valid, errors) = analyze(&sources);
        assert!(valid, "{:?}", diagnostics(&errors));
        for (file, routine_reference, function_result) in [
            ("callback.pps", true, false),
            ("result.pps", false, true),
            ("root.pps", false, false),
            ("result.pps", false, true),
            ("callback.pps", true, false),
        ] {
            visitor.select_source_file(Path::new(file));
            assert_eq!(visitor.is_routine_reference(OFFSET), routine_reference, "routine-reference flags for {file}");
            assert_eq!(visitor.is_function_return_value(OFFSET), function_result, "function-result flags for {file}");
        }

        let (compiler, executable) = successful(&sources);
        let commands = &compiler.get_hir_program().commands;
        let callback_arguments: Vec<_> = commands
            .iter()
            .filter_map(|command| match command {
                HirCommand::ProcedureCall(_, arguments) if arguments.len() == 1 => Some(&arguments[0]),
                _ => None,
            })
            .collect();
        assert_eq!(callback_arguments.len(), 1, "{commands:?}");
        let HirExpr::RoutineReference(target) = callback_arguments[0] else {
            panic!("callback must reference the routine, not its result variable: {commands:?}");
        };
        let target_entry = executable.variable_table.get_var_entry(target.0);
        assert_eq!(target_entry.entry_type, EntryType::Function, "{target_entry:?}");
        assert!(target_entry.name.ends_with("_Target"), "{target_entry:?}");

        let printed_variables: Vec<_> = commands
            .iter()
            .filter_map(|command| match command {
                HirCommand::PredefinedCall(OpCode::PRINTLN, arguments) => match arguments.as_slice() {
                    [HirExpr::Variable(id)] => Some(executable.variable_table.get_var_entry(id.0)),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        assert_eq!(
            printed_variables.len(),
            2,
            "plain and function-result reads must both remain variables: {commands:?}"
        );
        assert!(
            printed_variables
                .iter()
                .any(|entry| entry.name == "Plain" && entry.entry_type == EntryType::Variable),
            "{printed_variables:?}"
        );
        assert!(
            printed_variables
                .iter()
                .any(|entry| entry.name.ends_with("_Count result") && entry.header.variable_type == VariableType::Integer),
            "function-result read must resolve the integer result slot: {printed_variables:?}"
        );
    }
}

#[test]
fn lazy_constant_pool_labels_never_shadow_global_or_local_source_variables() {
    for (source, entry_type) in [
        ("INTEGER CONST_2\nPRINTLN 1\nPRINTLN CONST_2\n", EntryType::Variable),
        (
            "Show()\nPROCEDURE Show()\nINTEGER CONST_2\nPRINTLN 1\nPRINTLN CONST_2\nENDPROC\n",
            EntryType::LocalVariable,
        ),
        (
            "PRINTLN Show()\nFUNCTION Show() INTEGER\nINTEGER CONST_2\nPRINTLN 1\nPRINTLN CONST_2\nRETURN CONST_2\nENDFUNC\n",
            EntryType::LocalVariable,
        ),
    ] {
        let (compiler, executable) = successful(&[("constants.pps", source)]);
        let commands = &compiler.get_hir_program().commands;
        let prints: Vec<_> = commands
            .iter()
            .filter_map(|command| match command {
                HirCommand::PredefinedCall(OpCode::PRINTLN, arguments) => match arguments.as_slice() {
                    [value @ (HirExpr::Constant(_) | HirExpr::Variable(_))] => Some(value),
                    _ => None,
                },
                _ => None,
            })
            .collect();
        assert_eq!(prints.len(), 2, "{source}\n{commands:?}");
        let (HirExpr::Constant(literal), HirExpr::Variable(variable)) = (prints[0], prints[1]) else {
            panic!("PRINTLN 1 must emit a constant, PRINTLN CONST_2 a variable: {source}\n{commands:?}");
        };
        assert_ne!(
            literal.0, variable.0,
            "pool interning must not overwrite the source-name lookup: {source}\n{commands:?}"
        );
        let literal_entry = executable.variable_table.get_var_entry(literal.0);
        assert_eq!(literal_entry.name, "CONST_2", "fixture must trigger the first lazy pool-label collision");
        assert_eq!(literal_entry.entry_type, EntryType::Constant, "{literal_entry:?}");
        assert_eq!(literal_entry.value.as_int(), 1);
        let variable_entry = executable.variable_table.get_var_entry(variable.0);
        assert_eq!(variable_entry.name, "CONST_2", "{variable_entry:?}");
        assert_eq!(
            variable_entry.entry_type, entry_type,
            "HIR Variable must not secretly point to a pool entry: {source}\n{variable_entry:?}"
        );
        assert_eq!(variable_entry.header.variable_type, VariableType::Integer);
    }
}
