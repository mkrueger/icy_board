use crate::{ast::LoopStatement, ast::Statement, semantic::SemanticVisitor};

use super::optimize_block;

/* Compiled Example:

:LABEL001
    PRINT "Hello World!"
    GOTO LABEL001
:LABEL002

Was:
LOOP
  PRINT "Hello World!"
ENDLOOP
*/
pub fn scan_loop(visitor: &SemanticVisitor, statements: &mut Vec<Statement>, lang_version: u16) {
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
        let Statement::Label(break_label) = &statements[back_edge + 1] else {
            i += 1;
            continue;
        };
        let break_label = break_label.get_label().clone();

        let mut body: Vec<Statement> = statements.drain((i + 1)..back_edge).collect();
        statements.remove(i + 1);

        super::handle_break_continue(break_label, head_label, &mut body);
        optimize_block(visitor, &mut body, lang_version);

        statements.insert(i + 1, LoopStatement::create_empty_statement(body));
        i += 1;
    }
}

/// The unconditional jump back to the head, with the break label right behind it.
fn scan_back_edge(statements: &[Statement], from: usize, head: &unicase::Ascii<String>) -> Option<usize> {
    for j in from..statements.len().saturating_sub(1) {
        let Statement::Goto(goto_stmt) = &statements[j] else {
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
