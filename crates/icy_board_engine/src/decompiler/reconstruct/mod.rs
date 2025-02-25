use for_next::scan_for_next;
use select_case::scan_select_statements;
use unicase::Ascii;

use crate::{
    ast::{AstVisitorMut, BreakStatement, ContinueStatement, IfStatement, IfThenStatement, RenameVisitor},
    executable::OpCode,
    semantic::{ReferenceType, SemanticVisitor},
};

use self::while_do::scan_do_while;

use super::{Ast, Expression, Statement, rename_visitor::RenameScanVisitor};

pub mod for_next;
mod if_else;
mod remove_label_visitor;
mod select_case;
mod unused_label_visitor;
mod while_do;

pub fn reconstruct_block(visitor: &SemanticVisitor, statements: &mut Vec<Statement>) {
    optimize_block(visitor, statements);
}

fn _optimize_argument(arg: &mut Expression) {
    if let Expression::Parens(expr) = arg {
        *arg = expr.get_expression().clone();
    }
}

pub fn optimize_loops(visitor: &SemanticVisitor, statements: &mut Vec<Statement>) {
    scan_for_next(visitor, statements);
    scan_do_while(visitor, statements);
}

fn optimize_block(visitor: &SemanticVisitor, statements: &mut Vec<Statement>) {
    optimize_loops(visitor, statements);
    optimize_ifs(visitor, statements);
    scan_select_statements(statements);
}

fn optimize_ifs(visitor: &SemanticVisitor, statements: &mut Vec<Statement>) {
    scan_negated_if(visitor, statements);
    scan_if(visitor, statements);
    if_else::scan_if_else(visitor, statements);
}

fn scan_label(statements: &[Statement], from: usize, label: &unicase::Ascii<String>) -> Option<usize> {
    for j in from..statements.len() {
        if let Statement::Label(label_stmt) = &statements[j as usize] {
            if label_stmt.get_label() == label {
                return Some(j as usize);
            }
        }
    }
    None
}

fn scan_goto(statements: &[Statement], from: usize, label: &unicase::Ascii<String>) -> Option<usize> {
    for j in from..statements.len() {
        if let Statement::Goto(goto_stmt) = &statements[j as usize] {
            if goto_stmt.get_label() == label {
                return Some(j as usize);
            }
        }
    }
    None
}

// scan:
// IF (COND) GOTO SKIP
// STMT
// :SKIP
//
// replace with:
// IF !COND STMT
// :SKIP
fn scan_negated_if(_visitor: &SemanticVisitor, statements: &mut Vec<Statement>) {
    // scan:
    // IF (COND) GOTO SKIP
    // STATEMENTS..
    // :SKIP
    let mut i: usize = 0;
    while i + 2 < statements.len() {
        let label = if let Statement::Label(label_stmt) = &statements[i + 2] {
            label_stmt.get_label().clone()
        } else {
            i += 1;
            continue;
        };
        let Statement::If(mut if_stmt) = statements[i].clone() else {
            i += 1;
            continue;
        };

        if let Statement::Goto(endif_label) = if_stmt.get_statement() {
            if *endif_label.get_label() == label {
                let statement = statements.remove(i + 1);
                if_stmt.set_condition(if_stmt.get_condition().negate_expression());
                if_stmt.set_statement(statement);
                statements[i] = Statement::If(if_stmt);
            }
            i += 1;
        } else {
            i += 1;
            continue;
        }
    }
}

fn scan_if(visitor: &SemanticVisitor, statements: &mut Vec<Statement>) {
    // scan:
    // IF (COND) GOTO SKIP
    // STATEMENTS..
    // :SKIP
    let mut i: usize = 0;
    while i + 2 < statements.len() {
        let Statement::If(if_stmt) = statements[i].clone() else {
            i += 1;
            continue;
        };
        let Statement::Goto(endif_label) = if_stmt.get_statement() else {
            i += 1;
            continue;
        };

        // check skip label
        let Some(endif_label_index) = get_label_index(statements, i as i32 + 1, statements.len() as i32, endif_label.get_label()) else {
            i += 1;
            continue;
        };
        let remove_goto = visitor
            .references
            .iter()
            .find(|&(t, r)| {
                if !matches!(t, ReferenceType::Label(_)) {
                    return false;
                }
                let end_label = format!(":{}", endif_label.get_label());
                if let Some((_, decl)) = &r.declaration {
                    if decl.token == end_label {
                        return r.usages.len() == 1;
                    }
                }
                false
            })
            .is_some();
        if i + 1 >= endif_label_index {
            // don't generate if…then for empty if…then
            i += 1;
            continue;
        }
        if remove_goto {
            statements.remove(endif_label_index);
        }

        // replace if with if…then
        let mut statements2: Vec<Statement> = statements.drain((i + 1)..endif_label_index).collect();
        optimize_block(visitor, &mut statements2);
        if statements2.len() == 1 && is_simple_statement(&statements[0]) {
            statements[i] = IfStatement::create_empty_statement(if_stmt.get_condition().negate_expression(), statements2.pop().unwrap());
        } else {
            statements[i] = IfThenStatement::create_empty_statement(if_stmt.get_condition().negate_expression(), statements2, Vec::new(), None);
        }
    }
}

fn is_simple_statement(statements: &Statement) -> bool {
    match statements {
        Statement::Gosub(_) => true,
        Statement::Goto(_) => true,
        Statement::PredifinedCall(pcall) => match pcall.get_func().opcode {
            OpCode::RETURN => true,
            OpCode::END => true,
            OpCode::STOP => true,
            OpCode::PRINT => true,
            OpCode::PRINTLN => true,
            _ => false,
        },
        _ => false,
    }
}

fn get_label_index(statements: &[Statement], from: i32, to: i32, label: &String) -> Option<usize> {
    for j in from..to {
        if let Statement::Label(next_label) = &statements[j as usize] {
            if next_label.get_label() == label {
                return Some(j as usize);
            }
        }
    }
    None
}

pub fn strip_unused_labels(ast: &mut Ast) -> Ast {
    let mut visitor = unused_label_visitor::UnusedLabelVisitor::default();
    ast.visit(&mut visitor);
    let unused_labels = visitor.get_unused_labels();
    let mut visitor = remove_label_visitor::RemoveLabelVisitor::new(unused_labels.clone());
    ast.visit_mut(&mut visitor)
}

#[must_use]
pub fn finish_ast(prg: &mut Ast) -> Ast {
    let mut scanner = RenameScanVisitor::default();
    prg.visit(&mut scanner);
    let mut renamer = RenameVisitor::new(scanner.rename_map);
    prg.visit_mut(&mut renamer)
}

pub fn get_last_label(statements: &[Statement]) -> Ascii<String> {
    if let Some(Statement::Label(continue_label_stmt)) = statements.last() {
        continue_label_stmt.get_label().clone()
    } else {
        Ascii::new("".to_string())
    }
}

pub fn handle_break_continue(break_label: Ascii<String>, continue_label: Ascii<String>, statements: &mut Vec<Statement>) {
    let mut break_continue_visitor = BreakContinueVisitor::new(break_label, continue_label);
    for stmt in statements.iter_mut() {
        *stmt = stmt.visit_mut(&mut break_continue_visitor);
    }
}
struct BreakContinueVisitor {
    break_label: unicase::Ascii<String>,
    continue_label: unicase::Ascii<String>,
}
impl BreakContinueVisitor {
    fn new(break_label: unicase::Ascii<String>, continue_label: unicase::Ascii<String>) -> Self {
        Self { break_label, continue_label }
    }
}

impl AstVisitorMut for BreakContinueVisitor {
    fn visit_goto_statement(&mut self, goto: &crate::ast::GotoStatement) -> Statement {
        if self.break_label.len() > 0 && goto.get_label() == &self.break_label {
            BreakStatement::create_empty_statement()
        } else if self.continue_label.len() > 0 && goto.get_label() == &self.continue_label {
            ContinueStatement::create_empty_statement()
        } else {
            Statement::Goto(goto.clone())
        }
    }
}
