use std::path::PathBuf;

use crate::{
    ast::{
        AstNode, FunctionDeclarationAstNode, ParameterSpecifier, ProcedureDeclarationAstNode,
        VariableSpecifier,
    },
    executable::{VariableType, LAST_PPLC},
};

use super::{Encoding, Parser};

fn parse_ast_node(input: &str, assert_eof: bool) -> AstNode {
    let mut parser = Parser::new(PathBuf::from("."), input, Encoding::Utf8, LAST_PPLC);
    parser.next_token();
    let res = parser.parse_ast_node().unwrap();
    if assert_eof {
        assert!(parser.get_cur_token().is_none());
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
        &ProcedureDeclarationAstNode::empty_node(
            unicase::Ascii::new("PROC001".to_string()),
            vec![],
        ),
    );
    check_ast_node(
        "DECLARE PROCEDURE PROC001(BYTE B)",
        &ProcedureDeclarationAstNode::empty_node(
            unicase::Ascii::new("PROC001".to_string()),
            vec![ParameterSpecifier::empty(
                false,
                VariableType::Byte,
                VariableSpecifier::empty(unicase::Ascii::new("B".to_string()), vec![]),
            )],
        ),
    );
    check_ast_node(
        "DECLARE PROCEDURE PROC001(VAR BYTE B)",
        &ProcedureDeclarationAstNode::empty_node(
            unicase::Ascii::new("PROC001".to_string()),
            vec![ParameterSpecifier::empty(
                true,
                VariableType::Byte,
                VariableSpecifier::empty(unicase::Ascii::new("B".to_string()), vec![]),
            )],
        ),
    );
}

#[test]
fn test_func_declarations() {
    check_ast_node(
        "DECLARE FUNCTION FUNC001() INTEGER",
        &FunctionDeclarationAstNode::empty_node(
            unicase::Ascii::new("FUNC001".to_string()),
            vec![],
            VariableType::Integer,
        ),
    );
    check_ast_node(
        "DECLARE FUNCTION FUNC001(BYTE B) INTEGER",
        &FunctionDeclarationAstNode::empty_node(
            unicase::Ascii::new("FUNC001".to_string()),
            vec![ParameterSpecifier::empty(
                false,
                VariableType::Byte,
                VariableSpecifier::empty(unicase::Ascii::new("B".to_string()), vec![]),
            )],
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
