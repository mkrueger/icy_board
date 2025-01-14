use crate::ast::{ElseBlock, ElseIfBlock, Statement};

use super::optimize_block;

/*

Simple:

BOOLEAN BOOL001
BOOLEAN BOOL002
    BOOL001 = FALSE
    BOOL002 = TRUE
    IF (BOOL001) THEN
        PRINT "TRUE"
        GOTO LABEL001
    ENDIF

    IF (BOOL002 | BOOL001) THEN
        PRINT "ELSEIF1"
        GOTO LABEL001
    ENDIF

    IF (!BOOL002 & !BOOL001) THEN
        PRINT "ELSEIF2"
        GOTO LABEL001
    ENDIF

    PRINT "ELSE"
:LABEL001
    IF (BOOL001) THEN
        PRINT "TRUE"
        GOTO LABEL002
    ENDIF

    IF (BOOL002 | BOOL001) THEN
        PRINT "ELSEIF1"
    ENDIF

:LABEL002

Was:
BOOLEAN BOOL001
BOOLEAN BOOL002
BOOL001 = FALSE
BOOL002 = TRUE

IF (BOOL001) THEN
  PRINT "TRUE"
ELSEIF (BOOL002 | BOOL001) THEN
  PRINT "ELSEIF1"
ELSEIF (!BOOL002 & !BOOL001) THEN
  PRINT "ELSEIF2"
ELSE
  PRINT "ELSE"
ENDIF

IF (BOOL001) THEN
  PRINT "TRUE"
ELSEIF (BOOL002 | BOOL001) THEN
  PRINT "ELSEIF1"
ENDIF
*/

pub fn scan_if_else(statements: &mut Vec<Statement>) {
    super::strip_unused_labels(statements);
    let mut i = 0;
    if statements.len() < 1 {
        return;
    }
    while i < statements.len() - 1 {
        let start = i;
        if let Statement::Empty = statements[i] {
            i += 1;
            continue;
        }
        let Statement::IfThen(if_stmt) = &statements[i] else {
            i += 1;
            continue;
        };
        let Some(Statement::Goto(breakout_goto_stmt)) = if_stmt.get_statements().last().cloned() else {
            i += 1;
            continue;
        };
        let Some(mut idx) = super::scan_label(statements, i, breakout_goto_stmt.get_label()) else {
            i += 1;
            continue;
        };
        let mut if_stmt = if_stmt.clone();
        if_stmt.get_statements_mut().pop(); // pop goto

        let mut j = i + 1;
        let mut has_else = true;
        while j < statements.len() - 1 {
            if let Statement::Empty = statements[j] {
                j += 1;
                continue;
            }
            let Statement::IfThen(else_if_stmt) = &statements[j] else {
                break;
            };
            if let Some(Statement::Goto(goto_stmt)) = else_if_stmt.get_statements().last().cloned() {
                has_else = goto_stmt.get_label() == breakout_goto_stmt.get_label();
            } else {
                has_else = false;
            };
            let mut else_if_block = else_if_stmt.get_statements().clone();
            if has_else {
                else_if_block.pop(); // pop goto
            }
            if_stmt
                .get_else_if_blocks_mut()
                .push(ElseIfBlock::empty(else_if_stmt.get_condition().clone(), else_if_block));
            j += 1;
        }

        if has_else && j < idx {
            let mut stmts = statements.drain(j..idx).collect();
            optimize_block(&mut stmts);
            *if_stmt.get_else_block_mut() = Some(ElseBlock::empty(stmts));
            idx = j;
        }
        statements.drain(start + 1..idx);
        statements[start] = Statement::IfThen(if_stmt);
        i += 1;
    }
}
