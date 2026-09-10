//! Source-statement checks deliberately bypass both module and AST lowering.
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    ast::{
        Ast, AstNode, AstVisitor, BinOp, BinaryExpression, Expression, FunctionCallExpression, LetStatement, Statement, walk_binary_expression,
        walk_function_call_expression, walk_let_stmt,
    },
    compiler::workspace::Workspace,
    executable::VariableType,
    hir::CallId,
    parser::{Encoding, ErrorReporter, USER_ID, UserTypeRegistry, lexer::Token, parse_ast},
    semantic::{SemanticInfo, SemanticVisitor},
};

const ENUM: &str = "ENUM Choice\n One = 1\n Two = 2\n Both = 3\nENDENUM\n";
const RECORD: &str = "TYPE Item\n INTEGER Value\nENDTYPE\n";

fn analyze(source: &str, language: u16, runtime: u16) -> (Ast, SemanticVisitor, Vec<String>) {
    let mut workspace = Workspace::default();
    workspace.set_default_language_version(Some(language));
    workspace.package.runtime = Some(runtime);
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(PathBuf::from("source.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    let messages = || errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>();
    assert!(messages().is_empty(), "source must parse: {source}\n{:?}", messages());
    let original = format!("{ast:?}");
    let mut visitor = SemanticVisitor::new(&workspace, errors.clone(), registry);
    visitor.set_file_name(&ast.file_name);
    ast.visit(&mut visitor);
    visitor.finish();
    assert_eq!(original, format!("{ast:?}"), "semantic checking must leave the source AST intact");
    (ast, visitor, messages())
}

fn accepts(source: &str) -> (Ast, SemanticVisitor) {
    let (ast, visitor, errors) = analyze(source, 400, 400);
    assert!(errors.is_empty(), "{source}\n{errors:?}");
    (ast, visitor)
}

fn rejects(source: &str) -> Vec<String> {
    let (_, _, errors) = analyze(source, 400, 400);
    assert!(!errors.is_empty(), "accepted unchecked source: {source}");
    errors
}

#[derive(Default)]
struct ExpressionIds {
    calls: Vec<CallId>,
    binaries: Vec<u64>,
}

impl AstVisitor<()> for ExpressionIds {
    fn visit_let_statement(&mut self, statement: &LetStatement) {
        // The default walker does not traverse explicit assignment targets.
        if let Some(target) = statement.get_target_expression() {
            target.visit(self);
        }
        walk_let_stmt(self, statement);
    }

    fn visit_function_call_expression(&mut self, call: &FunctionCallExpression) {
        self.calls.push(CallId(call.id));
        walk_function_call_expression(self, call);
    }

    fn visit_binary_expression(&mut self, binary: &BinaryExpression) {
        self.binaries.push(binary.id);
        walk_binary_expression(self, binary);
    }
}

fn assert_call_metadata(ast: &Ast, visitor: &SemanticVisitor) -> Vec<CallId> {
    let mut ids = ExpressionIds::default();
    ast.visit(&mut ids);
    for id in &ids.calls {
        assert!(visitor.function_type_lookup.contains_key(id), "source call {id:?} was not resolved");
    }
    ids.calls
}

#[test]
fn for_checks_assignment_comparisons_and_increment() {
    for source in [
        "CONST INTEGER counter = 1\nFOR counter = 1 TO 2\nNEXT\n".to_string(),
        "INTEGER counter[1]\nFOR counter = 1 TO 2\nNEXT\n".to_string(),
        format!("{RECORD}Item counter\nFOR counter = counter TO counter\nNEXT\n"),
        format!("{ENUM}Choice counter\nFOR counter = Choice.One TO Choice.Two\nNEXT\n"),
        format!("{RECORD}INTEGER counter\nItem bound\nFOR counter = 1 TO bound\nNEXT\n"),
        format!("{RECORD}INTEGER counter\nItem stepValue\nFOR counter = 1 TO 2 STEP stepValue\nNEXT\n"),
        "INTEGER counter, bound[1]\nFOR counter = 1 TO bound\nNEXT\n".to_string(),
        "INTEGER counter, steps[1]\nFOR counter = 1 TO 2 STEP steps\nNEXT\n".to_string(),
    ] {
        rejects(&source);
    }
}

#[test]
fn for_visits_each_source_bound_and_keeps_call_metadata() {
    let source = "INTEGER counter\nFOR counter = LEN(\"a\") TO LEN(\"abc\") STEP LEN(\"x\")\nPRINT counter\nNEXT\n";
    let (ast, visitor) = accepts(source);
    assert_eq!(3, assert_call_metadata(&ast, &visitor).len());
    let (_, _, errors) = analyze("FOR missing = badStart TO badEnd STEP badStep\nPRINT badBody\nNEXT\n", 400, 400);
    for name in ["missing", "badStart", "badEnd", "badStep", "badBody"] {
        assert!(errors.iter().any(|error| error.contains(name)), "missing diagnostic for {name}: {errors:?}");
    }
}

#[test]
fn for_does_not_introduce_a_numeric_only_counter_policy() {
    for language in [340, 350, 400] {
        let (_, _, errors) = analyze("STRING counter\nFOR counter = 1 TO 2 STEP 1\nNEXT\n", language, 400);
        assert!(errors.is_empty(), "language={language}: {errors:?}");
    }
}

#[test]
fn compound_assignments_check_operations_not_just_assignment_types() {
    for statement in ["a += b", "a -= b", "a *= b", "a /= b", "a %= b", "a &= b", "a |= b"] {
        let errors = rejects(&format!("{RECORD}Item a, b\n{statement}\n"));
        assert!(errors.iter().any(|error| error.contains("not defined for custom types")), "{errors:?}");
    }
    rejects("INTEGER items[1]\nitems += items\n");
    rejects(&format!("{RECORD}Item items[1]\nitems[0] += items[1]\n"));
    rejects("TYPE Container\n INTEGER Values[1]\nENDTYPE\nContainer value\nvalue.Values += value.Values\n");
    accepts("INTEGER items[1]\nitems[0] += LEN(\"abc\")\n");
    accepts(&format!("{RECORD}Item items[1]\nitems[0].Value += LEN(\"abc\")\n"));
}

#[test]
fn compound_enum_checks_preserve_real_binary_ids_only() {
    let source = format!("{ENUM}Choice value = Choice.One\nvalue |= Choice.Two\nPRINT value & Choice.One\n");
    let (ast, visitor) = accepts(&source);
    let mut ids = ExpressionIds::default();
    ast.visit(&mut ids);
    assert_eq!(1, ids.binaries.len());
    assert_eq!(1, visitor.enum_binary_types.len(), "implicit operations must not leave phantom AST IDs");
    assert!(visitor.enum_binary_types.contains_key(&ids.binaries[0]));
    rejects(&format!("{ENUM}Choice value\nvalue |= 1\n"));
    rejects(&format!("{ENUM}Choice value\nvalue += Choice.One\n"));
    let (_, _, errors) = analyze(&format!("{ENUM}Choice value\nvalue |= Choice.One\n"), 350, 350);
    assert!(errors.iter().any(|error| error.contains("Checked enum operation")), "{errors:?}");
}

#[test]
fn compound_function_result_reads_keep_source_span_metadata() {
    let source = "PRINT Count()\nFUNCTION Count() INTEGER\n Count = 1\n Count += 2\nENDFUNC\n";
    let (_, visitor) = accepts(source);
    assert!(visitor.is_function_return_value(source.find("Count +=").unwrap()));
}

#[test]
fn compound_member_setters_check_rhs_and_keep_receiver_and_call_metadata() {
    let suffix = "\nFUNCTION Receiver() USER\n RETURN Session.User\nENDFUNC\n";
    let source = format!("Receiver().SecurityLevel += LEN(\"x\"){suffix}");
    let (ast, visitor) = accepts(&source);
    let statement = first_statement(&ast);
    let Statement::Let(assignment) = statement else {
        panic!("call-rooted member assignment must retain an explicit LET target: {statement:?}");
    };
    assert_eq!(Token::AddAssign, assignment.get_eq_token().token);
    let Some(Expression::MemberReference(member)) = assignment.get_target_expression() else {
        panic!("expected member target: {assignment:?}");
    };
    let Expression::FunctionCall(receiver) = member.get_expression() else {
        panic!("expected source receiver call: {member:?}");
    };
    let ids = assert_call_metadata(&ast, &visitor);
    assert_eq!(2, ids.len(), "both Receiver() and LEN() must be collected");
    assert!(matches!(
        visitor.function_type_lookup.get(&CallId(receiver.id)),
        Some(SemanticInfo::FunctionReference(_))
    ));
    assert_eq!(Some(&(USER_ID as u32)), visitor.user_type_lookup.get(&source.find("SecurityLevel").unwrap()));
    assert_eq!(Some(&VariableType::Integer), visitor.compound_target_types.get(&0));
    assert!(
        !ids.iter()
            .any(|id| matches!(visitor.function_type_lookup.get(id), Some(SemanticInfo::MemberSetterCall(_))))
    );

    // A board-rooted indexed receiver takes the actual MemberSetterCall path.
    let source = "Board.Users[1].SecurityLevel += LEN(\"x\")\n";
    let (ast, visitor) = accepts(source);
    let Statement::MemberCall(statement) = first_statement(&ast) else {
        panic!("expected board-rooted setter call: {ast:?}");
    };
    let Expression::FunctionCall(setter) = statement.get_expression() else {
        panic!("expected setter expression: {statement:?}");
    };
    assert_eq!(Token::AddAssign, setter.get_lpar_token().token);
    let ids = assert_call_metadata(&ast, &visitor);
    assert_eq!(3, ids.len(), "setter, indexed receiver and RHS call must all be resolved");
    assert!(matches!(
        visitor.function_type_lookup.get(&CallId(setter.id)),
        Some(SemanticInfo::MemberSetterCall(_))
    ));
    assert_eq!(Some(&(USER_ID as u32)), visitor.user_type_lookup.get(&source.find("SecurityLevel").unwrap()));
    let errors = rejects(&format!("{ENUM}Receiver().SecurityLevel += Choice.One{suffix}"));
    assert!(errors.iter().any(|error| error.contains("not defined for custom types")), "{errors:?}");
    rejects(&format!("{RECORD}Item value\nReceiver().SecurityLevel += value{suffix}"));
    rejects(&format!("INTEGER values[1]\nReceiver().SecurityLevel += values{suffix}"));
    rejects(&format!("{ENUM}Board.Users[1].SecurityLevel += Choice.One\n"));
    rejects(&format!("{RECORD}Item value\nBoard.Users[1].SecurityLevel += value\n"));
    rejects("INTEGER values[1]\nBoard.Users[1].SecurityLevel += values\n");
    rejects("Session.User.Contacts[0] += 1\n");
    rejects("STRING text = \"abc\"\ntext.Substring(0)[0] += \"x\"\n");
}

fn first_statement(ast: &Ast) -> &Statement {
    ast.nodes
        .iter()
        .find_map(|node| match node {
            AstNode::TopLevelStatement(statement) => Some(statement),
            AstNode::Main(main) => main.get_statements().first(),
            _ => None,
        })
        .expect("source statement")
}

#[test]
fn structured_conditions_reject_arrays_including_elseif() {
    for statement in [
        "IF (values) PRINT 1",
        "IF (values) THEN\nPRINT 1\nENDIF",
        "IF (TRUE) THEN\nPRINT 1\nELSEIF (values) THEN\nPRINT 2\nENDIF",
        "WHILE (values) BREAK",
        "WHILE (values) DO\nBREAK\nENDWHILE",
        "REPEAT\nPRINT 1\nUNTIL (values)",
    ] {
        rejects(&format!("INTEGER values[1]\n{statement}\n"));
    }
}

#[test]
fn conditions_keep_existing_enum_and_legacy_scalar_rules() {
    for statement in [
        "IF (value) PRINT 1",
        "IF (value) THEN\nPRINT 1\nENDIF",
        "IF (TRUE) THEN\nPRINT 1\nELSEIF (value) THEN\nPRINT 2\nENDIF",
        "WHILE (value) BREAK",
        "WHILE (value) DO\nBREAK\nENDWHILE",
        "REPEAT\nPRINT 1\nUNTIL (value)",
    ] {
        rejects(&format!("{ENUM}Choice value\n{statement}\n"));
        accepts(&format!("INTEGER value\n{statement}\n"));
    }
    // Unlike structured IF, this branch does not introduce an implicit NOT.
    accepts(&format!("{ENUM}Choice value\nIF (value) GOTO done\n:done\n"));
    accepts(&format!("{ENUM}Choice value\nIF (value = Choice.One) PRINT 1\n"));
}

#[test]
fn select_checks_equality_and_range_operand_rules() {
    accepts(&format!(
        "{ENUM}Choice value\nSELECT CASE value\nCASE Choice.One, Choice.Two\nPRINT 1\nENDSELECT\n"
    ));
    rejects(&format!("{ENUM}Choice value\nSELECT CASE value\nCASE 1\nPRINT 1\nENDSELECT\n"));
    rejects(&format!(
        "{ENUM}Choice value\nSELECT CASE value\nCASE Choice.One..Choice.Two\nPRINT 1\nENDSELECT\n"
    ));
    accepts(&format!("{RECORD}Item a, b\nSELECT CASE a\nCASE b\nPRINT 1\nENDSELECT\n"));
    rejects(&format!("{RECORD}Item a\nSELECT CASE a\nCASE 1\nPRINT 1\nENDSELECT\n"));
    rejects(&format!("{RECORD}Item a, b\nSELECT CASE a\nCASE a..b\nPRINT 1\nENDSELECT\n"));
    rejects(&format!("{RECORD}Item a[1], b[1]\nSELECT CASE a\nCASE b\nPRINT 1\nENDSELECT\n"));
    rejects("INTEGER a, values[1]\nSELECT CASE a\nCASE values\nPRINT 1\nENDSELECT\n");
    accepts("INTEGER a\nSELECT CASE a\nCASE 1..2\nPRINT 1\nENDSELECT\n");
}

#[test]
fn select_visits_original_calls_even_without_cases() {
    let (ast, visitor) = accepts("SELECT CASE LEN(\"abc\")\nCASE LEN(\"a\"), LEN(\"ab\")..LEN(\"abc\")\nPRINT 1\nENDSELECT\n");
    assert_eq!(4, assert_call_metadata(&ast, &visitor).len());
    let (ast, visitor) = accepts("SELECT CASE LEN(\"abc\")\nENDSELECT\n");
    assert_eq!(1, assert_call_metadata(&ast, &visitor).len());
    rejects("SELECT CASE unknown\nENDSELECT\n");
}

#[test]
fn source_control_flow_does_not_hide_invalid_branches() {
    for source in [
        "IF (FALSE) PRINT missing",
        "IF (FALSE) THEN\nPRINT missing\nENDIF",
        "WHILE (FALSE) PRINT missing",
        "WHILE (FALSE) DO\nPRINT missing\nENDWHILE",
        "REPEAT\nPRINT missing\nUNTIL (TRUE)",
        "SELECT CASE 1\nCASE 2\nPRINT missing\nENDSELECT",
        "LOOP\nBREAK\nPRINT missing\nENDLOOP",
    ] {
        let errors = rejects(&format!("{source}\n"));
        assert!(errors.iter().any(|error| error.contains("missing")), "{errors:?}");
    }
}

#[test]
fn break_and_continue_keep_the_existing_noop_outside_loops_policy() {
    accepts("BREAK\nCONTINUE\nPRINT 1\n");
    accepts("SELECT CASE 1\nCASE 1\nBREAK\nCONTINUE\nENDSELECT\n");
    accepts("INTEGER item, values[1]\nFOREACH item IN values\nLOOP\nCONTINUE\nBREAK\nENDLOOP\nCONTINUE\nBREAK\nENDFOREACH\n");
}

#[test]
fn return_values_use_function_result_assignment_checks() {
    rejects(&format!("{ENUM}FUNCTION GetChoice() Choice\n RETURN 1\nENDFUNC\n"));
    rejects(&format!("{RECORD}FUNCTION GetItem() Item\n RETURN 1\nENDFUNC\n"));
    rejects("FUNCTION GetItems() INTEGER[]\n RETURN 1\nENDFUNC\n");
    rejects("FUNCTION GetItem() INTEGER\n INTEGER values[1]\n RETURN values\nENDFUNC\n");
    let source = "INTEGER values[] = GetItems()\nFUNCTION GetItems() INTEGER[]\n INTEGER result[1]\n RETURN result\nENDFUNC\n";
    let (ast, visitor) = accepts(source);
    assert_call_metadata(&ast, &visitor);
    assert!(visitor.references.iter().any(|(_, reference)| {
        reference
            .return_types
            .iter()
            .any(|(_, location)| location.span.start == source.find("RETURN result").unwrap())
    }));
}

#[test]
fn source_initializers_use_scalar_and_array_assignment_checks() {
    rejects(&format!("{RECORD}Item value = 1\n"));
    rejects(&format!("{RECORD}Item value\nINTEGER number = value\n"));
    rejects(&format!("{ENUM}Choice values[] = {{Choice.One, 1}}\n"));
    rejects("INTEGER source[1]\nINTEGER value = source\n");
    rejects("INTEGER values[] = 1\n");
    accepts(&format!("{ENUM}Choice values[] = {{Choice.One, Choice.Two}}\n"));
    accepts("INTEGER source[1]\nINTEGER copy[] = source\n");
    // Like the lowered declaration followed by LET, the variable is in scope.
    accepts("INTEGER value = value + 1\n");
    let (ast, visitor) = accepts("INTEGER values[] = {LEN(\"a\"), LEN(\"abc\")}\n");
    assert_eq!(2, assert_call_metadata(&ast, &visitor).len());
}

#[test]
fn binary_results_cannot_bypass_custom_return_or_initializer_checks() {
    for expression in ["1 + 2", "LEN(\"abc\") - 1", "(1 + 2) * 3", "1 < 2", "\"a\" + \"b\""] {
        rejects(&format!("{RECORD}FUNCTION GetItem() Item\n RETURN {expression}\nENDFUNC\n"));
        rejects(&format!("{RECORD}Item value = {expression}\n"));
        rejects(&format!("{RECORD}Item values[] = {{{expression}}}\n"));
        rejects(&format!("{ENUM}FUNCTION GetChoice() Choice\n RETURN {expression}\nENDFUNC\n"));
        rejects(&format!("{ENUM}Choice value = {expression}\n"));
        // Preserve legacy scalar assignment coercions, including STRING counters.
        accepts(&format!(
            "STRING value = {expression}\nFUNCTION GetValue() INTEGER\n RETURN {expression}\nENDFUNC\n"
        ));
    }
    accepts(&format!(
        "{RECORD}Item copy = GetItem()\nFUNCTION GetItem() Item\n Item value\n RETURN value\nENDFUNC\n"
    ));
    accepts(&format!(
        "{ENUM}Choice value = Choice.One | Choice.Two\nFUNCTION GetChoice() Choice\n RETURN Choice.One | Choice.Two\nENDFUNC\n"
    ));
}

#[test]
fn inferred_scalar_binary_types_match_runtime_arithmetic() {
    struct CheckTypes<'a>(&'a mut SemanticVisitor, usize);
    impl AstVisitor<()> for CheckTypes<'_> {
        fn visit_binary_expression(&mut self, binary: &BinaryExpression) {
            let left = binary.get_left_expression().visit(self.0).create_empty_value();
            let right = binary.get_right_expression().visit(self.0).create_empty_value();
            let expected = match binary.get_op() {
                BinOp::Add => (left + right).get_type(),
                BinOp::Sub => (left - right).get_type(),
                BinOp::Mul => (left * right).get_type(),
                BinOp::Div => (left / right).get_type(),
                BinOp::Mod => (left % right).get_type(),
                BinOp::PoW => left.pow(right).get_type(),
                _ => VariableType::Boolean,
            };
            assert_eq!(expected, self.0.visit_binary_expression(binary), "{binary:?}");
            self.1 += 1;
        }
    }

    let types = [
        "BOOLEAN", "UNSIGNED", "DATE", "EDATE", "INTEGER", "MONEY", "FLOAT", "STRING", "TIME", "BYTE", "WORD", "SBYTE", "SWORD", "BIGSTR", "DOUBLE", "DDATE",
        "LONG", "ULONG",
    ];
    let operators = ["+", "-", "*", "/", "%", "^", "=", "<>", "<", "<=", ">", ">=", "&", "|"];
    let mut source = String::new();
    for (index, ty) in types.iter().enumerate() {
        source.push_str(&format!("{ty} operand{index}\n"));
    }
    for left in 0..types.len() {
        for right in 0..types.len() {
            for operator in operators {
                source.push_str(&format!("PRINT operand{left} {operator} operand{right}\n"));
            }
        }
    }
    let (ast, mut visitor) = accepts(&source);
    let mut check = CheckTypes(&mut visitor, 0);
    ast.visit(&mut check);
    assert_eq!(types.len() * types.len() * operators.len(), check.1);

    // Before 4.00 STRING denotes the bounded legacy string type.
    let (ast, mut visitor, errors) = analyze("STRING lhs, rhs\nPRINT lhs + rhs, lhs - rhs, lhs = rhs\n", 340, 400);
    assert!(errors.is_empty(), "{errors:?}");
    let mut check = CheckTypes(&mut visitor, 0);
    ast.visit(&mut check);
    assert_eq!(3, check.1);
}
