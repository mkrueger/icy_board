use for_next::scan_for_next;

use crate::ast::RenameVisitor;

use self::{if_else::scan_if_else, while_do::scan_do_while};

use super::{constant_scan_visitor::ConstantScanVisitor, rename_visitor::RenameScanVistitor, Ast, Expression, Statement};

mod if_else;
mod while_do;
mod for_next;

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
    //  scan_if(statements);

    //    scan_select_statements(statements);

    //    strip_unused_labels(statements);
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

pub fn strip_unused_labels(statements: &mut Vec<Statement>) {
    let mut used_labels = HashSet::new();
    let mut i = 0;
    while i < statements.len() {
        gather_labels(&statements[i], &mut used_labels);
        i += 1;
    }
    strip_unused_labels2(statements, &used_labels);
}

fn strip_unused_labels2(
    statements: &mut Vec<Statement>,
    used_labels: &HashSet<unicase::Ascii<String>>,
) {
    let mut i = 0;
    while i < statements.len() {
        if let Statement::Label(label) = &statements[i] {
            if !used_labels.contains(label.get_label()) {
                statements.remove(i);
                continue;
            }
        }

        match &mut statements[i] {
            Statement::IfThen(if_then_stmt) => {
                strip_unused_labels2(if_then_stmt.get_statements_mut(), used_labels);
                for else_if_block in if_then_stmt.get_else_if_blocks_mut() {
                    strip_unused_labels2(else_if_block.get_statements_mut(), used_labels);
                }
                if let Some(else_block) = if_then_stmt.get_else_block_mut() {
                    strip_unused_labels2(else_block.get_statements_mut(), used_labels);
                }
            }
            Statement::Select(case_stmt) => {
                for block in case_stmt.get_case_blocks_mut() {
                    strip_unused_labels2(block.get_statements_mut(), used_labels);
                }
                strip_unused_labels2(case_stmt.get_default_statements_mut(), used_labels);
            }
            Statement::Block(block_stmt) => {
                strip_unused_labels2(block_stmt.get_statements_mut(), used_labels);
            }
            Statement::WhileDo(while_do_stmt) => {
                strip_unused_labels2(while_do_stmt.get_statements_mut(), used_labels);
            }
            Statement::For(for_stmt) => {
                strip_unused_labels2(for_stmt.get_statements_mut(), used_labels);
            }
            _ => {}
        }
        i += 1;
    }
}
*/

#[must_use]
pub fn finish_ast(prg: &mut Ast) -> Ast {
    let mut scanner = RenameScanVistitor::default();
    prg.visit(&mut scanner);
    let prg = prg.visit_mut(&mut ConstantScanVisitor::default());
    let mut renamer = RenameVisitor::new(scanner.rename_map);
    prg.visit_mut(&mut renamer)
}
