//! Exercise the compiler and the untransformed semantic path used by the LSP.
use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use icy_board_engine::{
    ast::Ast,
    compiler::{PPECompiler, lower_modules, workspace::Workspace},
    executable::Executable,
    icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState},
    parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast_with_predeclared_types, preparse_type_declarations},
    semantic::{SemanticInfo, SemanticVisitor},
    vm::{self, io::DiskIO},
};
use icy_net::{Connection, ConnectionType, channel::ChannelConnection};

const COLORS: &str = "MODULE Colors\nENUM Color\n First = 7\n Second = -3\n Zero = 0\n Alias = 7\nENDENUM\nCONST Color Preferred = Color.Second\nColor DefaultValue\nColor Initialized = Color.First\nColor FromConstant = Preferred\nFUNCTION Cast(INTEGER number) Color\n RETURN Color(number)\nENDFUNC\nENDMODULE\n";

#[test]
fn enum_bitwise_module_aliases_constants_initializers_and_lsp_agree() {
    let bits = "MODULE FlagsModule\nENUM Bits\n Both = 3\n One = 1\n Two = 2\n Zero = 0\nENDENUM\nCONST Bits Combined = Bits.One | Bits.Two\nBits Initial = Combined & Bits.Two\nBits DefaultValue\nENDMODULE\n";
    let values = "IMPORT FlagsModule AS F\nMODULE Values\nCONST F.Bits Alias = F.Combined | F.Bits.One\nF.Bits Initial = Alias & F.Bits.One\nCONST RegexOptions Options = RegexOptions.IgnoreCase | RegexOptions.MultiLine\nRegexOptions Configured = Options\nENDMODULE\n";
    succeeds(
        &[
            (
                "main.pps",
                "IMPORT FlagsModule AS F\nIMPORT Values AS V\nF.Bits a = V.Alias\na &= F.Bits.Two\nPRINT F.Combined, \"|\", F.Initial, \"|\", F.DefaultValue, \"|\", V.Initial, \"|\", a, \"|\", V.Options, \"|\", V.Configured\nIF ((V.Options & RegexOptions.MultiLine) = RegexOptions.MultiLine) PRINT \"|yes\"\n",
            ),
            ("bits.pps", bits),
            ("values.pps", values),
        ],
        "3|2|3|1|2|3|3|yes",
    );
}

#[test]
fn enum_bitwise_module_constants_and_intermediate_domains_are_checked() {
    let bits = "MODULE BitsModule\nENUM Bits\n One = 1\n Two = 2\nENDENUM\nCONST Bits Alias = Bits.One\nENDMODULE\n";
    for expression in ["F.Bits.One | F.Bits.Two", "F.Alias & F.Bits.Two", "(F.Alias | F.Bits.Two) & F.Bits.One"] {
        for declaration in [format!("CONST F.Bits Bad = {expression}"), format!("F.Bits Bad = {expression}")] {
            let values = format!("IMPORT BitsModule AS F\nMODULE Values\n{declaration}\nENDMODULE\n");
            rejects(
                &[("main.pps", "IMPORT Values AS V\nPRINT V.Bad\n"), ("bits.pps", bits), ("values.pps", &values)],
                "not a declared member",
            );
        }
    }
}

#[test]
fn enum_bitwise_compound_and_condition_nominality_match_lsp() {
    for statement in [
        "value += Color.First",
        "value -= Color.First",
        "value *= Color.First",
        "value |= 1",
        "PRINT value & 1",
        "IF ((value | Color.First) = 1) PRINT 1",
    ] {
        let main = format!("ENUM Color\n First = 1\n Both = 3\nENDENUM\nColor value\n{statement}\n");
        for language in [350, 400] {
            let sources = [("main.pps", main.as_str())];
            assert!(compile(&sources, language).is_err(), "compiler accepted {statement}");
            assert!(!lsp_diagnostics(&sources, language).is_empty(), "LSP accepted {statement}");
        }
    }
}

fn messages(errors: &Arc<Mutex<ErrorReporter>>) -> Vec<String> {
    errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect()
}

fn parse(sources: &[(&str, &str)], language: u16) -> (Workspace, UserTypeRegistry, Arc<Mutex<ErrorReporter>>, Vec<Ast>) {
    let mut workspace = Workspace::default();
    workspace.package.runtime = Some(400);
    workspace.set_default_language_version(Some(language));
    let registry = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    for (file, source) in sources {
        preparse_type_declarations(PathBuf::from(file), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
    }
    let asts = sources
        .iter()
        .map(|(file, source)| parse_ast_with_predeclared_types(PathBuf::from(file), errors.clone(), source, &registry, Encoding::Utf8, &workspace))
        .collect();
    assert!(messages(&errors).is_empty(), "language={language}: parse: {:?}", messages(&errors));
    (workspace, registry, errors, asts)
}

fn compile(sources: &[(&str, &str)], language: u16) -> Result<Executable, Vec<String>> {
    let (workspace, registry, errors, asts) = parse(sources, language);
    let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
    compiler.compile(&asts.iter().collect::<Vec<_>>());
    let diagnostics = messages(&errors);
    if !diagnostics.is_empty() {
        return Err(diagnostics);
    }
    let executable = compiler.create_executable().map_err(|error| vec![error.to_string()])?;
    icy_board_engine::executable::PPEScript::from_ppe_file(&executable).unwrap_or_else(|error| panic!("language={language}: malformed PPE: {error:?}"));
    Ok(executable)
}

fn lsp_diagnostics(sources: &[(&str, &str)], language: u16) -> Vec<String> {
    let (workspace, registry, errors, asts) = parse(sources, language);
    let mut visitor = SemanticVisitor::new(&workspace, errors.clone(), registry);
    visitor.set_modules(&asts.iter().collect::<Vec<_>>());
    let lowered = lower_modules(&asts.iter().collect::<Vec<_>>(), errors.clone(), &visitor.type_registry);
    visitor.prepare_legacy_call_signatures(&lowered.iter().collect::<Vec<_>>());
    for ast in lowered
        .iter()
        .filter(|ast| ast.module.is_some())
        .chain(lowered.iter().filter(|ast| ast.module.is_none()))
    {
        visitor.set_file_name(&ast.file_name);
        ast.visit(&mut visitor);
    }
    visitor.finish();
    messages(&errors)
}

fn run(executable: &Executable) -> Result<String, String> {
    // Execute serialized code too, so defaults and module constants must survive
    // the real metadata/value-table representation, not just semantic checking.
    let executable = Executable::from_buffer(&mut executable.to_buffer().unwrap(), false).unwrap();
    tokio::runtime::Builder::new_current_thread().enable_all().build().unwrap().block_on(async {
        let directory = tempfile::tempdir().unwrap();
        let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (mut peer, connection) = ChannelConnection::create_pair();
        let mut board = IcyBoard::new();
        board.root_path = directory.path().to_path_buf();
        let mut state = IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(board)), nodes, node, Box::new(connection)).await;
        let reader = async {
            let mut bytes = Vec::new();
            let mut buffer = [0; 1024];
            while let Ok(size) = peer.read(&mut buffer).await {
                if size == 0 {
                    break;
                }
                bytes.extend_from_slice(&buffer[..size]);
            }
            String::from_utf8(bytes).unwrap().replace("\r\n", "\n")
        };
        let execute = async {
            let mut io = DiskIO::new(directory.path().to_str().unwrap(), None);
            let result = vm::run(&PathBuf::from("modules.ppe"), &executable, &mut io, &mut state).await;
            drop(state);
            result.map_err(|error| error.to_string())
        };
        let (output, result) = tokio::join!(reader, execute);
        result.map(|_| output.clone()).map_err(|error| format!("{error}; output={output}"))
    })
}

fn succeeds(sources: &[(&str, &str)], expected: &str) {
    for language in [350, 400] {
        assert_eq!(Vec::<String>::new(), lsp_diagnostics(sources, language), "LSP language={language}");
        let executable = compile(sources, language).unwrap_or_else(|errors| panic!("compiler language={language}: {errors:?}"));
        let output = run(&executable).unwrap_or_else(|error| panic!("language={language}: {error}"));
        assert_eq!(expected, output, "language={language}");
    }
}

fn rejects(sources: &[(&str, &str)], diagnostic: &str) {
    for language in [350, 400] {
        let compiler = match compile(sources, language) {
            Ok(_) => panic!("compiler accepted invalid program, language={language}: {sources:?}"),
            Err(errors) => errors,
        };
        for (path, errors) in [("compiler", compiler), ("LSP", lsp_diagnostics(sources, language))] {
            assert!(
                errors.iter().any(|error| error.contains(diagnostic)),
                "{path} language={language}: expected {diagnostic:?}, got {errors:?}; sources={sources:?}"
            );
        }
    }
}

#[test]
fn qualified_casts_import_aliases_same_module_casts_and_scalar_defaults() {
    for import in ["IMPORT Colors AS Colors", "IMPORT Colors AS E"] {
        let name = if import.ends_with(" AS E") { "E" } else { "Colors" };
        let main = format!(
            "{import}\nINTEGER number = -3\n{name}.Color value = {name}.Color(number)\nPRINT value, \"|\", {name}.Color(7), \"|\", {name}.Color(0), \"|\", {name}.Cast(number), \"|\", {name}.DefaultValue, \"|\", {name}.Initialized, \"|\", {name}.FromConstant, \"|\", {name}.Preferred\n"
        );
        succeeds(&[("main.pps", &main), ("colors.pps", COLORS)], "-3|7|0|-3|7|7|-3|-3");
    }
}

#[test]
fn module_member_constants_initialize_other_modules_with_aliases() {
    succeeds(
        &[
            (
                "main.pps",
                "IMPORT Values AS V\nPRINT V.DefaultValue, \"|\", V.MemberValue, \"|\", V.ConstantValue, \"|\", V.LocalValue\n",
            ),
            ("colors.pps", COLORS),
            (
                "values.pps",
                "IMPORT Colors AS E\nMODULE Values\nCONST E.Color Favorite = E.Color.Second\nE.Color DefaultValue\nE.Color MemberValue = E.Color.Zero\nE.Color ConstantValue = E.Preferred\nE.Color LocalValue = Favorite\nENDMODULE\n",
            ),
        ],
        "7|0|-3|-3",
    );
}

#[test]
fn nonmodule_casts_accept_integer_constants_first_member_and_zero() {
    succeeds(
        &[(
            "main.pps",
            "ENUM Color\n First = 7\n Second = -3\n Zero = 0\nENDENUM\nCONST INTEGER FirstNumber = 7\nCONST INTEGER ZeroNumber = 0\nCONST Color Favorite = Color.Second\nColor defaultValue\nColor value = Color(FirstNumber)\nPRINT defaultValue, \"|\", value, \"|\", Color(ZeroNumber), \"|\", Color(7), \"|\", Color(0), \"|\", Favorite\n",
        )],
        "7|7|0|7|0|-3",
    );
}

#[test]
fn module_cast_initializers_keep_the_constant_only_policy() {
    for argument in ["7", "number"] {
        let source = format!("IMPORT Colors AS E\nMODULE Values\nINTEGER number = 7\nE.Color value = E.Color({argument})\nENDMODULE\n");
        rejects(
            &[
                ("main.pps", "IMPORT Values AS V\nPRINT V.value\n"),
                ("colors.pps", COLORS),
                ("values.pps", &source),
            ],
            "Module initializer for 'value' must be constant",
        );
    }
}

#[test]
fn same_domain_numbers_do_not_make_different_module_enums_assignable() {
    let other = "MODULE Other\nENUM Color\n First = 7\nENDENUM\nCONST Color Favorite = Color.First\nENDMODULE\n";
    for statement in [
        "E.Color value = O.Color.First\nPRINT value",
        "E.Color value = O.Favorite\nPRINT value",
        "E.Color value = O.Color(7)\nPRINT value",
        "CONST E.Color value = O.Color.First\nPRINT value",
        "PRINT E.Color.First = O.Color.First",
    ] {
        let main = format!("IMPORT Colors AS E\nIMPORT Other AS O\n{statement}\n");
        let diagnostic = if statement.contains("PRINT E.Color.First =") { "compare" } else { "assign" };
        rejects(&[("main.pps", &main), ("colors.pps", COLORS), ("other.pps", other)], diagnostic);
    }
}

#[test]
fn checked_casts_reject_wrong_enum_arguments_and_invalid_constant_values() {
    for (expression, diagnostic) in [
        ("E.Color(E.Color.First)", "INTEGER"),
        ("E.Color(8)", "not a declared member"),
        ("E.Color(1 - 1 + 8)", "not a declared member"),
    ] {
        let main = format!("IMPORT Colors AS E\nPRINT {expression}\n");
        rejects(&[("main.pps", &main), ("colors.pps", COLORS)], diagnostic);
    }
}

#[test]
fn qualified_dynamic_cast_checks_domain_at_runtime() {
    for language in [350, 400] {
        let sources = [
            ("main.pps", "IMPORT Colors AS E\nINTEGER number = 8\nPRINT E.Color(number)\n"),
            ("colors.pps", COLORS),
        ];
        assert!(lsp_diagnostics(&sources, language).is_empty());
        let executable = compile(&sources, language).unwrap();
        let error = run(&executable).unwrap_err();
        assert!(error.contains("not a member of closed enum"), "{error}");
    }
}

#[test]
fn numeric_for_rejects_enum_counters_in_compiler_and_lsp() {
    rejects(
        &[
            (
                "main.pps",
                "IMPORT Colors AS E\nE.Color value\nFOR value = E.Color.First TO E.Color.First\n PRINT value\nNEXT\n",
            ),
            ("colors.pps", COLORS),
        ],
        "numeric FOR counters",
    );
}

#[test]
fn numeric_for_rejects_enum_bounds_and_steps_in_compiler_and_lsp() {
    for range in ["E.Color.First TO 7", "1 TO E.Color.First", "1 TO 7 STEP E.Color.First"] {
        let main = format!("IMPORT Colors AS E\nINTEGER value\nFOR value = {range}\n PRINT value\nNEXT\n");
        // Compiler lowering can report assignment/comparison errors as well as
        // arithmetic errors; both paths must reject the enum operand.
        for language in [350, 400] {
            let sources = [("main.pps", main.as_str()), ("colors.pps", COLORS)];
            assert!(compile(&sources, language).is_err(), "compiler accepted {range}");
            assert!(!lsp_diagnostics(&sources, language).is_empty(), "LSP accepted {range}, language={language}");
        }
    }
}

#[test]
fn enum_declaration_expression_cannot_smuggle_another_enum_member() {
    for language in [350, 400] {
        let mut workspace = Workspace::default();
        workspace.set_default_language_version(Some(language));
        let registry = UserTypeRegistry::icy_board_registry();
        let errors = Arc::new(Mutex::new(ErrorReporter::default()));
        let source = "ENUM A\n X = 7\nENDENUM\nENUM B\n Y = A.X + 1\nENDENUM\n";
        icy_board_engine::parser::parse_ast(PathBuf::from("main.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
        assert!(
            messages(&errors).iter().any(|error| error.contains("enum member needs an integer value")),
            "{:?}",
            messages(&errors)
        );
    }
}

#[test]
fn numeric_for_with_explicit_integer_conversion_remains_valid() {
    succeeds(
        &[
            (
                "main.pps",
                "IMPORT Colors AS E\nINTEGER value\nFOR value = TOINTEGER(E.Color.First) TO TOINTEGER(E.Color.First)\n PRINT value\nNEXT\n",
            ),
            ("colors.pps", COLORS),
        ],
        "7",
    );
}

#[test]
fn private_enum_cast_is_not_accessible_through_an_import_alias() {
    rejects(
        &[
            ("main.pps", "IMPORT Hidden AS H\nPRINT H.Secret(7)\n"),
            ("hidden.pps", "MODULE Hidden\nPRIVATE\nENUM Secret\n First = 7\nENDENUM\nENDMODULE\n"),
        ],
        "private to module Hidden",
    );
}

#[test]
fn shared_lsp_lowering_classifies_qualified_and_same_module_calls_as_enum_casts() {
    for language in [350, 400] {
        let sources = [
            ("main.pps", "IMPORT Colors AS E\nPRINT E.Color(7), E.Color(0), E.Cast(-3)\n"),
            ("colors.pps", COLORS),
        ];
        let (workspace, registry, errors, asts) = parse(&sources, language);
        let mut visitor = SemanticVisitor::new(&workspace, errors.clone(), registry);
        visitor.set_modules(&asts.iter().collect::<Vec<_>>());
        let lowered = lower_modules(&asts.iter().collect::<Vec<_>>(), errors.clone(), &visitor.type_registry);
        visitor.prepare_legacy_call_signatures(&lowered.iter().collect::<Vec<_>>());
        for ast in lowered
            .iter()
            .filter(|ast| ast.module.is_some())
            .chain(lowered.iter().filter(|ast| ast.module.is_none()))
        {
            visitor.set_file_name(&ast.file_name);
            ast.visit(&mut visitor);
        }
        visitor.finish();
        assert!(messages(&errors).is_empty(), "{:?}", messages(&errors));
        let name = UserTypeRegistry::module_type_name(&unicase::Ascii::new("Colors".into()), &unicase::Ascii::new("Color".into()));
        let enum_id = visitor.type_registry.get_enum(&name).unwrap().id;
        assert_eq!(
            3,
            visitor
                .function_type_lookup
                .values()
                .filter(|info| matches!(info, SemanticInfo::EnumCast(id) if *id == enum_id))
                .count()
        );
    }
}

#[test]
fn nonmodule_constant_casts_reject_zero_when_it_is_not_a_member() {
    for argument in ["0", "ZeroNumber", "FirstNumber + 1"] {
        let main = format!(
            "ENUM Color\n First = 7\nENDENUM\nCONST INTEGER ZeroNumber = 0\nCONST INTEGER FirstNumber = 7\nColor value = Color({argument})\nPRINT value\n"
        );
        rejects(&[("main.pps", &main)], "not a declared member");
    }
}

#[test]
fn wrong_enum_module_initializers_and_constants_are_rejected_nominally() {
    for declaration in ["E.Color value = Other.First", "CONST E.Color value = Other.First"] {
        let module = format!("IMPORT Colors AS E\nMODULE Values\nENUM Other\n First = 7\nENDENUM\n{declaration}\nENDMODULE\n");
        rejects(
            &[
                ("main.pps", "IMPORT Values AS V\nPRINT V.value\n"),
                ("colors.pps", COLORS),
                ("values.pps", &module),
            ],
            "assign",
        );
    }
}

#[test]
fn same_module_cast_initializer_remains_forbidden_even_with_a_constant_argument() {
    let module = "MODULE Values\nENUM Color\n First = 7\nENDENUM\nCONST INTEGER Number = 7\nColor value = Color(Number)\nENDMODULE\n";
    rejects(
        &[("main.pps", "IMPORT Values AS V\nPRINT V.value\n"), ("values.pps", module)],
        "Module initializer for 'value' must be constant",
    );
}
