use crate::{
    ast::{ElseBlock, ElseIfBlock, Statement},
    semantic::SemanticVisitor,
};

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
        GOTO LABEL001
    ENDIF

    IF (BOOL002 | BOOL001) THEN
        PRINT "ELSEIF1"
        GOTO LABEL001
    ENDIF

    IF (!(BOOL002 | BOOL001)) THEN
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
*/

pub fn scan_if_else(visitor: &SemanticVisitor, statements: &mut Vec<Statement>) {
    let mut i = 0;
    while i + 1 < statements.len() {
        let start = i;
        let Statement::IfThen(if_stmt) = &statements[i] else {
            i += 1;
            continue;
        };
        let Some(Statement::Goto(breakout_goto_stmt)) = if_stmt.get_statements().last().cloned() else {
            i += 1;
            continue;
        };
        let Some(mut end_label_idx) = super::scan_label(statements, i, breakout_goto_stmt.get_label()) else {
            i += 1;
            continue;
        };
        let mut if_stmt = if_stmt.clone();
        if_stmt.get_statements_mut().pop(); // pop goto

        let j: usize = i + 1;
        while j < statements.len() - 1 {
            let Statement::IfThen(else_if_stmt) = &statements[j] else {
                break;
            };
            if let Some(Statement::Goto(goto_stmt)) = else_if_stmt.get_statements().last().cloned() {
                if goto_stmt.get_label() == breakout_goto_stmt.get_label() {
                    let mut else_if_block = else_if_stmt.get_statements().clone();
                    else_if_block.pop(); // pop goto
                    if_stmt
                        .get_else_if_blocks_mut()
                        .push(ElseIfBlock::empty(else_if_stmt.get_condition().clone(), else_if_block));

                    statements.remove(j);
                    end_label_idx -= 1;

                    continue;
                }
            }
            break;
        }

        if j < end_label_idx {
            let mut stmts = statements.drain(j..end_label_idx).collect();
            optimize_block(visitor, &mut stmts);
            let mut is_else_if = false;
            if stmts.len() == 1 {
                if let Statement::IfThen(if_then_stmt) = &stmts[0] {
                    is_else_if = true;
                    if_stmt
                        .get_else_if_blocks_mut()
                        .push(ElseIfBlock::empty(if_then_stmt.get_condition().clone(), if_then_stmt.get_statements().clone()));
                } else if let Statement::If(if_then_stmt) = &stmts[0] {
                    is_else_if = true;
                    if_stmt.get_else_if_blocks_mut().push(ElseIfBlock::empty(
                        if_then_stmt.get_condition().clone(),
                        vec![if_then_stmt.get_statement().clone()],
                    ));
                }
            }

            if !is_else_if {
                *if_stmt.get_else_block_mut() = Some(ElseBlock::empty(stmts));
            }
        }

        statements[start] = Statement::IfThen(if_stmt);
        i += 1;
    }
}
