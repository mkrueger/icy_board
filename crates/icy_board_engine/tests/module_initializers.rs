//! Module initializers are checked before folding or dead-code elimination.
//! `PPECompiler::with_optimization` is currently private and cfg(test), so an
//! integration test cannot turn optimization off. Exercise the public lowering
//! pass directly as well as the default optimized compiler; do not pretend that
//! direct lowering is an unoptimized end-to-end compilation.

use std::{
    ops::Range,
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    ast::{Ast, ModuleDeclaration},
    compiler::{CompilationErrorType, PPECompiler, lower_modules, workspace::Workspace},
    executable::{Executable, PPECommand},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast_with_predeclared_types, preparse_type_declarations},
};

#[derive(Debug)]
struct Diagnostic {
    file: PathBuf,
    span: Range<usize>,
    message: String,
    initializer: Option<String>,
}

fn diagnostics(errors: &Arc<Mutex<ErrorReporter>>) -> Vec<Diagnostic> {
    errors
        .lock()
        .unwrap()
        .errors
        .iter()
        .map(|error| Diagnostic {
            file: error.file_name.clone(),
            span: error.span.clone(),
            message: error.error.to_string(),
            initializer: match error.error.downcast_ref::<CompilationErrorType>() {
                Some(CompilationErrorType::ModuleInitializerMustBeConstant(name)) => Some(name.clone()),
                _ => None,
            },
        })
        .collect()
}

fn parse(
    sources: &[(&str, &str)],
    implicit: Option<(&str, &str)>,
    workspace: &Workspace,
    registry: &UserTypeRegistry,
    errors: &Arc<Mutex<ErrorReporter>>,
) -> Vec<Ast> {
    // Complete the package-wide type prepass before parsing even the first file.
    for (file, source) in sources {
        preparse_type_declarations(PathBuf::from(file), errors.clone(), source, registry, Encoding::Utf8, workspace);
    }
    let asts = sources
        .iter()
        .map(|(file, source)| {
            let mut ast = parse_ast_with_predeclared_types(PathBuf::from(file), errors.clone(), source, registry, Encoding::Utf8, workspace);
            // Plain scalar library fixtures need no predeclared module types.
            if let Some((library_file, module)) = implicit
                && *file == library_file
            {
                assert!(ast.module.is_none());
                ast.module = Some(ModuleDeclaration::implicit(module));
            }
            ast
        })
        .collect();
    assert!(diagnostics(errors).is_empty(), "fixture must parse: {:?}", diagnostics(errors));
    asts
}

struct Compilation {
    diagnostics: Vec<Diagnostic>,
    executable: Option<Executable>,
    commands: Vec<PPECommand>,
}

fn compile(sources: &[(&str, &str)], implicit: Option<(&str, &str)>) -> Compilation {
    let workspace = Workspace::default();
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let asts = parse(sources, implicit, &workspace, &registry, &errors);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&asts.iter().collect::<Vec<_>>());
    let diagnostics = diagnostics(&errors);
    let executable = diagnostics
        .is_empty()
        .then(|| compiler.create_executable().expect("valid program must emit a PPE"));
    let commands = compiler.get_script().statements.iter().map(|statement| statement.command.clone()).collect();
    Compilation {
        diagnostics,
        executable,
        commands,
    }
}

fn lower(sources: &[(&str, &str)], implicit: Option<(&str, &str)>) -> Vec<Diagnostic> {
    let workspace = Workspace::default();
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let asts = parse(sources, implicit, &workspace, &registry, &errors);
    lower_modules(&asts.iter().collect::<Vec<_>>(), errors.clone(), &registry);
    diagnostics(&errors)
}

fn successful(sources: &[(&str, &str)], implicit: Option<(&str, &str)>) -> Compilation {
    let lowering_errors = lower(sources, implicit);
    assert!(lowering_errors.is_empty(), "unexpected lowering diagnostics: {lowering_errors:?}");
    let result = compile(sources, implicit);
    assert!(result.diagnostics.is_empty(), "{:?}", result.diagnostics);
    let executable = result.executable.as_ref().unwrap();
    let mut bytes = executable.to_buffer().expect("serialize initialized globals");
    let loaded = Executable::from_buffer(&mut bytes, false).expect("reload initialized globals");
    assert_eq!(executable.script_buffer, loaded.script_buffer);
    assert_eq!(executable.user_types, loaded.user_types);
    result
}

fn assert_initializer_error(errors: &[Diagnostic], file: &str, source: &str, name: &str, expression: &str) {
    let matches = errors.iter().filter(|error| error.initializer.as_deref() == Some(name)).collect::<Vec<_>>();
    assert_eq!(1, matches.len(), "expected one initializer diagnostic for {name}: {errors:?}");
    let error = matches[0];
    // A bare variable read may also appear in its earlier declaration.
    let start = source.rfind(expression).expect("expression must occur verbatim in fixture");
    assert_eq!(PathBuf::from(file), error.file, "{error:?}");
    assert_eq!(start..start + expression.len(), error.span, "{error:?}");
    assert!(error.message.contains(&format!("'{name}'")), "original name must remain readable: {error:?}");
    assert!(error.message.contains("must be constant"), "{error:?}");
    assert!(!error.message.contains("__M"), "lowered names must not leak: {error:?}");
}

fn forbidden(declarations: &str, variable_type: &str, expression: &str) {
    for implicit in [false, true] {
        let body = format!(";$USEFUNCS\n{declarations}\n{variable_type} OriginalName = {expression}\n");
        let source = if implicit { body } else { format!("MODULE Values\n{body}ENDMODULE\n") };
        let sources = [
            ("application.pps", "IMPORT Values AS V\nPRINT V.OriginalName\n"),
            ("library.pps", source.as_str()),
        ];
        let module = implicit.then_some(("library.pps", "Values"));
        let before_optimization = lower(&sources, module);
        assert_eq!(1, before_optimization.len(), "{expression}, implicit={implicit}: {before_optimization:?}");
        assert_initializer_error(&before_optimization, "library.pps", &source, "OriginalName", expression);
        let result = compile(&sources, module);
        assert_initializer_error(&result.diagnostics, "library.pps", &source, "OriginalName", expression);
        assert!(result.executable.is_none());
    }
}

#[test]
fn literals_operators_private_constants_and_uninitialized_globals_compile() {
    successful(
        &[
            ("application.pps", "IMPORT Values AS V\nPRINT V.Answer, V.Caption, V.Enabled, V.Unset\n"),
            (
                "library.pps",
                "MODULE Values\nPRIVATE\nCONST INTEGER Seed = 7\nCONST INTEGER Twice = Seed * 2\nPUBLIC\nINTEGER Answer = -(Twice + 1) + +3\nSTRING Caption = \"hello\" + \" world\"\nBOOLEAN Enabled = !FALSE & (TRUE | FALSE)\nINTEGER Unset\nENDMODULE\n",
            ),
        ],
        None,
    );
}

#[test]
fn imported_alias_constants_and_enums_are_resolved_by_the_real_registry() {
    successful(
        &[
            ("application.pps", "IMPORT Values AS V\nPRINT V.Answer, V.Shade, V.Favorite\n"),
            (
                "definitions.pps",
                "MODULE Definitions\nCONST INTEGER Seed = 21\nENUM Color\n Red\n Green = 5\nENDENUM\nCONST Color Preferred = Color.Green\nENDMODULE\n",
            ),
            (
                "library.pps",
                "IMPORT Definitions AS D\nMODULE Values\nINTEGER Answer = D.Seed * 2\nD.Color Shade = D.Color.Green\nD.Color Favorite = D.Preferred\nENDMODULE\n",
            ),
        ],
        None,
    );
}

#[test]
fn nested_record_and_array_literals_accept_constant_leaves() {
    successful(
        &[
            (
                "application.pps",
                "IMPORT Values AS V\nPRINT V.Items[0].Child.Number, V.Numbers[1], V.Empty.Child.Number\n",
            ),
            (
                "library.pps",
                "MODULE Values\nCONST INTEGER Seed = 7\nTYPE Inner\n INTEGER Number\nENDTYPE\nTYPE Outer\n Inner Child\nENDTYPE\nOuter Items[] = { Outer { Child = Inner { Number = -(Seed + 1) } }, Outer { Child = Inner { Number = 42 } } }\nINTEGER Numbers[] = { Seed, 2 * (Seed + 1) }\nOuter Empty = Outer {}\nENDMODULE\n",
            ),
        ],
        None,
    );
}

#[test]
fn builtin_enum_members_are_constants_too() {
    successful(
        &[
            ("application.pps", "IMPORT Values AS V\nPRINT V.Comparison\n"),
            (
                "library.pps",
                "MODULE Values\nStringComparison Comparison = StringComparison.OrdinalIgnoreCase\nENDMODULE\n",
            ),
        ],
        None,
    );
}

#[test]
fn explicit_and_implicit_library_initializers_are_emitted_before_root_code() {
    for implicit in [false, true] {
        let body = "CONST INTEGER Seed = 21\nINTEGER Answer = Seed * 2\nINTEGER Unset\n";
        let source = if implicit {
            body.to_string()
        } else {
            format!("MODULE Values\n{body}ENDMODULE\n")
        };
        let result = successful(
            &[("application.pps", "IMPORT Values AS V\nPRINT V.Answer, V.Unset\n"), ("library.pps", &source)],
            implicit.then_some(("library.pps", "Values")),
        );
        assert!(
            matches!(result.commands.first(), Some(PPECommand::Let(_, _))),
            "initializer must precede the root PRINT"
        );
        let initializer = result.commands.iter().position(|command| matches!(command, PPECommand::Let(_, _))).unwrap();
        let end = result.commands.iter().position(|command| matches!(command, PPECommand::End)).unwrap();
        assert!(initializer < end, "initializer must be reachable");
    }
}

#[test]
fn constant_expression_initializers_emit_the_same_bytecode_as_literals() {
    let root = "IMPORT Values AS V\nPRINT V.Answer\n";
    let expression = successful(
        &[
            ("application.pps", root),
            (
                "library.pps",
                "MODULE Values\nCONST INTEGER Seed = 7\nINTEGER Answer = (Seed + 1) * 2\nENDMODULE\n",
            ),
        ],
        None,
    );
    let literal = successful(
        &[("application.pps", root), ("library.pps", "MODULE Values\nINTEGER Answer = 16\nENDMODULE\n")],
        None,
    );
    assert_eq!(
        expression.executable.unwrap().to_buffer().unwrap(),
        literal.executable.unwrap().to_buffer().unwrap()
    );
}

#[test]
fn a_zero_argument_pure_user_function_is_still_a_call() {
    forbidden("FUNCTION Pure() INTEGER\n RETURN 42\nENDFUNC\n", "INTEGER", "Pure()");
}

#[test]
fn builtin_and_static_or_instance_member_calls_are_not_constants() {
    for (declarations, variable_type, expression) in [
        ("", "INTEGER", "ABS(-7)"),
        ("", "INTEGER", "LEN(\"abc\")"),
        ("", "STRING", "STRING.Repeat(\"x\", 2)"),
        ("STRING Text = \"abc\"", "INTEGER", "Text.Len()"),
        ("", "INTEGER", "Session.Area.HighMsg()"),
    ] {
        forbidden(declarations, variable_type, expression);
    }
}

#[test]
fn mutable_variables_and_array_elements_are_not_constants() {
    forbidden("INTEGER Other = 7", "INTEGER", "Other");
    forbidden("INTEGER Data[] = { 7, 8 }", "INTEGER", "Data[0]");
    forbidden("INTEGER Data[] = { 7, 8 }", "INTEGER", "Data(0)");
}

#[test]
fn board_and_session_reads_are_not_constants() {
    forbidden("", "STRING", "Board.Conferences[0].Name");
    forbidden("", "STRING", "Session.User.Name");
}

#[test]
fn folding_cannot_hide_a_forbidden_read_or_call() {
    forbidden("INTEGER Other = 7", "INTEGER", "0 * Other");
    forbidden("BOOLEAN Flag = TRUE", "BOOLEAN", "FALSE & Flag");
    forbidden("FUNCTION Pure() INTEGER\n RETURN 42\nENDFUNC\n", "INTEGER", "0 * Pure()");
    forbidden("FUNCTION Pure() BOOLEAN\n RETURN TRUE\nENDFUNC\n", "BOOLEAN", "FALSE & Pure()");
}

#[test]
fn recursive_validation_rejects_runtime_leaves_inside_aggregates() {
    for expression in [
        "{ Outer { Child = Inner { Number = Other } } }",
        "{ Outer { Child = Inner { Number = ABS(-1) } } }",
        "{ Outer { Child = Inner { Number = 0 * Other } } }",
    ] {
        let source = format!(
            "MODULE Values\nTYPE Inner\n INTEGER Number\nENDTYPE\nTYPE Outer\n Inner Child\nENDTYPE\nINTEGER Other = 7\nOuter OriginalName[] = {expression}\nENDMODULE\n"
        );
        let sources = [
            ("application.pps", "IMPORT Values AS V\nPRINT V.OriginalName[0].Child.Number\n"),
            ("library.pps", source.as_str()),
        ];
        assert_initializer_error(&lower(&sources, None), "library.pps", &source, "OriginalName", expression);
        assert_initializer_error(&compile(&sources, None).diagnostics, "library.pps", &source, "OriginalName", expression);
    }
}

#[test]
fn record_field_reads_are_not_enum_members() {
    let source =
        "MODULE Values\nTYPE Item\n INTEGER Number\nENDTYPE\nItem RecordValue = Item { Number = 7 }\nINTEGER OriginalName = RecordValue.Number\nENDMODULE\n";
    let sources = [("application.pps", "IMPORT Values AS V\nPRINT V.OriginalName\n"), ("library.pps", source)];
    assert_initializer_error(&lower(&sources, None), "library.pps", source, "OriginalName", "RecordValue.Number");
    assert_initializer_error(
        &compile(&sources, None).diagnostics,
        "library.pps",
        source,
        "OriginalName",
        "RecordValue.Number",
    );
}

#[test]
fn imported_mutable_values_and_functions_are_rejected_at_the_consuming_file() {
    for expression in ["D.Value", "D.Pure()"] {
        let source = format!("IMPORT Definitions AS D\nMODULE Values\nINTEGER OriginalName = {expression}\nENDMODULE\n");
        let sources = [
            ("application.pps", "IMPORT Values AS V\nPRINT V.OriginalName\n"),
            (
                "definitions.pps",
                "MODULE Definitions\nINTEGER Value = 7\nFUNCTION Pure() INTEGER\n RETURN 42\nENDFUNC\nENDMODULE\n",
            ),
            ("library.pps", source.as_str()),
        ];
        assert_initializer_error(&lower(&sources, None), "library.pps", &source, "OriginalName", expression);
        assert_initializer_error(&compile(&sources, None).diagnostics, "library.pps", &source, "OriginalName", expression);
    }
}

#[test]
fn routine_local_and_ordinary_application_initializers_remain_runtime_expressions() {
    successful(
        &[
            (
                "application.pps",
                "IMPORT Values AS V\nINTEGER RuntimeValue = V.Pure()\nINTEGER Copy = RuntimeValue\nSTRING Name = Session.User.Name\nV.Show()\nPRINT RuntimeValue, Copy, Name\n",
            ),
            (
                "library.pps",
                "MODULE Values\nINTEGER Seed = 7\nFUNCTION Pure() INTEGER\n INTEGER LocalValue = Seed + ABS(-1)\n RETURN LocalValue\nENDFUNC\nPROCEDURE Show()\n INTEGER LocalValue = Pure()\n STRING Name = Session.User.Name\n PRINT LocalValue, Name\nENDPROC\nENDMODULE\n",
            ),
        ],
        None,
    );
}

#[test]
fn a_const_declaration_does_not_launder_a_runtime_expression() {
    for value in ["Other + 1", "ABS(-7)"] {
        let source = format!("MODULE Values\nINTEGER Other = 7\nCONST INTEGER Invalid = {value}\nINTEGER Answer = Invalid\nENDMODULE\n");
        let result = compile(&[("application.pps", "IMPORT Values AS V\nPRINT V.Answer\n"), ("library.pps", &source)], None);
        assert!(
            result
                .diagnostics
                .iter()
                .any(|error| error.message.contains("A constant needs a value the compiler can work out")),
            "invalid CONST must still fail ordinary semantic validation: {:?}",
            result.diagnostics
        );
        assert!(result.executable.is_none());
    }
}

#[test]
fn each_invalid_declarator_keeps_its_own_name_and_expression_span() {
    let source = "MODULE Values\nINTEGER Other = 7\nINTEGER First = Other + 1, Second = ABS(-2)\nENDMODULE\n";
    let sources = [("application.pps", "IMPORT Values AS V\nPRINT V.First, V.Second\n"), ("library.pps", source)];
    for errors in [lower(&sources, None), compile(&sources, None).diagnostics] {
        assert_initializer_error(&errors, "library.pps", source, "First", "Other + 1");
        assert_initializer_error(&errors, "library.pps", source, "Second", "ABS(-2)");
        assert_eq!(2, errors.iter().filter(|error| error.initializer.is_some()).count(), "{errors:?}");
    }
}

#[test]
fn an_unused_module_initializer_is_rejected_before_dead_code_elimination() {
    let source = "MODULE Unused\nINTEGER Other = 7\nINTEGER OriginalName = 0 * Other\nENDMODULE\n";
    let sources = [("application.pps", "PRINT 42\n"), ("unused.pps", source)];
    for errors in [lower(&sources, None), compile(&sources, None).diagnostics] {
        assert_initializer_error(&errors, "unused.pps", source, "OriginalName", "0 * Other");
    }
}
