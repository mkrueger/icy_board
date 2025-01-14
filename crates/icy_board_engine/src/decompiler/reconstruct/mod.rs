use for_next::scan_for_next;
use select_case::scan_select_statements;
use unicase::Ascii;

use crate::ast::{AstVisitorMut, BreakStatement, ContinueStatement, IfStatement, IfThenStatement, RenameVisitor};

use self::{if_else::scan_if_else, while_do::scan_do_while};

use super::{constant_scan_visitor::ConstantScanVisitor, rename_visitor::RenameScanVisitor, Ast, Expression, Statement};

pub mod for_next;
mod if_else;
mod remove_label_visitor;
mod select_case;
mod unused_label_visitor;
mod while_do;

pub fn reconstruct_block(statements: &mut Vec<Statement>) {
    optimize_block(statements);
}

fn _optimize_argument(arg: &mut Expression) {
    if let Expression::Parens(expr) = arg {
        *arg = expr.get_expression().clone();
    }
}

pub fn optimize_loops(statements: &mut Vec<Statement>) {
    scan_for_next(statements);
    scan_do_while(statements);
}

fn optimize_block(statements: &mut Vec<Statement>) {
    optimize_loops(statements);
    optimize_ifs(statements);
    scan_select_statements(statements);
    strip_unused_labels(statements);
}

fn optimize_ifs(statements: &mut Vec<Statement>) {
    scan_if(statements);
    scan_if_else(statements);
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

fn scan_if(statements: &mut Vec<Statement>) {
    // scan:
    // IF (COND) GOTO SKIP
    // STATEMENTS
    // :SKIP
    if statements.len() < 2 {
        return;
    }
    let mut i = 0;
    while i < statements.len() - 2 {
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

        // replace if with if…then
        // do not remove labels they may be needed to analyze other constructs
        let mut statements2: Vec<Statement> = statements.drain((i + 1)..endif_label_index).collect();
        optimize_block(&mut statements2);
        if statements2.len() == 1 {
            statements[i] = IfStatement::create_empty_statement(if_stmt.get_condition().negate_expression(), statements2.pop().unwrap());
        } else {
            statements[i] = IfThenStatement::create_empty_statement(if_stmt.get_condition().negate_expression(), statements2, Vec::new(), None);
        }
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

pub fn strip_unused_labels(statements: &mut Vec<Statement>) {
    let mut visitor = unused_label_visitor::UnusedLabelVisitor::default();
    for stmt in statements.clone() {
        stmt.visit(&mut visitor);
    }
    let unused_labels = visitor.get_unused_labels();
    let mut visitor = remove_label_visitor::RemoveLabelVisitor::new(unused_labels.clone());
    for stmt in statements {
        *stmt = stmt.visit_mut(&mut visitor);
    }
}

#[must_use]
pub fn finish_ast(prg: &mut Ast) -> Ast {
    let mut scanner = RenameScanVisitor::default();
    prg.visit(&mut scanner);
    let prg = prg.visit_mut(&mut ConstantScanVisitor::default());
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
