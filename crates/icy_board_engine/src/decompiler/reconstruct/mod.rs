use for_next::scan_for_next;
use unicase::Ascii;

use crate::ast::{AstVisitorMut, BreakStatement, ContinueStatement, IfStatement, IfThenStatement, RenameVisitor};

use self::{if_else::scan_if_else, while_do::scan_do_while};

use super::{constant_scan_visitor::ConstantScanVisitor, rename_visitor::RenameScanVisitor, Ast, Expression, Statement};

pub mod for_next;
mod if_else;
mod remove_label_visitor;
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
    scan_if_else(statements);
    scan_if(statements);

    //    scan_select_statements(statements);
    strip_unused_labels(statements);
}

fn optimize_ifs(statements: &mut Vec<Statement>) {
    scan_if_else(statements);
    scan_if(statements);
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
    if statements.len() < 3 {
        return;
    }
    // scan:
    // IF (COND) GOTO SKIP
    // STATEMENTS
    // :SKIP
    for i in 0..statements.len() - 2 {
        let Statement::If(if_stmt) = statements[i].clone() else {
            continue;
        };
        let Statement::Goto(endif_label) = if_stmt.get_statement() else {
            continue;
        };

        // check skip label
        let endif_label_index = get_label_index(statements, i as i32 + 1, statements.len() as i32, endif_label.get_label());
        if endif_label_index.is_none() {
            continue;
        }

        // replace if with if…then
        // do not remove labels they may be needed to analyze other constructs
        let mut statements2 = statements.drain((i + 1)..(endif_label_index.unwrap() as usize)).collect();
        optimize_loops(&mut statements2);
        optimize_ifs(&mut statements2);
        if statements2.len() == 1 {
            statements[i] = IfStatement::create_empty_statement(if_stmt.get_condition().negate_expression(), statements2.pop().unwrap());
        } else {
            statements[i] = IfThenStatement::create_empty_statement(if_stmt.get_condition().negate_expression(), statements2, Vec::new(), None);
        }
        scan_if(statements);
        break;
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

/*
fn get_first(s: &[Statement]) -> Option<&Statement> {
    if s.is_empty() {
        return None;
    }
    if let Statement::If(_) = s[0] {
        return Some(&s[0]);
    }
    if s.len() < 2 {
        return None;
    }

    Some(&s[1])
}*/
/*
fn scan_select_statements(statements: &mut [Statement]) {
    let mut i = 0;
    while i < statements.len() {
        if let Statement::IfThen(if_then_stmt) = statements[i].clone() {
            if if_then_stmt.get_else_block().is_none() {
                i += 1;
                continue;
            }
            let Expression::Binary(bin_expr) =
                Statement::try_boolean_conversion(if_then_stmt.get_condition())
            else {
                i += 1;
                continue;
            };
            if bin_expr.get_op() != BinOp::Eq {
                i += 1;
                continue;
            }

            let mut skip = false;
            for if_else_block in if_then_stmt.get_else_if_blocks() {
                let Expression::Binary(bin_expr2) =
                    Statement::try_boolean_conversion(if_else_block.get_condition())
                else {
                    skip = true;
                    break;
                };
                if bin_expr2.get_op() != BinOp::Eq {
                    skip = true;
                    break;
                }
                if bin_expr.get_left_expression() != bin_expr2.get_left_expression() {
                    skip = true;
                    break;
                }
            }

            if skip {
                i += 1;
                continue;
            }

            let mut case_blocks = Vec::new();

            if !if_then_stmt.get_statements().is_empty() {
                case_blocks.push(CaseBlock::empty(
                    vec![CaseSpecifier::Expression(Box::new(
                        bin_expr.get_right_expression().clone(),
                    ))],
                    if_then_stmt.get_statements().clone(),
                ));
            }

            for if_else_block in if_then_stmt.get_else_if_blocks() {
                let Expression::Binary(bin_expr2) =
                    Statement::try_boolean_conversion(if_else_block.get_condition())
                else {
                    i += 1;
                    continue;
                };

                if bin_expr2.get_op() != BinOp::Eq {
                    i += 1;
                    continue;
                }
                case_blocks.push(CaseBlock::empty(
                    vec![CaseSpecifier::Expression(Box::new(
                        bin_expr2.get_right_expression().clone(),
                    ))],
                    if_else_block.get_statements().clone(),
                ));
            }
            let default_statements = if let Some(smts) = if_then_stmt.get_else_block() {
                smts.get_statements().clone()
            } else {
                Vec::new()
            };
            statements[i] = SelectStatement::create_empty_statement(
                bin_expr.get_left_expression().clone(),
                case_blocks,
                default_statements,
            );
        }
        i += 1;
    }
}
*/

/*
fn gather_labels(stmt: &Statement, used_labels: &mut HashSet<unicase::Ascii<String>>) {
    match stmt {
        Statement::If(if_stmt) => {
            gather_labels(if_stmt.get_statement(), used_labels);
        }
        Statement::While(while_stmt) => {
            gather_labels(while_stmt.get_statement(), used_labels);
        }
        Statement::IfThen(if_then_stmt) => {
            for stmt in if_then_stmt.get_statements() {
                gather_labels(stmt, used_labels);
            }
            for block in if_then_stmt.get_else_if_blocks() {
                for stmt in block.get_statements() {
                    gather_labels(stmt, used_labels);
                }
            }
            if let Some(stmts) = if_then_stmt.get_else_block() {
                for stmt in stmts.get_statements() {
                    gather_labels(stmt, used_labels);
                }
            }
        }
        Statement::Select(select_stmt) => {
            for block in select_stmt.get_case_blocks() {
                for stmt in block.get_statements() {
                    gather_labels(stmt, used_labels);
                }
            }
            for stmt in select_stmt.get_default_statements() {
                gather_labels(stmt, used_labels);
            }
        }
        Statement::Block(block_stmt) => {
            for stmt in block_stmt.get_statements() {
                gather_labels(stmt, used_labels);
            }
        }
        Statement::WhileDo(while_do_stmt) => {
            for stmt in while_do_stmt.get_statements() {
                gather_labels(stmt, used_labels);
            }
        }
        Statement::For(for_stmt) => {
            for stmt in for_stmt.get_statements() {
                gather_labels(stmt, used_labels);
            }
        }
        Statement::Goto(goto_stmt) => {
            used_labels.insert(goto_stmt.get_label().clone());
        }
        Statement::Gosub(gosub_stmt) => {
            used_labels.insert(gosub_stmt.get_label().clone());
        }
        _ => {}
    }
}
*/

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
        println!("goto: {} break:{}  continue:{}", goto.get_label(), self.break_label, self.continue_label);
        if self.break_label.len() > 0 && goto.get_label() == &self.break_label {
            BreakStatement::create_empty_statement()
        } else if self.continue_label.len() > 0 && goto.get_label() == &self.continue_label {
            ContinueStatement::create_empty_statement()
        } else {
            Statement::Goto(goto.clone())
        }
    }
}
