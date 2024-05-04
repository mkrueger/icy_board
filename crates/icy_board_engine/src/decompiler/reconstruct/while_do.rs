use crate::{
    ast::{Statement, WhileDoStatement},
    decompiler::reconstruct::scan_goto,
};

use super::reconstruct_block;

/* Compiled Example:

:LABEL004
    IF (!1) GOTO LABEL002
    PRINT "Hello World!"
    GOTO LABEL004
:LABEL002

Was:
WHILE (1) DO
  PRINT "Hello World!"
ENDWHILE
*/
pub fn scan_do_while(statements: &mut Vec<Statement>) {
    scan_do_while_case2(statements);
    let mut i = 0;
    while i + 3 < statements.len() {
        let Statement::Label(while_continue_label) = statements[i].clone() else {
            i += 1;
            continue;
        };
        let Statement::If(next_loop_if) = statements[i + 1].clone() else {
            i += 1;
            continue;
        };
        let Statement::Goto(break_goto) = &next_loop_if.get_statement() else {
            i += 1;
            continue;
        };
        // search "loop" goto
        let Some(matching_goto) = scan_goto(&statements, i + 2, while_continue_label.get_label()) else {
            i += 1;
            continue;
        };

        let Statement::Label(break_label) = &statements[matching_goto + 1] else {
            i += 1;
            continue;
        };
        if break_label.get_label() != break_goto.get_label() {
            i += 1;
            continue;
        }

        // reconstruct while…do block
        let mut while_block = statements.drain((i + 2)..matching_goto as usize).collect();
        statements.drain(i + 1..i + 3);
        reconstruct_block(&mut while_block);
        statements[i + 1] = WhileDoStatement::create_empty_statement(next_loop_if.get_condition().negate_expression(), while_block);
        i += 1;
    }
}

/*

Compiled Example:

:LABEL002
    IF (TRUE) GOTO LABEL001
    GOTO LABEL003
:LABEL001
    PRINT "Hello World!"
    GOTO LABEL002
:LABEL003

Was:
WHILE (TRUE) DO
  PRINT "Hello World!"
ENDWHILE
*/
fn scan_do_while_case2(statements: &mut Vec<Statement>) {
    let mut i = 0;

    while i + 4 < statements.len() {
        let Statement::Label(while_continue_label) = statements[i].clone() else {
            i += 1;
            continue;
        };
        let Statement::If(next_loop_if) = statements[i + 1].clone() else {
            i += 1;
            continue;
        };
        let Statement::Goto(next_loop_goto) = &next_loop_if.get_statement() else {
            i += 1;
            continue;
        };
        let Statement::Goto(break_goto) = statements[i + 2].clone() else {
            i += 1;
            continue;
        };
        let Statement::Label(next_loop_label) = statements[i + 3].clone() else {
            i += 1;
            continue;
        };

        if next_loop_goto.get_label() != next_loop_label.get_label() {
            i += 1;
            continue;
        }
        // search "loop" goto
        let Some(matching_goto) = scan_goto(&statements, i + 4, while_continue_label.get_label()) else {
            i += 1;
            continue;
        };

        let Statement::Label(break_label) = &statements[matching_goto + 1] else {
            i += 1;
            continue;
        };
        if break_label.get_label() != break_goto.get_label() {
            i += 1;
            continue;
        }
        // reconstruct while…do block
        let mut while_block = statements.drain((i + 4)..matching_goto as usize).collect();

        statements.drain(i + 1..i + 4);

        reconstruct_block(&mut while_block);

        statements[i + 1] = WhileDoStatement::create_empty_statement(next_loop_if.get_condition().clone(), while_block);
        i += 1;
    }
}
