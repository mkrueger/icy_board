use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
    vec,
};

use crate::{
    ast::{
        AstNode, FunctionDeclarationAstNode, ParameterSpecifier, ProcedureDeclarationAstNode, VariableDeclarationStatement, VariableParameterSpecifier,
        VariableSpecifier,
    },
    compiler::{user_data::UserDataMemberRegistry, workspace::Workspace},
    executable::VariableType,
};

use super::{Encoding, ErrorReporter, Parser, UserTypeRegistry, parse_ast_with_predeclared_types, preparse_type_declarations};

fn parse_ast_node(input: &str, assert_eof: bool) -> AstNode {
    let reg = UserTypeRegistry::default();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut parser: Parser<'_> = Parser::new(PathBuf::from("."), errors, &reg, input, Encoding::Utf8, &Workspace::default());
    parser.next_token();
    let res: AstNode = parser.parse_ast_node().unwrap();
    if assert_eof {
        assert!(parser.get_cur_token().is_none(), "Expected EOF, but got {:?}", parser.get_cur_token());
    }
    res
}

fn check_ast_node(input: &str, check: &AstNode) {
    let node = parse_ast_node(input, true);
    if !node.is_similar(check) {
        println!("AstNode {node:?} is not similar to {check:?}");
        println!("was:\n{node}\nShould be:\n{check}");
        panic!();
    }
}

#[test]
fn a_file_can_use_a_type_declared_in_a_later_file() {
    let registry = UserTypeRegistry::default();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let workspace = Workspace::default();
    let main = "Point value\nvalue.X = 1\n";
    let types = "TYPE Point\n  INTEGER X\nENDTYPE\n";

    preparse_type_declarations(PathBuf::from("main.pps"), errors.clone(), main, &registry, Encoding::Utf8, &workspace);
    preparse_type_declarations(PathBuf::from("types.pps"), errors.clone(), types, &registry, Encoding::Utf8, &workspace);
    let ast = parse_ast_with_predeclared_types(PathBuf::from("main.pps"), errors.clone(), main, &registry, Encoding::Utf8, &workspace);

    assert!(!ast.nodes.is_empty());
    let messages: Vec<String> = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect();
    assert!(messages.is_empty(), "{messages:?}");
}

#[test]
fn a_file_can_use_an_enum_declared_in_a_later_file() {
    let registry = UserTypeRegistry::default();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let workspace = Workspace::default();
    let main = "Color favorite = Color.Green\n";
    let types = "ENUM Color\n  Red\n  Green\nENDENUM\n";

    preparse_type_declarations(PathBuf::from("main.pps"), errors.clone(), main, &registry, Encoding::Utf8, &workspace);
    preparse_type_declarations(PathBuf::from("types.pps"), errors.clone(), types, &registry, Encoding::Utf8, &workspace);
    let ast = parse_ast_with_predeclared_types(PathBuf::from("main.pps"), errors.clone(), main, &registry, Encoding::Utf8, &workspace);

    assert!(!ast.nodes.is_empty());
    let messages: Vec<String> = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect();
    assert!(messages.is_empty(), "{messages:?}");
}

#[test]
fn the_type_pass_does_not_report_what_the_real_parse_reports_again() {
    let registry = UserTypeRegistry::default();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let workspace = Workspace::default();
    let src = ";$ENDIF\nTYPE Point\n  INTEGER X\nENDTYPE\n";

    preparse_type_declarations(PathBuf::from("main.pps"), errors.clone(), src, &registry, Encoding::Utf8, &workspace);
    parse_ast_with_predeclared_types(PathBuf::from("main.pps"), errors.clone(), src, &registry, Encoding::Utf8, &workspace);

    let messages: Vec<String> = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect();
    assert_eq!(vec!["$ENDIF without $IF".to_string()], messages);
}

#[test]
fn a_type_declared_twice_is_reported_by_the_type_pass() {
    let registry = UserTypeRegistry::default();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let workspace = Workspace::default();
    let src = "TYPE Point\n  INTEGER X\nENDTYPE\nTYPE Point\n  INTEGER Y\nENDTYPE\n";

    preparse_type_declarations(PathBuf::from("main.pps"), errors.clone(), src, &registry, Encoding::Utf8, &workspace);

    let messages: Vec<String> = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect();
    assert_eq!(1, messages.len(), "{messages:?}");
}

#[test]
fn test_proc_declarations() {
    check_ast_node(
        "DECLARE PROCEDURE PROC001()",
        &ProcedureDeclarationAstNode::empty_node(unicase::Ascii::new("PROC001".to_string()), vec![]),
    );
    check_ast_node(
        "DECLARE PROCEDURE PROC001(BYTE B)",
        &ProcedureDeclarationAstNode::empty_node(
            unicase::Ascii::new("PROC001".to_string()),
            vec![ParameterSpecifier::Variable(VariableParameterSpecifier::empty(
                false,
                VariableType::Byte,
                Some(VariableSpecifier::empty(unicase::Ascii::new("B".to_string()), vec![])),
            ))],
        ),
    );
    check_ast_node(
        "DECLARE PROCEDURE PROC001(VAR BYTE B)",
        &ProcedureDeclarationAstNode::empty_node(
            unicase::Ascii::new("PROC001".to_string()),
            vec![ParameterSpecifier::Variable(VariableParameterSpecifier::empty(
                true,
                VariableType::Byte,
                Some(VariableSpecifier::empty(unicase::Ascii::new("B".to_string()), vec![])),
            ))],
        ),
    );
}

#[test]
fn test_proc_without_name() {
    check_ast_node(
        "DECLARE PROCEDURE PROC001(BYTE)",
        &ProcedureDeclarationAstNode::empty_node(
            unicase::Ascii::new("PROC001".to_string()),
            vec![ParameterSpecifier::Variable(VariableParameterSpecifier::empty(false, VariableType::Byte, None))],
        ),
    );
}

#[test]
fn test_variable_declariton() {
    check_ast_node(
        "STRING FOO[5]",
        &AstNode::TopLevelStatement(crate::ast::Statement::VariableDeclaration(VariableDeclarationStatement::empty(
            VariableType::String,
            vec![VariableSpecifier::empty(unicase::Ascii::new("FOO".to_string()), vec![5])],
        ))),
    );
}

#[test]
fn test_func_declarations() {
    check_ast_node(
        "DECLARE FUNCTION FUNC001() INTEGER",
        &FunctionDeclarationAstNode::empty_node(unicase::Ascii::new("FUNC001".to_string()), vec![], VariableType::Integer),
    );
    check_ast_node(
        "DECLARE FUNCTION FUNC001(BYTE B) INTEGER",
        &FunctionDeclarationAstNode::empty_node(
            unicase::Ascii::new("FUNC001".to_string()),
            vec![ParameterSpecifier::Variable(VariableParameterSpecifier::empty(
                false,
                VariableType::Byte,
                Some(VariableSpecifier::empty(unicase::Ascii::new("B".to_string()), vec![])),
            ))],
            VariableType::Integer,
        ),
    );
}

/*  use super::*;

#[test]
fn test_procedure() {
    let prg = get_ast("Procedure Proc() PRINT 5 EndProc");
    assert_eq!(1, prg.procedure_implementations.len());
}

#[test]
fn test_function() {
    let prg = get_ast("Function Func() BOOLEAN PRINT 5 EndFunc");
    assert_eq!(1, prg.function_implementations.len());
}*/

/// The id of a board object is stored in every PPE that names its type, so the
/// list may only ever grow at the end.
#[test]
fn board_object_type_ids_are_frozen() {
    let registry = UserTypeRegistry::icy_board_registry();
    let expected = [
        ("CONFERENCE", 30),
        ("AREA", 31),
        ("DIRECTORY", 32),
        ("DOOR", 33),
        ("CONTACT", 34),
        ("SURFACE", 35),
        ("EVENT", 36),
        ("AUDIO", 37),
        ("ERROR", 39),
        ("TERMINFO", 40),
        ("TERMINPUT", 41),
        ("TERMINAL", 42),
        ("GFX", 43),
        ("MARGINS", 44),
        ("PALETTE", 45),
        ("MACROS", 47),
        ("BOARD", 49),
        ("SESSION", 50),
        ("USER", 51),
        ("AREAS", 52),
        ("DIRECTORIES", 53),
        ("DOORS", 54),
        ("CONFERENCES", 55),
        ("NOTES", 56),
        ("CONTACTS", 57),
        ("MSG", 58),
        ("HTTP", 59),
        ("HTTPREQUEST", 60),
        ("HTTPRESPONSE", 61),
        ("USERS", 62),
        ("REGEX", 63),
        ("REGEXMATCH", 64),
        ("REGEXMATCHES", 65),
    ];

    for (name, id) in expected {
        assert_eq!(
            registry.get_type(&unicase::Ascii::new(name.to_string())),
            Some(VariableType::UserData(id)),
            "{name} moved"
        );
    }
    assert_eq!(
        registry.registered_types.len(),
        expected.len(),
        "a board object was added without freezing its id"
    );
}

#[test]
fn error_member_ids_are_frozen() {
    let registry = UserTypeRegistry::icy_board_registry();
    let error = &registry.types[&(super::ERROR_ID as u8)];
    for (name, id) in [("OK", 0), ("KIND", 1), ("CODE", 2), ("MESSAGE", 3), ("CHANNEL", 4)] {
        assert_eq!(error.get_member_id(&unicase::Ascii::new(name.to_string())), Some(id), "ERROR.{name} moved");
    }
}

/// A builtin enum takes a fixed id at the top of the space and a program's own enums
/// grow down from below them, so adding one in the middle would move every id after it.
#[test]
fn builtin_enum_ids_are_frozen() {
    let registry = UserTypeRegistry::icy_board_registry();
    let expected = [
        ("EventKind", 255),
        ("MouseAction", 254),
        ("MouseButton", 253),
        ("MouseMode", 252),
        ("MouseTracking", 251),
        ("GfxBackend", 250),
        ("ErrKind", 249),
        ("ErrCode", 248),
        ("EditorMode", 247),
        ("MsgField", 246),
        ("HttpMethod", 245),
        ("RegexOptions", 244),
        ("StringComparison", 243),
    ];

    for (name, id) in expected {
        let definition = registry
            .get_enum(&unicase::Ascii::new(name.to_string()))
            .unwrap_or_else(|| panic!("{name} is not registered"));
        assert_eq!(definition.id, id, "{name} moved");
    }
    assert_eq!(registry.enums().len(), expected.len(), "a builtin enum was added without freezing its id");
}

/// A program keeps naming its own enums, below the board's.
#[test]
fn a_program_enum_starts_below_the_builtin_ones() {
    let registry = UserTypeRegistry::icy_board_registry();
    let id = registry
        .declare_enum(unicase::Ascii::new("Mine".to_string()), vec![(unicase::Ascii::new("One".to_string()), 1)])
        .expect("a program enum should still fit");

    assert_eq!(id, 242);
}

fn parse_types(input: &str) -> (Vec<AstNode>, UserTypeRegistry, Arc<Mutex<ErrorReporter>>) {
    let reg = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut nodes = Vec::new();
    {
        let mut parser: Parser<'_> = Parser::new(PathBuf::from("."), errors.clone(), &reg, input, Encoding::Utf8, &Workspace::default());
        parser.next_token();
        parser.skip_eol();
        while parser.get_cur_token().is_some() {
            if let Some(node) = parser.parse_ast_node() {
                nodes.push(node);
            }
        }
    }
    (nodes, reg, errors)
}

#[test]
fn test_type_declaration() {
    let (nodes, reg, errors) = parse_types("TYPE Employee\n  STRING Name\n  INTEGER Age, Level\nENDTYPE\n");
    assert!(errors.lock().unwrap().errors.is_empty(), "{:?}", errors.lock().unwrap().errors.len());
    assert_eq!(1, nodes.len());
    let AstNode::TypeDeclaration(decl) = &nodes[0] else {
        panic!("expected a type declaration, got {:?}", nodes[0]);
    };
    assert_eq!(unicase::Ascii::new("Employee".to_string()), *decl.get_identifier());
    assert_eq!(3, decl.get_fields().len());

    let def = reg.get_user_type(&unicase::Ascii::new("EMPLOYEE".to_string())).unwrap();
    assert_eq!(super::FIRST_USER_TYPE_ID, def.id);
    assert_eq!(Some(0), def.field_index(&unicase::Ascii::new("name".to_string())));
    assert_eq!(Some(VariableType::String), def.field_type(0));
    assert_eq!(Some(VariableType::Integer), def.field_type(2));
    assert_eq!(Some(def.clone()), reg.get_user_type_from_id(def.id as u8));
}

#[test]
fn test_type_is_usable_as_a_variable_type() {
    let (nodes, reg, errors) = parse_types("TYPE Point\n  INTEGER X\n  INTEGER Y\nENDTYPE\nPoint P\n");
    assert!(errors.lock().unwrap().errors.is_empty());
    assert_eq!(2, nodes.len());
    let id = reg.get_user_type(&unicase::Ascii::new("point".to_string())).unwrap().id;
    let AstNode::TopLevelStatement(crate::ast::Statement::VariableDeclaration(decl)) = &nodes[1] else {
        panic!("expected a variable declaration, got {:?}", nodes[1]);
    };
    assert_eq!(VariableType::UserData(id as u8), decl.get_variable_type());
}

#[test]
fn test_two_types_get_distinct_ids() {
    let (nodes, reg, errors) = parse_types("TYPE A\n  INTEGER X\nENDTYPE\nTYPE B\n  INTEGER Y\nENDTYPE\n");
    assert!(errors.lock().unwrap().errors.is_empty());
    assert_eq!(2, nodes.len());
    assert_eq!(super::FIRST_USER_TYPE_ID, reg.get_user_type(&unicase::Ascii::new("a".to_string())).unwrap().id);
    assert_eq!(
        super::FIRST_USER_TYPE_ID + 1,
        reg.get_user_type(&unicase::Ascii::new("b".to_string())).unwrap().id
    );
}

#[test]
fn test_end_type_with_a_space_closes_the_declaration() {
    let (nodes, _, errors) = parse_types("TYPE Point\n  INTEGER X\nEND TYPE\nPoint value\n");
    assert!(errors.lock().unwrap().errors.is_empty());
    assert_eq!(2, nodes.len());
}

#[test]
fn test_type_and_field_names_are_case_insensitive() {
    for source in [
        "TYPE Point\n  INTEGER X\nENDTYPE\nTYPE pOiNt\n  INTEGER Y\nENDTYPE\n",
        "TYPE Point\n  INTEGER Value\n  STRING vAlUe\nENDTYPE\n",
    ] {
        let (_, _, errors) = parse_types(source);
        assert!(!errors.lock().unwrap().errors.is_empty(), "expected a duplicate-name error for:\n{source}");
    }
}

#[test]
fn test_a_record_field_can_name_an_earlier_record() {
    let (_, reg, errors) = parse_types("TYPE Inner\n  INTEGER X\nENDTYPE\nTYPE Outer\n  Inner Value\nENDTYPE\n");
    assert!(errors.lock().unwrap().errors.is_empty());
    let outer = reg.get_user_type(&unicase::Ascii::new("outer".to_string())).unwrap();
    assert_eq!(Some(VariableType::UserData(super::FIRST_USER_TYPE_ID as u8)), outer.field_type(0));
}

#[test]
fn test_a_record_field_cannot_name_a_later_record() {
    let (_, _, errors) = parse_types("TYPE Outer\n  Inner Value\nENDTYPE\nTYPE Inner\n  INTEGER X\nENDTYPE\n");
    assert!(!errors.lock().unwrap().errors.is_empty());
}

#[test]
fn test_record_field_arrays_keep_their_dimensions() {
    let (_, registry, errors) = parse_types("TYPE Rec\n  INTEGER Vector(10)\n  STRING Matrix(2, 3)\n  BOOLEAN Cube(1, 2, 3)\nENDTYPE\n");
    assert!(errors.lock().unwrap().errors.is_empty());
    let record = registry.get_user_type(&unicase::Ascii::new("Rec".to_string())).unwrap();
    assert_eq!(
        (1, 10, 0, 0),
        record
            .field(0)
            .map(|field| (field.dim, field.vector_size, field.matrix_size, field.cube_size))
            .unwrap()
    );
    assert_eq!(
        (2, 2, 3, 0),
        record
            .field(1)
            .map(|field| (field.dim, field.vector_size, field.matrix_size, field.cube_size))
            .unwrap()
    );
    assert_eq!(
        (3, 1, 2, 3),
        record
            .field(2)
            .map(|field| (field.dim, field.vector_size, field.matrix_size, field.cube_size))
            .unwrap()
    );
}

#[test]
fn test_a_record_field_dimension_must_fit_the_runtime_format() {
    let (_, _, errors) = parse_types("TYPE Rec\n  INTEGER Values(65536)\nENDTYPE\n");
    let errors: Vec<String> = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect();
    assert_eq!(vec!["Record field 'Values' has a dimension above 65535"], errors);
}

#[test]
fn test_a_record_field_initializer_is_rejected_explicitly() {
    let (_, _, errors) = parse_types("TYPE Rec\n  INTEGER Value = 7\nENDTYPE\n");
    let errors: Vec<String> = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect();
    assert!(
        errors.iter().any(|error| error == "Record field 'Value' cannot have an initializer"),
        "{errors:?}"
    );
}

fn many_types(count: usize) -> String {
    use std::fmt::Write as _;
    let mut result = String::new();
    for index in 0..count {
        let _ = writeln!(result, "TYPE Type{index:03}\n  INTEGER Value\nENDTYPE");
    }
    result
}

#[test]
fn test_all_reserved_custom_type_ids_are_available() {
    let source = many_types(super::MAX_USER_TYPES);
    let (nodes, registry, errors) = parse_types(&source);
    assert!(errors.lock().unwrap().errors.is_empty());
    assert_eq!(super::MAX_USER_TYPES, nodes.len());
    assert_eq!(
        u8::MAX as usize - super::BUILTIN_ENUM_COUNT,
        registry
            .get_user_type(&unicase::Ascii::new(format!("Type{:03}", super::MAX_USER_TYPES - 1)))
            .unwrap()
            .id,
    );
}

#[test]
fn test_one_more_than_the_reserved_custom_type_ids_is_rejected() {
    let source = many_types(super::MAX_USER_TYPES + 1);
    let (_, _, errors) = parse_types(&source);
    let errors: Vec<String> = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect();
    assert!(errors.iter().any(|error| error.contains("No room for another type")), "{errors:?}");
}

#[test]
fn test_type_errors() {
    for (source, expected) in [
        ("TYPE Point\n  INTEGER X\n  INTEGER X\nENDTYPE\n", "duplicate field"),
        ("TYPE Point\nENDTYPE\n", "no fields"),
        ("TYPE Point\n  Point Nested\nENDTYPE\n", "self reference"),
        ("TYPE Conference\n  INTEGER X\nENDTYPE\n", "clashes with a host type"),
        ("TYPE Point\n  INTEGER X\n", "missing ENDTYPE"),
    ] {
        let (_, _, errors) = parse_types(source);
        assert!(!errors.lock().unwrap().errors.is_empty(), "expected an error for {expected}: {source}");
    }
}

#[test]
fn test_type_is_not_a_keyword_before_400() {
    let reg = UserTypeRegistry::icy_board_registry();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut workspace = Workspace::default();
    workspace.package.runtime = Some(350);
    let mut parser: Parser<'_> = Parser::new(PathBuf::from("."), errors.clone(), &reg, "INTEGER type\n", Encoding::Utf8, &workspace);
    parser.next_token();
    parser.parse_ast_node().unwrap();
    assert!(errors.lock().unwrap().errors.is_empty());
}
