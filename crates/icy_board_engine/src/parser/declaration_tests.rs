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
    compiler::workspace::Workspace,
    executable::VariableType,
};

use super::{Encoding, ErrorReporter, Parser, UserTypeRegistry};

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
    assert_eq!(super::FIRST_USER_TYPE_ID + 1, reg.get_user_type(&unicase::Ascii::new("b".to_string())).unwrap().id);
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
