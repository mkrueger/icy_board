use std::ops::Range;

use crate::ast::{ElseBlock, ElseIfBlock, Expression, IfThenStatement, Statement};

use super::scan_label;

/*

Simple:

IF (BOOL001) GOTO LABEL001
GOTO LABEL002
:LABEL001
PRINT "Hello World!"
:LABEL002


Complex:

IF (BOOL001) GOTO LABEL008
GOTO LABEL001

:LABEL008
PRINT "TRUE"
GOTO LABEL006

:LABEL001
IF (BOOL002 | BOOL001) GOTO LABEL005
GOTO LABEL009

:LABEL005
PRINT "ELSEIF1"
GOTO LABEL006

:LABEL009
IF (!BOOL002 & !BOOL001) GOTO LABEL011
GOTO LABEL003

:LABEL011
PRINT "ELSEIF2"
GOTO LABEL006

:LABEL003
PRINT "ELSE"

:LABEL006


Was:
IF (BOOL001) THEN
  PRINT "TRUE"
ELSEIF (BOOL002 | BOOL001) THEN
  PRINT "ELSEIF1"
ELSEIF (!BOOL002 & !BOOL001) THEN
  PRINT "ELSEIF2"
ELSE
  PRINT "ELSE"
ENDIF

*/

pub fn scan_if_else(statements: &mut Vec<Statement>) {
    let mut i = 0;
    let mut start_if = usize::MAX;
    let mut conditions: Vec<Expression> = Vec::new();
    let mut if_blocks: Vec<Range<usize>> = Vec::new();
    let mut scan_next = false;
    let mut exit_goto = None;
    while !if_blocks.is_empty() || i < statements.len() {
        if !scan_next && !if_blocks.is_empty() {
            let mut else_if_blocks = Vec::new();
            for i in 1..if_blocks.len() {
                let rng = if_blocks[i].clone();

                let else_if_block = ElseIfBlock::empty(conditions[i].clone(), statements[rng.start..rng.end].iter().cloned().collect());
                else_if_blocks.push(else_if_block);
            }
            let end = if_blocks.last().unwrap().end;
            let else_block = if let Some(_exit_goto) = exit_goto {
                let rng = end + 2..i;
                if rng.start < rng.end {
                    Some(ElseBlock::empty(statements[rng].iter().cloned().collect()))
                } else {
                    None
                }
            } else {
                None
            };

            let rng = if_blocks[0].clone();
            let stmt = IfThenStatement::create_empty_statement(
                conditions[0].clone(),
                statements[rng.start..rng.end].iter().cloned().collect(),
                else_if_blocks,
                else_block,
            );

            conditions.clear();
            if_blocks.clear();
            statements.drain(start_if + 1..i);
            statements[start_if] = stmt;
            i = start_if + 1;
            start_if = usize::MAX;
            exit_goto = None;
            continue;
        }

        if i + 2 >= statements.len() {
            break;
        }

        let Statement::If(if_stmt) = statements[i].clone() else {
            scan_next = false;
            i += 1;
            continue;
        };
        let Statement::Goto(false_goto) = &if_stmt.get_statement() else {
            scan_next = false;
            i += 1;
            continue;
        };
        let Statement::Goto(true_goto) = statements[i + 1].clone() else {
            scan_next = false;
            i += 1;
            continue;
        };
        let Statement::Label(false_label) = statements[i + 2].clone() else {
            scan_next = false;
            i += 1;
            continue;
        };
        if false_goto.get_label() != false_label.get_label() {
            scan_next = false;
            i += 1;
            continue;
        }
        if start_if > i {
            start_if = i;
        }
        println!("IF TRUE: {:?}/{}", true_goto.get_label(), if_stmt.get_condition());

        let Some(true_goto) = scan_label(&statements, i + 3, true_goto.get_label()) else {
            scan_next = false;
            i += 1;
            continue;
        };
        conditions.push(if_stmt.get_condition().clone());

        if let Statement::Goto(exit_goto_stmt) = statements[true_goto - 1].clone() {
            exit_goto = scan_label(&statements, true_goto + 1, exit_goto_stmt.get_label());
            scan_next = true;
            if_blocks.push(i + 3..true_goto - 1);
        } else {
            scan_next = false;
            if_blocks.push(i + 3..true_goto);
        }
        i = true_goto + 1;
    }
}
