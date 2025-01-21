use crate::{
    ast::{BinOp, Expression, ForStatement, Statement},
    semantic::SemanticVisitor,
};

pub fn scan_for_next<'a>(visitor: &SemanticVisitor<'a>, statements: &mut Vec<Statement>) {
    // FOR Header:
    // LET VAR001 = [START]
    // :LABEL002
    // IF (!(((1 < 0) + (VAR001 > [END])) & ((1 > 0) + (VAR001 <= [END])))) GOTO LABEL001
    // ...
    // LET VAR001 = VAR001 + [STEP]
    // GOTO LABEL002
    // :LABEL001
    if statements.len() < 4 {
        return;
    }
    for i in 0..statements.len() - 4 {
        if let Statement::Let(outer_let) = &statements[i] {
            let Statement::Label(for_loop_label) = &statements[i + 1] else { continue };
            let for_loop_label = for_loop_label.get_label().clone();
            let if_statement = &statements[i + 2];
            let m: Option<(String, unicase::Ascii<String>, Expression)> = match_for_header(if_statement);

            if let Some((index_variable, for_body_label, to_expr)) = m {
                let Statement::Goto(skip_label_stmt) = &statements[i + 3] else {
                    continue;
                };
                let Statement::Label(body_label_stmt) = &statements[i + 4] else {
                    continue;
                };

                let skip_label = skip_label_stmt.get_label().clone();

                if body_label_stmt.get_label() != &for_body_label {
                    continue;
                }
                let mut matching_goto = -1;
                for j in i + 4..statements.len() - 1 {
                    let Statement::Goto(goto_label) = &statements[j] else {
                        continue;
                    };
                    let Statement::Label(label) = &statements[j + 1] else {
                        continue;
                    };
                    if goto_label.get_label() == &for_loop_label && label.get_label() == &skip_label {
                        matching_goto = j as i32;
                        break;
                    }
                }
                if matching_goto < 0 {
                    continue;
                }

                let step_expr;
                let Statement::Let(inner_let) = &statements[matching_goto as usize - 1] else {
                    continue;
                };
                let Expression::Binary(bin_expr) = inner_let.get_value_expression() else {
                    continue;
                };
                // todo: match expression as string
                if inner_let.get_identifier() != outer_let.get_identifier() {
                    continue;
                }
                if bin_expr.get_op() != BinOp::Add {
                    continue;
                } // always add even if step is negative
                if let Expression::Identifier(lstr) = bin_expr.get_left_expression() {
                    if *lstr.get_identifier() != index_variable {
                        continue;
                    }
                }
                step_expr = bin_expr.get_right_expression().clone();

                let from_expr: Expression = outer_let.get_value_expression().clone();
                let var_name = outer_let.get_identifier().clone();

                let mut for_block: Vec<Statement> = statements.drain(i..matching_goto as usize + 1).collect();
                // pop for header
                for_block.drain(0..5);

                // pop goto and label and step
                let Some(Statement::Goto(continue_label_stmt)) = for_block.pop() else {
                    continue;
                };
                for_block.pop();
                let continue_label = continue_label_stmt.get_label().clone();
                super::optimize_block(visitor, &mut for_block);
                super::handle_break_continue(skip_label, continue_label, &mut for_block);
                if step_expr.to_string() == "1" {
                    statements.insert(i, ForStatement::create_empty_statement(var_name, from_expr, to_expr, None, for_block));
                } else {
                    statements.insert(
                        i,
                        ForStatement::create_empty_statement(var_name, from_expr, to_expr, Some(Box::new(step_expr)), for_block),
                    );
                }
                break;
            }
        }
    }
}

fn match_for_header(
    if_statement: &Statement,
) -> Option<(
    String,                 // indexName
    unicase::Ascii<String>, // for_label
    Expression,             // to_expr
)> {
    match if_statement {
        Statement::If(if_stmt) => {
            if let Expression::Binary(bin_op) = if_stmt.get_condition() {
                if bin_op.get_op() != BinOp::LowerEq && bin_op.get_op() != BinOp::GreaterEq {
                    return None;
                }
                let Expression::Identifier(index_variable) = bin_op.get_left_expression() else {
                    return None;
                };
                let to_expr: Expression = bin_op.get_right_expression().clone();

                let Statement::Goto(for_label) = if_stmt.get_statement() else {
                    return None;
                };

                return Some((index_variable.get_identifier().to_string(), for_label.get_label().clone(), to_expr));
            }
        }
        _ => return None,
    }

    None
}
