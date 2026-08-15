use crate::{ast::RepeatUntilStatement, ast::Statement, semantic::SemanticVisitor};

use super::optimize_block;

/* Compiled Example:

:LABEL001
    PRINT "Hello World!"
:LABEL002
    IF (!BOOL001) GOTO LABEL001
:LABEL003

Was:
REPEAT
  PRINT "Hello World!"
UNTIL BOOL001
*/
pub fn scan_repeat_until(visitor: &SemanticVisitor, statements: &mut Vec<Statement>, lang_version: u16) {
    let mut i = 0;
    while i + 2 < statements.len() {
        let Statement::Label(head_label) = statements[i].clone() else {
            i += 1;
            continue;
        };
        let head_label = head_label.get_label().clone();

        let Some(back_edge) = scan_back_edge(statements, i + 1, &head_label) else {
            i += 1;
            continue;
        };
        let Statement::If(until_if) = statements[back_edge].clone() else {
            i += 1;
            continue;
        };
        // A WHILE tests before the body, so a back edge right after the head is not a REPEAT.
        if back_edge <= i + 1 {
            i += 1;
            continue;
        }
        if back_edge + 1 >= statements.len() {
            i += 1;
            continue;
        }
        let Statement::Label(break_label) = &statements[back_edge + 1] else {
            i += 1;
            continue;
        };
        let break_label = break_label.get_label().clone();

        let mut body: Vec<Statement> = statements.drain((i + 1)..back_edge).collect();
        statements.remove(i + 1);

        let continue_label = super::get_last_label(&body);
        super::handle_break_continue(break_label, continue_label, &mut body);
        optimize_block(visitor, &mut body, lang_version);

        statements.insert(
            i + 1,
            RepeatUntilStatement::create_empty_statement(until_if.get_condition().negate_expression(), body),
        );
        i += 1;
    }
}

/// The jump back to the head that closes the loop: the last statement of the body,
/// with the break label right behind it.
fn scan_back_edge(statements: &[Statement], from: usize, head: &unicase::Ascii<String>) -> Option<usize> {
    for j in from..statements.len().saturating_sub(1) {
        let Statement::If(if_stmt) = &statements[j] else {
            continue;
        };
        let Statement::Goto(goto_stmt) = if_stmt.get_statement() else {
            continue;
        };
        if goto_stmt.get_label() != head {
            continue;
        }
        if matches!(&statements[j + 1], Statement::Label(_)) {
            return Some(j);
        }
    }
    None
}
