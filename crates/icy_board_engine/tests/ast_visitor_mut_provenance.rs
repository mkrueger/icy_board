//! Generic mutation is a source-to-source walk, not synthetic AST lowering.
use std::{
    collections::HashMap,
    ops::Range,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    ast::{Ast, AstVisitorMut, RenameVisitor},
    compiler::{lower_modules, workspace::Workspace},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    semantic::SemanticVisitor,
};
use unicase::Ascii;

struct NoOp;
impl AstVisitorMut for NoOp {}

fn workspace(language: u16) -> Workspace {
    let mut workspace = Workspace::default();
    workspace.set_default_language_version(Some(language));
    workspace.package.runtime = Some(400);
    workspace
}

type Diagnostic = (PathBuf, Range<usize>, String);

fn diagnostics(errors: &Arc<Mutex<ErrorReporter>>) -> Vec<Diagnostic> {
    errors
        .lock()
        .unwrap()
        .errors
        .iter()
        .map(|error| (error.file_name.clone(), error.span.clone(), error.error.to_string()))
        .collect()
}

fn parse(source: &str, language: u16) -> (Ast, UserTypeRegistry) {
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let ast = parse_ast(
        PathBuf::from("provenance.pps"),
        errors.clone(),
        source,
        &registry,
        Encoding::Utf8,
        &workspace(language),
    );
    assert!(diagnostics(&errors).is_empty(), "fixture must parse: {source}\n{:?}", diagnostics(&errors));
    assert!(!ast.nodes.is_empty());
    (ast, registry)
}

fn assert_identical(original: &Ast, transformed: &Ast) {
    // Ast itself has no PartialEq. Node equality includes every stored token and
    // span, call/routine ID, declaration, parameter and optional syntax field.
    assert_eq!(original.nodes, transformed.nodes);
    assert_eq!(original.file_name, transformed.file_name);
    assert_eq!(original.module, transformed.module);
    assert_eq!(original.imports, transformed.imports);
    assert_eq!(original.language_version, transformed.language_version);
    assert_eq!(original.require_user_variables, transformed.require_user_variables);
    // BinaryExpression deliberately excludes its ID from PartialEq. Debug
    // includes that ID too, even inside assignment targets and nested literals.
    assert_eq!(format!("{original:?}"), format!("{transformed:?}"));
}

const FULL_SOURCE: &str = r#";$LANGVERSION 400
; source tokens must survive all default mutable visitors
TYPE Item
    INTEGER Value
ENDTYPE
ENUM Choice
    One = 1
    Two = 2
ENDENUM
DECLARE FUNCTION Compute(INTEGER number) INTEGER
DECLARE PROCEDURE Apply(VAR INTEGER number)
CONST INTEGER Seed = (1 + 2)
INTEGER counter, items[] = {-(Seed + 1), +LEN("abc")}, grid[2, 3]
Item record = Item { Value = (Seed + 1) }
BEGIN
    LET counter = items[LEN("a") - 1]
    grid[0, 1] += Compute((counter + 1))
    record.Value = (counter)
    Receiver().SecurityLevel += LEN("x")
    Board.Users[1].SecurityLevel += LEN("x")
    Session.User.Notes[0] += "note"
    items.Add((counter))
    Apply(counter)
    IF ((counter > 0)) PRINT counter
    IF counter > 0 THEN
        PRINT (counter)
    ELSEIF (counter = 0) THEN
        PRINT -counter
    ELSE
        PRINT +counter
    ENDIF
    WHILE ((counter > 0)) LET counter -= 1
    WHILE counter > 0 DO
        CONTINUE
        BREAK
    ENDWHILE
    REPEAT
        counter += 1
    UNTIL ((counter > 2))
    LOOP
        BREAK
    ENDLOOP
    FOR counter = LEN("a") TO (Seed + 1) STEP +1
        PRINT counter
    NEXT counter
    FOREACH counter IN items
        PRINT counter
    ENDFOREACH
    SELECT CASE (counter)
    CASE 1, (2)..(Seed + 1)
        PRINT counter
    DEFAULT
        PRINT 0
    ENDSELECT
    SELECT CASE counter
    DEFAULT
    ENDSELECT
    BEGIN
        GOSUB finish
        GOTO finish
        ONERROR GOTO finish
    END
    :finish
    RETURN
END
; Compute documentation
FUNCTION Compute(INTEGER number) INTEGER
    RETURN (number + LEN("abc"))
ENDFUNC
; Apply documentation
PROCEDURE Apply(VAR INTEGER number)
    number += 1
    RETURN
ENDPROC
FUNCTION Receiver() USER
    RETURN Session.User
ENDFUNC
"#;

#[test]
fn no_op_preserves_full_parsed_ast_including_spans_and_identities() {
    let (mut ast, _) = parse(FULL_SOURCE, 400);
    // Both values of this file-level flag must survive, independently of the
    // parser's user-variable discovery policy.
    for required in [false, true] {
        ast.require_user_variables = required;
        let transformed = ast.visit_mut(&mut NoOp);
        assert_identical(&ast, &transformed);
        assert_identical(&ast, &transformed.visit_mut(&mut NoOp));
    }
}

#[test]
fn no_op_preserves_module_import_visibility_and_declaration_metadata() {
    let (ast, _) = parse(
        "IMPORT Other AS O\nMODULE Example\nPRIVATE\nCONST INTEGER Seed = (1 + 2)\nPUBLIC\nINTEGER Values[] = {Seed, (Seed + 1)}\nPROCEDURE Run()\nIF (Seed) THEN\nPRINT O.Value\nENDIF\nENDPROC\nENDMODULE\n",
        400,
    );
    assert!(ast.module.is_some());
    assert_eq!(ast.imports.len(), 1);
    assert_identical(&ast, &ast.visit_mut(&mut NoOp));
}

#[test]
fn no_op_preserves_legacy_parenthesized_control_flow_and_optional_tokens() {
    let (ast, _) = parse(
        "INTEGER counter\nIF (counter) THEN\nPRINT 1\nELSEIF (counter = 1)\nPRINT 2\nENDIF\nWHILE (counter) DO\nBREAK\nENDWHILE\nFOR counter = 1 TO 2\nNEXT\nSELECT CASE counter\nCASE 1\nPRINT 1\nENDSELECT\n",
        340,
    );
    assert_identical(&ast, &ast.visit_mut(&mut NoOp));
}

#[test]
fn synthetic_empty_constructs_keep_their_exact_existing_structure() {
    use icy_board_engine::ast::{
        ArrayInitializerExpression, AstNode, BinOp, BinaryExpression, BlockStatement, BreakStatement, CaseBlock, CaseSpecifier, Constant, ConstantExpression,
        ElseBlock, ElseIfBlock, Expression, FunctionCallExpression, IdentifierExpression, IfStatement, IfThenStatement, LoopStatement, ParensExpression,
        RepeatUntilStatement, SelectStatement, Statement, UnaryExpression, UnaryOp, WhileDoStatement, WhileStatement,
    };

    let condition = Expression::Parens(ParensExpression::empty(Expression::Binary(BinaryExpression::empty(
        Expression::Unary(UnaryExpression::empty(
            UnaryOp::Not,
            Expression::Const(ConstantExpression::empty(Constant::Boolean(false))),
        )),
        BinOp::Eq,
        Expression::FunctionCall(FunctionCallExpression::empty(
            Expression::Identifier(IdentifierExpression::empty(Ascii::new("Check".to_string()))),
            vec![Expression::ArrayInitializer(ArrayInitializerExpression::empty(Vec::new()))],
        )),
    ))));
    let body = vec![Statement::Break(BreakStatement::empty())];
    let statements = vec![
        Statement::If(IfStatement::empty(condition.clone(), body[0].clone())),
        Statement::IfThen(IfThenStatement::empty(
            condition.clone(),
            body.clone(),
            vec![ElseIfBlock::empty(condition.clone(), body.clone())],
            Some(ElseBlock::empty(body.clone())),
        )),
        Statement::While(WhileStatement::empty(condition.clone(), body[0].clone())),
        Statement::WhileDo(WhileDoStatement::empty(condition.clone(), body.clone())),
        Statement::RepeatUntil(RepeatUntilStatement::empty(condition.clone(), body.clone())),
        Statement::Loop(LoopStatement::empty(body.clone())),
        Statement::Select(SelectStatement::empty(
            condition.clone(),
            vec![CaseBlock::empty(vec![CaseSpecifier::Expression(Box::new(condition))], body.clone())],
            body,
        )),
    ];
    let mut ast = Ast::new();
    ast.nodes.push(AstNode::Main(BlockStatement::empty(statements)));
    assert_identical(&ast, &ast.visit_mut(&mut NoOp));
}

#[test]
fn renaming_still_walks_children_and_leaves_labels_and_error_targets_alone() {
    let (ast, _) = parse(FULL_SOURCE, 400);
    let original = Ascii::new("counter".to_string());
    let renamed = Ascii::new("changed".to_string());
    let mut visitor = RenameVisitor::new(HashMap::from([
        (original.clone(), renamed.clone()),
        (Ascii::new("finish".to_string()), Ascii::new("wrong_label".to_string())),
    ]));
    let transformed = ast.visit_mut(&mut visitor);
    let text = transformed.to_string();
    assert!(!text.contains("counter"), "all identifier children must be visited: {text}");
    assert!(text.contains("changed"));
    assert!(!text.contains("wrong_label"), "generic identifier renaming must not rename labels");
    let restored = transformed.visit_mut(&mut RenameVisitor::new(HashMap::from([(renamed, original)])));
    assert_identical(&ast, &restored);
}

fn analyze(ast: &Ast, registry: UserTypeRegistry) -> Vec<Diagnostic> {
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut visitor = SemanticVisitor::new(&workspace(400), errors.clone(), registry);
    visitor.set_file_name(&ast.file_name);
    ast.visit(&mut visitor);
    visitor.finish();
    diagnostics(&errors)
}

#[test]
fn malformed_semantics_keep_exact_source_spans_after_generic_mutation_and_binding() {
    let header = "; source offsets deliberately start after the first line\nENUM Choice\n One = 1\nENDENUM\nChoice value = Choice.One\nINTEGER counter\n";
    for (statement, offending) in [
        ("IF ((value)) PRINT 1", "(value)"),
        ("IF ((value)) THEN\nPRINT 1\nENDIF", "(value)"),
        ("IF (TRUE) THEN\nPRINT 1\nELSEIF ((value)) THEN\nPRINT 2\nENDIF", "(value)"),
        ("WHILE ((value)) BREAK", "(value)"),
        ("WHILE ((value)) DO\nBREAK\nENDWHILE", "(value)"),
        // UNTIL parses an expression, unlike IF/WHILE's condition delimiters.
        ("REPEAT\nPRINT 1\nUNTIL ((value))", "((value))"),
        ("SELECT CASE value\nCASE (1)\nPRINT 1\nENDSELECT", "(1)"),
        ("FOREACH counter IN (1)\nPRINT counter\nENDFOREACH", "(1)"),
        ("CONST INTEGER Bad = {1, 2}", "{1, 2}"),
        ("CONST INTEGER Bad = {{1}, {2}}", "{{1}, {2}}"),
        ("LOOP\nLog \"bad\"\nBREAK\nENDLOOP", "Log"),
        ("IF (FALSE) THEN\nPRINT 1\nELSE\nLog \"bad\"\nENDIF", "Log"),
        ("FOR counter = 1 TO 2\nLog \"bad\"\nNEXT", "Log"),
        ("BEGIN\nLog \"bad\"\nEND", "Log"),
    ] {
        let source = format!("{header}{statement}\n");
        let (ast, registry) = parse(&source, 400);
        let expected_span = source.find(offending).unwrap()..source.find(offending).unwrap() + offending.len();
        // Registries are consumed by semantic analysis, not Clone. Reparse to
        // build independent registries with the same source type declarations.
        let before = analyze(&ast, parse(&source, 400).1);
        assert!(
            before.iter().any(|(_, span, _)| *span == expected_span),
            "{statement}: {before:?}; expected {expected_span:?}"
        );
        assert!(
            before
                .iter()
                .all(|(file, span, _)| *file == ast.file_name && span.start > 0 && span.end <= source.len())
        );
        let transformed = ast.visit_mut(&mut NoOp);
        assert_eq!(before, analyze(&transformed, parse(&source, 400).1), "generic mutation: {statement}");

        // The shared pre-semantic module binder inherits these defaults even
        // for sources without MODULE declarations.
        let errors = Arc::new(Mutex::new(ErrorReporter::default()));
        let bound = lower_modules(&[&ast], errors.clone(), &registry);
        assert!(diagnostics(&errors).is_empty(), "binding: {:?}", diagnostics(&errors));
        assert_eq!(before, analyze(&bound[0], registry), "module binding: {statement}");
    }
}
