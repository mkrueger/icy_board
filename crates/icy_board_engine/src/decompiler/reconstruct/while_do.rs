use crate::{
    ast::{Statement, WhileDoStatement, WhileStatement},
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
        let break_label = break_label.get_label().clone();
        if break_label != break_goto.get_label() {
            i += 1;
            continue;
        }

        // reconstruct while…do block
        let mut while_block = statements.drain((i + 2)..matching_goto as usize).collect();
        statements.drain(i + 1..i + 3);
        let continue_label = super::get_last_label(&statements[i..i + 1]);
        super::handle_break_continue(break_label, continue_label, statements);
        println!("111111");
        reconstruct_block(&mut while_block);

        if while_block.len() == 1 {
            statements[i + 1] = WhileStatement::create_empty_statement(next_loop_if.get_condition().negate_expression().clone(), while_block.pop().unwrap());
        } else {
            statements[i + 1] = WhileDoStatement::create_empty_statement(next_loop_if.get_condition().negate_expression().clone(), while_block);
        }
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
        let while_continue_label = while_continue_label.get_label().clone();
        let next_loop_label = next_loop_label.get_label().clone();
        if next_loop_goto.get_label() != &next_loop_label {
            i += 1;
            continue;
        }
        // search "loop" goto
        let Some(matching_goto) = scan_goto(&statements, i + 4, &while_continue_label) else {
            i += 1;
            continue;
        };

        let Statement::Label(break_label) = &statements[matching_goto + 1] else {
            i += 1;
            continue;
        };
        let break_label = break_label.get_label().clone();
        if break_label != break_goto.get_label() {
            i += 1;
            continue;
        }
        // reconstruct while…do block
        let mut while_block: Vec<Statement> = statements.drain((i + 4)..matching_goto as usize).collect();
        let continue_label = super::get_last_label(&while_block);
        statements.drain(i + 1..i + 4);
        reconstruct_block(&mut while_block);
        super::handle_break_continue(break_label, continue_label, &mut while_block);

        if while_block.len() == 1 {
            statements[i + 1] = WhileStatement::create_empty_statement(next_loop_if.get_condition().clone(), while_block.pop().unwrap());
        } else {
            statements[i + 1] = WhileDoStatement::create_empty_statement(next_loop_if.get_condition().clone(), while_block);
        }

        break;
    }
}
