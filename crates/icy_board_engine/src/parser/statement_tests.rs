use std::{
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    ast::{
        Ast, AstNode, BlockStatement, BreakStatement, CaseBlock, CaseSpecifier, CommentAstNode, Constant, ConstantExpression, ContinueStatement, ElseBlock,
        ElseIfBlock, ForStatement, GosubStatement, GotoStatement, IdentifierExpression, IfStatement, IfThenStatement, LabelStatement, LetStatement,
        LoopStatement, ParensExpression, PredefinedCallStatement, RepeatUntilStatement, ReturnStatement, SelectStatement, Statement, UnaryExpression, UnaryOp,
        VariableDeclarationStatement, VariableSpecifier, WhileDoStatement, WhileStatement, constant::NumberFormat,
    },
    compiler::workspace::{CompilerData, Workspace},
    executable::{OpCode, VariableType},
};

use super::{
    Encoding, ErrorReporter, Parser, ParserErrorType, UserTypeRegistry,
    lexer::{Spanned, Token},
    parse_ast,
};

fn parse_statement(input: &str, assert_eof: bool) -> Statement {
    let reg = UserTypeRegistry::default();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut parser = Parser::new(PathBuf::from("."), errors, &reg, input, Encoding::Utf8, &Workspace::default());
    parser.next_token();
    match parser.parse_statement() {
        Some(stmt) => {
            if assert_eof {
                assert!(parser.get_cur_token().is_none());
            }
            stmt
        }
        None => {
            for error in &parser.error_reporter.lock().unwrap().errors {
                println!("{}", error.error);
            }
            panic!("Error");
        }
    }
}

fn check_statement(input: &str, check: &Statement) {
    let stmt = parse_statement(input, true);

    if !stmt.is_similar(check) {
        println!("Statement {stmt:?} is not similar to {check:?}");
        println!("was:\n{stmt}\nShould be:\n{check}");
        panic!();
    }
}

fn check_statement_without_eol(input: &str, check: &Statement) {
    let stmt = parse_statement(input, false);

    if !stmt.is_similar(check) {
        println!("Statement {stmt:?} is not similar to {check:?}");
        println!("was:\n{stmt}\nShould be:\n{check}");
        panic!();
    }
}

#[test]
fn test_parse_comment_statement() {
    check_statement(";FOO", &CommentAstNode::create_empty_statement("FOO"));
}

#[test]
fn test_parse_return_statement() {
    check_statement("RETURN", &ReturnStatement::create_empty_statement(None));
}

#[test]
fn test_label_statement() {
    check_statement(":MyLabel", &LabelStatement::create_empty_statement(unicase::Ascii::new("MyLabel".to_string())));

    check_statement(":END", &LabelStatement::create_empty_statement(unicase::Ascii::new("END".to_string())));
}

#[test]
fn test_goto_statement() {
    check_statement("Goto Foo", &GotoStatement::create_empty_statement(unicase::Ascii::new("Foo".to_string())));
    check_statement("goto end", &GotoStatement::create_empty_statement(unicase::Ascii::new("end".to_string())));
}

#[test]
fn test_gosub_statement() {
    check_statement("GOSUB Foo", &GosubStatement::create_empty_statement(unicase::Ascii::new("Foo".to_string())));
    check_statement("GOSUB end", &GosubStatement::create_empty_statement(unicase::Ascii::new("end".to_string())));
}

#[test]
fn test_parse_break_statement() {
    check_statement("BREAK", &BreakStatement::create_empty_statement());
    // Alias
    // check_statement("QUIT", &BreakStatement::create_empty_statement());
}

#[test]
fn test_parse_continue_statement() {
    check_statement("CONTINUE", &ContinueStatement::create_empty_statement());

    // Alias
    // check_statement("LOOP", &ContinueStatement::create_empty_statement());
}

#[test]
fn test_if_statement() {
    check_statement(
        "if (A) BREAK",
        &IfStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            BreakStatement::create_empty_statement(),
        ),
    );
}

#[test]
fn test_if_then_statement() {
    check_statement(
        r"if (A) THEN
        END IF",
        &IfThenStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![],
            vec![],
            None,
        ),
    );

    check_statement(
        r"if (A) THEN
        
        ENDIF",
        &IfThenStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![],
            vec![],
            None,
        ),
    );

    check_statement(
        r"if (A) THEN
        BREAK
        RETURN
        END IF",
        &IfThenStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![BreakStatement::create_empty_statement(), ReturnStatement::create_empty_statement(None)],
            vec![],
            None,
        ),
    );

    check_statement(
        r"if (A) THEN
        CONTINUE
        :FOO
        ENDIF",
        &IfThenStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![
                ContinueStatement::create_empty_statement(),
                LabelStatement::create_empty_statement(unicase::Ascii::new("FOO".to_string())),
            ],
            vec![],
            None,
        ),
    );
}

#[test]
fn test_if_then_statement2() {
    check_statement(
        r"if A THEN
        END IF",
        &IfThenStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![],
            vec![],
            None,
        ),
    );
}
#[test]
fn test_if_do_statement() {
    check_statement(
        r"if (A) DO
        END IF",
        &IfThenStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![],
            vec![],
            None,
        ),
    );
    check_statement(
        r"if (A) DO ;COMMENT
        END IF",
        &IfThenStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![],
            vec![],
            None,
        ),
    );
}

#[test]
fn test_if_then_else_statement() {
    check_statement(
        r"if (A) THEN
        ELSE
            BREAK
        END IF",
        &IfThenStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![],
            vec![],
            Some(ElseBlock::empty(vec![BreakStatement::create_empty_statement()])),
        ),
    );
}

#[test]
fn test_if_then_ifelse_statement() {
    check_statement(
        r"if (A) THEN
        ELSEIF (B) THEN
            CONTINUE
        END IF",
        &IfThenStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![],
            vec![ElseIfBlock::empty(
                IdentifierExpression::create_empty_expression(unicase::Ascii::new("B".to_string())),
                vec![ContinueStatement::create_empty_statement()],
            )],
            None,
        ),
    );

    check_statement(
        r"if (A) THEN
        ELSE IF (B) THEN
            CONTINUE
        END IF",
        &IfThenStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![],
            vec![ElseIfBlock::empty(
                IdentifierExpression::create_empty_expression(unicase::Ascii::new("B".to_string())),
                vec![ContinueStatement::create_empty_statement()],
            )],
            None,
        ),
    );
    check_statement(
        r"if (A) THEN
        ELSE IF (B) CONTINUE
        END IF",
        &IfThenStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![],
            vec![ElseIfBlock::empty(
                IdentifierExpression::create_empty_expression(unicase::Ascii::new("B".to_string())),
                vec![ContinueStatement::create_empty_statement()],
            )],
            None,
        ),
    );
}

#[test]
fn test_if_then_ifelse_withoutthen_statement() {
    check_statement(
        r"if (A) THEN
        ELSEIF (B)
            CONTINUE
        END IF",
        &IfThenStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![],
            vec![ElseIfBlock::empty(
                IdentifierExpression::create_empty_expression(unicase::Ascii::new("B".to_string())),
                vec![ContinueStatement::create_empty_statement()],
            )],
            None,
        ),
    );

    check_statement(
        r"if (A) THEN
        ELSE IF (B)
            CONTINUE
        END IF",
        &IfThenStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![],
            vec![ElseIfBlock::empty(
                IdentifierExpression::create_empty_expression(unicase::Ascii::new("B".to_string())),
                vec![ContinueStatement::create_empty_statement()],
            )],
            None,
        ),
    );
}

#[test]
fn test_if_then_ifelse_else_statement() {
    check_statement(
        r"if (A) THEN
        ELSEIF (B) THEN
            CONTINUE
        ELSE
            BREAK
        END IF",
        &IfThenStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![],
            vec![ElseIfBlock::empty(
                IdentifierExpression::create_empty_expression(unicase::Ascii::new("B".to_string())),
                vec![ContinueStatement::create_empty_statement()],
            )],
            Some(ElseBlock::empty(vec![BreakStatement::create_empty_statement()])),
        ),
    );
}

#[test]
fn test_while_statement() {
    check_statement(
        "while (A) BREAK",
        &WhileStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            BreakStatement::create_empty_statement(),
        ),
    );
}

#[test]
fn test_while_do_statement() {
    check_statement(
        r"WHILE (A) DO
        END WHILE",
        &WhileDoStatement::create_empty_statement(IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())), vec![]),
    );

    check_statement(
        r"WHILE (A) DO
        
        ENDWHILE",
        &WhileDoStatement::create_empty_statement(IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())), vec![]),
    );

    check_statement(
        r"WHILE (A) DO
        BREAK
        RETURN
        END WHILE",
        &WhileDoStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![BreakStatement::create_empty_statement(), ReturnStatement::create_empty_statement(None)],
        ),
    );

    check_statement(
        r"WHILE (A) DO
        CONTINUE
        :FOO
        ENDWHILE",
        &WhileDoStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![
                ContinueStatement::create_empty_statement(),
                LabelStatement::create_empty_statement(unicase::Ascii::new("FOO".to_string())),
            ],
        ),
    );
}

#[test]
fn test_empty_select_statement() {
    check_statement(
        r"SELECT CASE A
ENDSELECT",
        &SelectStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![],
            vec![],
        ),
    );
    check_statement(
        r"SELECT CASE A
END SELECT",
        &SelectStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![],
            vec![],
        ),
    );
}

#[test]
fn test_select_statement() {
    check_statement(
        r"SELECT CASE A
CASE 1
BREAK
CASE 1, 2, 3
BREAK
ENDSELECT",
        &SelectStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![
                CaseBlock::empty(
                    vec![CaseSpecifier::Expression(Box::new(ConstantExpression::create_empty_expression(
                        Constant::Integer(1, NumberFormat::Default),
                    )))],
                    vec![BreakStatement::create_empty_statement()],
                ),
                CaseBlock::empty(
                    vec![
                        CaseSpecifier::Expression(Box::new(ConstantExpression::create_empty_expression(Constant::Integer(
                            1,
                            NumberFormat::Default,
                        )))),
                        CaseSpecifier::Expression(Box::new(ConstantExpression::create_empty_expression(Constant::Integer(
                            2,
                            NumberFormat::Default,
                        )))),
                        CaseSpecifier::Expression(Box::new(ConstantExpression::create_empty_expression(Constant::Integer(
                            3,
                            NumberFormat::Default,
                        )))),
                    ],
                    vec![BreakStatement::create_empty_statement()],
                ),
                /*        CaseBlock::empty(
                vec![CaseSpecifier::FromTo(Box::new(ConstantExpression::create_empty_expression(Constant::Integer(1))), Box::new(ConstantExpression::create_empty_expression(Constant::Integer(3))))],
                vec![BreakStatement::create_empty_statement()]),*/
            ],
            vec![],
        ),
    );
}

#[test]
fn test_select_multiple_case_specifiers_statement() {
    check_statement(
        r"SELECT CASE A
CASE 1, 2, 3
BREAK
ENDSELECT",
        &SelectStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![
                CaseBlock::empty(
                    vec![
                        CaseSpecifier::Expression(Box::new(ConstantExpression::create_empty_expression(Constant::Integer(
                            1,
                            NumberFormat::Default,
                        )))),
                        CaseSpecifier::Expression(Box::new(ConstantExpression::create_empty_expression(Constant::Integer(
                            2,
                            NumberFormat::Default,
                        )))),
                        CaseSpecifier::Expression(Box::new(ConstantExpression::create_empty_expression(Constant::Integer(
                            3,
                            NumberFormat::Default,
                        )))),
                    ],
                    vec![BreakStatement::create_empty_statement()],
                ),
                /*        CaseBlock::empty(
                vec![CaseSpecifier::FromTo(Box::new(ConstantExpression::create_empty_expression(Constant::Integer(1))), Box::new(ConstantExpression::create_empty_expression(Constant::Integer(3))))],
                vec![BreakStatement::create_empty_statement()]),*/
            ],
            vec![],
        ),
    );
}

#[test]
fn test_select_from_to_case_specifiers_statement() {
    check_statement(
        r"SELECT CASE A
CASE 1..3
BREAK
ENDSELECT",
        &SelectStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![CaseBlock::empty(
                vec![CaseSpecifier::FromTo(
                    Box::new(ConstantExpression::create_empty_expression(Constant::Integer(1, NumberFormat::Default))),
                    Box::new(ConstantExpression::create_empty_expression(Constant::Integer(3, NumberFormat::Default))),
                )],
                vec![BreakStatement::create_empty_statement()],
            )],
            vec![],
        ),
    );
}

#[test]
fn test_case_default_statement() {
    check_statement(
        r"SELECT CASE A
DEFAULT
BREAK
ENDSELECT",
        &SelectStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![],
            vec![BreakStatement::create_empty_statement()],
        ),
    );

    check_statement(
        r"SELECT CASE A
CASE ELSE
BREAK
ENDSELECT",
        &SelectStatement::create_empty_statement(
            IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())),
            vec![],
            vec![BreakStatement::create_empty_statement()],
        ),
    );
}

#[test]
fn test_predefined_call() {
    check_statement(
        "PRINTLN",
        &PredefinedCallStatement::create_empty_statement(OpCode::PRINTLN.get_definition(), Vec::new()),
    );
    check_statement_without_eol(
        "PRINTLN ;COMMENT",
        &PredefinedCallStatement::create_empty_statement(OpCode::PRINTLN.get_definition(), Vec::new()),
    );

    check_statement(
        "PRINTLN (1)",
        &PredefinedCallStatement::create_empty_statement(
            OpCode::PRINTLN.get_definition(),
            vec![ParensExpression::create_empty_expression(ConstantExpression::create_empty_expression(
                Constant::Integer(1, NumberFormat::Default),
            ))],
        ),
    );
}

#[test]
fn test_for_statement() {
    check_statement(
        r"FOR I = 0 TO 5 
NEXT",
        &ForStatement::create_empty_statement(
            unicase::Ascii::new("I".to_string()),
            ConstantExpression::create_empty_expression(Constant::Integer(0, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
            None,
            vec![],
        ),
    );

    check_statement(
        r"FOR I = 0 TO 5 
NEXT I",
        &ForStatement::create_empty_statement(
            unicase::Ascii::new("I".to_string()),
            ConstantExpression::create_empty_expression(Constant::Integer(0, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
            None,
            vec![],
        ),
    );
}

#[test]
fn test_for_statement_alt_next() {
    check_statement(
        r"FOR I = 0 TO 5 
ENDFOR",
        &ForStatement::create_empty_statement(
            unicase::Ascii::new("I".to_string()),
            ConstantExpression::create_empty_expression(Constant::Integer(0, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
            None,
            vec![],
        ),
    );

    check_statement(
        r"FOR I = 0 TO 5 
END FOR",
        &ForStatement::create_empty_statement(
            unicase::Ascii::new("I".to_string()),
            ConstantExpression::create_empty_expression(Constant::Integer(0, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
            None,
            vec![],
        ),
    );
}

#[test]
fn test_for_step_statement() {
    check_statement(
        r"FOR I = 0 TO 5 STEP 3
NEXT",
        &ForStatement::create_empty_statement(
            unicase::Ascii::new("I".to_string()),
            ConstantExpression::create_empty_expression(Constant::Integer(0, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
            Some(Box::new(ConstantExpression::create_empty_expression(Constant::Integer(
                3,
                NumberFormat::Default,
            )))),
            vec![],
        ),
    );

    check_statement(
        r"FOR I = 5 TO 0 STEP -4
NEXT I",
        &ForStatement::create_empty_statement(
            unicase::Ascii::new("I".to_string()),
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
            ConstantExpression::create_empty_expression(Constant::Integer(0, NumberFormat::Default)),
            Some(Box::new(UnaryExpression::create_empty_expression(
                UnaryOp::Minus,
                ConstantExpression::create_empty_expression(Constant::Integer(4, NumberFormat::Default)),
            ))),
            vec![],
        ),
    );
}

#[test]
fn test_check_begin() {
    check_statement("BEGIN\nEND", &BlockStatement::create_empty_statement(Vec::new()));
    check_statement(
        "BEGIN\n  PRINT 1\nEND",
        &BlockStatement::create_empty_statement(vec![PredefinedCallStatement::create_empty_statement(
            OpCode::PRINT.get_definition(),
            vec![ConstantExpression::create_empty_expression(Constant::Integer(1, NumberFormat::Default))],
        )]),
    );
}

fn parse_program(input: &str, language_version: u16) -> (Ast, Vec<String>) {
    let registry = UserTypeRegistry::default();
    let errors = Arc::new(Mutex::new(ErrorReporter::default()));
    let mut workspace = Workspace::default();
    workspace.compiler = Some(CompilerData {
        language_version: Some(language_version),
        defines: None,
    });
    let ast = parse_ast(PathBuf::from("."), errors.clone(), input, &registry, Encoding::Utf8, &workspace);
    let messages = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect();
    (ast, messages)
}

#[test]
fn a_block_without_an_end_is_an_error() {
    let (_, messages) = parse_program(";$USEFUNCS\nBEGIN\n  PRINT 1\n", 400);
    assert_eq!(vec![ParserErrorType::BlockEndExpected.to_string()], messages);
}

#[test]
fn statements_outside_of_a_block_are_an_error() {
    let (_, messages) = parse_program("BEGIN\n  PRINT 1\nEND\nPRINT 2\n", 400);
    assert_eq!(vec![ParserErrorType::NoStatementsAllowedOutsideBlock.to_string()], messages);
}

#[test]
fn a_block_collects_the_main_program() {
    let (ast, messages) = parse_program("BEGIN\n  PRINT 1\n  PRINT 2\nEND\n", 400);
    assert!(messages.is_empty(), "{messages:?}");
    let [AstNode::Main(main)] = ast.nodes.as_slice() else {
        panic!("expected a single main block, got {:?}", ast.nodes);
    };
    assert_eq!(2, main.get_statements().len());
    assert!(main.get_begin_token().is_some());
    assert!(main.get_end_token().is_some());
}

#[test]
fn an_older_language_keeps_begin_as_a_pseudo_label() {
    let (ast, messages) = parse_program(";$USEFUNCS\nBEGIN\n  PRINT 1\n", 350);
    assert!(messages.is_empty(), "{messages:?}");
    assert!(
        ast.nodes.iter().any(|node| matches!(node, AstNode::Main(_))),
        "expected a main block, got {:?}",
        ast.nodes
    );
}

#[test]
fn end_is_only_a_block_end_from_400() {
    let (_, messages) = parse_program("PRINT 1\nEND\n", 400);
    assert_eq!(vec![ParserErrorType::EndIsNotAStatement.to_string()], messages);

    let (_, messages) = parse_program("BEGIN\n  IF (1) THEN\n    END\n  ENDIF\nEND\n", 400);
    assert_eq!(vec![ParserErrorType::EndIsNotAStatement.to_string()], messages);

    let (_, messages) = parse_program("BEGIN\n  IF (1) THEN\n    STOP\n  ENDIF\nEND\n", 400);
    assert!(messages.is_empty(), "{messages:?}");
}

#[test]
fn an_older_language_still_ends_a_program_with_end() {
    let (_, messages) = parse_program("PRINT 1\nEND\n", 350);
    assert!(messages.is_empty(), "{messages:?}");
}

#[test]
fn a_type_is_only_known_from_the_version_that_named_it() {
    // The PPL release notes put the widths and the big string in 2.00 and the
    // DBase date in 3.00.
    for (source, since) in [("BIGSTR s\n", 200), ("DDATE d\n", 300), ("MSGAREAID a\n", 400)] {
        let (_, messages) = parse_program(source, since);
        assert!(messages.is_empty(), "{source:?} at {since}: {messages:?}");

        let (_, messages) = parse_program(source, since - 100);
        assert!(!messages.is_empty(), "{source:?} should not be a type before {since}");
    }
}

#[test]
fn a_board_object_is_only_a_type_from_400() {
    let parse = |language_version: u16| {
        let registry = UserTypeRegistry::icy_board_registry();
        let errors = Arc::new(Mutex::new(ErrorReporter::default()));
        let mut workspace = Workspace::default();
        workspace.compiler = Some(CompilerData {
            language_version: Some(language_version),
            defines: None,
        });
        parse_ast(PathBuf::from("."), errors.clone(), "Conference c\n", &registry, Encoding::Utf8, &workspace);
        let messages: Vec<String> = errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect();
        messages
    };

    assert!(parse(400).is_empty(), "{:?}", parse(400));
    // 350 has enums, so this has to be the objects being unknown rather than
    // declared types as a whole.
    assert!(!parse(350).is_empty(), "Conference should not be a type in 350");
}

#[test]
fn exit_ends_a_program_from_400() {
    let (ast, messages) = parse_program("BEGIN\n  IF (1) THEN\n    EXIT\n  ENDIF\n  PRINT 1\nEND\n", 400);
    assert!(messages.is_empty(), "{messages:?}");

    let mut visitor = OpCodeCollector::default();
    ast.visit(&mut visitor);
    assert!(
        visitor.opcodes.contains(&OpCode::END),
        "EXIT did not become the END statement: {:?}",
        visitor.opcodes
    );
}

#[test]
fn exit_is_not_reserved_before_400() {
    let (_, messages) = parse_program("INTEGER exit\nexit = 1\n", 350);
    assert!(messages.is_empty(), "{messages:?}");
}

#[derive(Default)]
struct OpCodeCollector {
    opcodes: Vec<OpCode>,
}

impl crate::ast::AstVisitor<()> for OpCodeCollector {
    fn visit_predefined_call_statement(&mut self, call: &PredefinedCallStatement) {
        self.opcodes.push(call.get_func().opcode);
    }
}

#[test]
fn check_let_statement() {
    check_statement(
        "LET A = 5",
        &LetStatement::create_empty_statement(
            unicase::Ascii::new("A".to_string()),
            Token::Eq,
            vec![],
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );

    check_statement(
        "LET A(1, 2, 3) = 5",
        &LetStatement::create_empty_statement(
            unicase::Ascii::new("A".to_string()),
            Token::Eq,
            vec![
                ConstantExpression::create_empty_expression(Constant::Integer(1, NumberFormat::Default)),
                ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
                ConstantExpression::create_empty_expression(Constant::Integer(3, NumberFormat::Default)),
            ],
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );

    check_statement(
        "LET A[1, 2, 3] = 5",
        &LetStatement::create_empty_statement(
            unicase::Ascii::new("A".to_string()),
            Token::Eq,
            vec![
                ConstantExpression::create_empty_expression(Constant::Integer(1, NumberFormat::Default)),
                ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
                ConstantExpression::create_empty_expression(Constant::Integer(3, NumberFormat::Default)),
            ],
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
}

#[test]
fn check_let_without_let_statement() {
    check_statement(
        "A = 5",
        &LetStatement::create_empty_statement(
            unicase::Ascii::new("A".to_string()),
            Token::Eq,
            vec![],
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );

    check_statement(
        "A(1, 2, 3) = 5",
        &LetStatement::create_empty_statement(
            unicase::Ascii::new("A".to_string()),
            Token::Eq,
            vec![
                ConstantExpression::create_empty_expression(Constant::Integer(1, NumberFormat::Default)),
                ConstantExpression::create_empty_expression(Constant::Integer(2, NumberFormat::Default)),
                ConstantExpression::create_empty_expression(Constant::Integer(3, NumberFormat::Default)),
            ],
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
}
/*
#[test]
fn check_let_with_keywords() {
    check_statement(
        "LOOP = 5",
        &LetStatement::create_empty_statement(
            unicase::Ascii::new("LOOP".to_string()),
            Token::Eq,
            vec![],
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );

    check_statement(
        "QUIT(1, 2, 3) = 5",
        &LetStatement::create_empty_statement(
            unicase::Ascii::new("QUIT".to_string()),
            Token::Eq,
            vec![
                ConstantExpression::create_empty_expression(Constant::Integer(1)),
                ConstantExpression::create_empty_expression(Constant::Integer(2)),
                ConstantExpression::create_empty_expression(Constant::Integer(3)),
            ],
            ConstantExpression::create_empty_expression(Constant::Integer(5)),
        ),
    );
}*/

#[test]
fn test_variable_declaration_statement() {
    check_statement(
        "BOOLEAN VAR001",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::Boolean,
            vec![VariableSpecifier::empty(unicase::Ascii::new("VAR001".to_string()), vec![])],
        ),
    );

    check_statement(
        "INTEGER VAR001",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::Integer,
            vec![VariableSpecifier::empty(unicase::Ascii::new("VAR001".to_string()), vec![])],
        ),
    );

    check_statement(
        "Money VAR001",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::Money,
            vec![VariableSpecifier::empty(unicase::Ascii::new("VAR001".to_string()), vec![])],
        ),
    );

    check_statement(
        "Money VAR001",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::Money,
            vec![VariableSpecifier::empty(unicase::Ascii::new("VAR001".to_string()), vec![])],
        ),
    );

    check_statement(
        "String VAR001",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::String,
            vec![VariableSpecifier::empty(unicase::Ascii::new("VAR001".to_string()), vec![])],
        ),
    );

    check_statement(
        "Time VAR001",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::Time,
            vec![VariableSpecifier::empty(unicase::Ascii::new("VAR001".to_string()), vec![])],
        ),
    );

    check_statement(
        "Date VAR001",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::Date,
            vec![VariableSpecifier::empty(unicase::Ascii::new("VAR001".to_string()), vec![])],
        ),
    );

    check_statement(
        "DDate VAR001",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::DDate,
            vec![VariableSpecifier::empty(unicase::Ascii::new("VAR001".to_string()), vec![])],
        ),
    );

    check_statement(
        "Byte VAR001",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::Byte,
            vec![VariableSpecifier::empty(unicase::Ascii::new("VAR001".to_string()), vec![])],
        ),
    );

    check_statement(
        "UByte VAR001",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::Byte,
            vec![VariableSpecifier::empty(unicase::Ascii::new("VAR001".to_string()), vec![])],
        ),
    );

    check_statement(
        "Word VAR001",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::Word,
            vec![VariableSpecifier::empty(unicase::Ascii::new("VAR001".to_string()), vec![])],
        ),
    );

    check_statement(
        "SByte VAR001",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::SByte,
            vec![VariableSpecifier::empty(unicase::Ascii::new("VAR001".to_string()), vec![])],
        ),
    );

    check_statement(
        "SWord VAR001",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::SWord,
            vec![VariableSpecifier::empty(unicase::Ascii::new("VAR001".to_string()), vec![])],
        ),
    );

    check_statement(
        "BigStr VAR001",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::BigStr,
            vec![VariableSpecifier::empty(unicase::Ascii::new("VAR001".to_string()), vec![])],
        ),
    );

    check_statement(
        "Real VAR001",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::Float,
            vec![VariableSpecifier::empty(unicase::Ascii::new("VAR001".to_string()), vec![])],
        ),
    );

    check_statement(
        "Float VAR001",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::Float,
            vec![VariableSpecifier::empty(unicase::Ascii::new("VAR001".to_string()), vec![])],
        ),
    );

    check_statement(
        "DReal VAR001",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::Double,
            vec![VariableSpecifier::empty(unicase::Ascii::new("VAR001".to_string()), vec![])],
        ),
    );

    check_statement(
        "Double VAR001",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::Double,
            vec![VariableSpecifier::empty(unicase::Ascii::new("VAR001".to_string()), vec![])],
        ),
    );
}

#[test]
fn test_dim_variable_declaration_statement() {
    check_statement(
        "INTEGER A(4)",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::Integer,
            vec![VariableSpecifier::empty(unicase::Ascii::new("A".to_string()), vec![4])],
        ),
    );
    check_statement(
        "INTEGER A(4, 5)",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::Integer,
            vec![VariableSpecifier::empty(unicase::Ascii::new("A".to_string()), vec![4, 5])],
        ),
    );
    check_statement(
        "INTEGER A(4, 5, 6)",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::Integer,
            vec![VariableSpecifier::empty(unicase::Ascii::new("A".to_string()), vec![4, 5, 6])],
        ),
    );
}

#[test]
fn test_repeat_until_statement() {
    check_statement(
        r"REPEAT
        UNTIL A",
        &RepeatUntilStatement::create_empty_statement(IdentifierExpression::create_empty_expression(unicase::Ascii::new("A".to_string())), vec![]),
    );
}

#[test]
fn test_loop_statement() {
    check_statement(
        r"LOOP
        ENDLOOP",
        &LoopStatement::create_empty_statement(vec![]),
    );
    check_statement(
        r"LOOP
        END LOOP",
        &LoopStatement::create_empty_statement(vec![]),
    );
}

#[test]
fn test_variable_declaration_initalizer() {
    check_statement(
        "INTEGER VAR001=42",
        &VariableDeclarationStatement::create_empty_statement(
            VariableType::Integer,
            vec![VariableSpecifier::new(
                Spanned::create_empty(Token::Identifier(unicase::Ascii::new("VAR001".to_string()))),
                None,
                Vec::new(),
                None,
                None,
                Some(ConstantExpression::create_empty_expression(Constant::Integer(42, NumberFormat::Default))),
            )],
        ),
    );
}

#[test]
fn check_assign_variants() {
    check_statement(
        "A += 5",
        &LetStatement::create_empty_statement(
            unicase::Ascii::new("A".to_string()),
            Token::AddAssign,
            vec![],
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );

    check_statement(
        "A /= 5",
        &LetStatement::create_empty_statement(
            unicase::Ascii::new("A".to_string()),
            Token::DivAssign,
            vec![],
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );

    check_statement(
        "LET A &= 5",
        &LetStatement::create_empty_statement(
            unicase::Ascii::new("A".to_string()),
            Token::AndAssign,
            vec![],
            ConstantExpression::create_empty_expression(Constant::Integer(5, NumberFormat::Default)),
        ),
    );
}
