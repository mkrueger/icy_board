use crate::ast::{BinOp, BreakStatement, ContinueStatement, Expression, ForStatement, IfStatement, Statement};

pub fn scan_for_next(statements: &mut Vec<Statement>) {
    // FOR Header:
    // LET VAR001 = [START]
    // :LABEL002
    // IF (!(((1 < 0) + (VAR001 > [END])) & ((1 > 0) + (VAR001 <= [END])))) GOTO LABEL001
    // ...
    // LET VAR001 = VAR001 + [STEP]
    // GOTO LABEL002
    // :LABEL001
    if statements.len() < 2 {
        return;
    }
    let mut i = 0;
    while i < statements.len() - 2 {
        if let Statement::Let(outer_let) = &statements[i] {
            let label = &statements[i + 1];
            let if_statement = &statements[i + 2];
            let m = mach_for_construct(label, if_statement);
            if let Some((for_label, index_label, breakout_label, to_expr)) = m {
                let mut j = i + 1;
                let mut matching_goto = -1;
                while j < statements.len() {
                    if let Statement::Goto(next_label) = &statements[j] {
                        if *next_label.get_label() == for_label {
                            if j + 1 >= statements.len() {
                                continue;
                            }
                            if let Statement::Label(next_label) = &statements[j + 1] {
                                if *next_label.get_label() == breakout_label {
                                    matching_goto = j as i32;
                                    break;
                                }
                            }
                            i += 1;
                            continue;
                        }
                    }
                    j += 1;
                }
                if matching_goto < 0 {
                    i += 1;
                    continue;
                }

                let step_expr;
                if let Statement::Let(inner_let) = &statements[matching_goto as usize - 1] {
                    // todo: match expression as string
                    if inner_let.get_identifier() != outer_let.get_identifier() {
                        i += 1;
                        continue;
                    }

                    if let Expression::Binary(bin_expr) = inner_let.get_value_expression() {
                        if bin_expr.get_op() != BinOp::Add {
                            continue;
                        } // always add even if step is negative
                        if let Expression::Identifier(lstr) = bin_expr.get_left_expression() {
                            if *lstr.get_identifier() != index_label {
                                i += 1;
                                continue;
                            }
                        }
                        step_expr = bin_expr.get_right_expression().clone();
                    } else {
                        i += 1;
                        continue;
                    }
                } else {
                    i += 1;
                    continue;
                }

                let from_expr = outer_let.get_value_expression().clone();
                let var_name = outer_let.get_identifier().clone();

                statements.remove((matching_goto - 1) as usize); // remove LET
                statements.remove((matching_goto - 1) as usize); // remove matching goto
                statements.remove(i + 1); // remove for_label
                statements.remove(i + 1); // remove if

                let mut statements2 = statements
                    .drain((i + 1)..(matching_goto as usize - 3))
                    .collect();
                super::optimize_loops(&mut statements2);
                scan_possible_breaks(&mut statements2, &breakout_label);
                // there needs to be a better way to handle that
                if !statements2.is_empty() {
                    if let Statement::Label(lbl) = statements2.last().unwrap().clone() {
                        scan_possible_continues(&mut statements2, lbl.get_label());
                    }
                }
                // super::optimize_ifs(&mut statements2);

                if step_expr.to_string() == "1" {
                    statements[i] = ForStatement::create_empty_statement(
                        var_name,
                        from_expr,
                        to_expr,
                        None,
                        statements2,
                    );
                } else {
                    statements[i] = ForStatement::create_empty_statement(
                        var_name,
                        from_expr,
                        to_expr,
                        Some(Box::new(step_expr)),
                        statements2,
                    );
                }
                continue;
            }
        }
        i += 1;
    }
}

fn mach_for_construct(
    label: &Statement,
    if_statement: &Statement,
) -> Option<(
    unicase::Ascii<String>,
    String,
    unicase::Ascii<String>,
    Expression,
)> // for_label, indexName, breakout_label, to_expr
{
    let breakout_label;
    let Statement::Label(for_label) = label else {
        return None;
    };
    println!("label : {:?}", for_label);
    match if_statement {
        Statement::If(if_stmt) => {
            match if_stmt.get_statement() {
                Statement::Goto(goto_stmt) => {
                    // todo: match expression
                    breakout_label = goto_stmt.get_label();
                }
                _ => return None,
            }
            println!("condition : {}", if_stmt.get_condition());

            if let Expression::Binary(bin_op) = if_stmt.get_condition() {
                    // TODO: Check _op
                    if let Expression::Parens(p_expr_l) = bin_op.get_left_expression() {
                        if let Expression::Parens(p_expr_r) = bin_op.get_right_expression()
                        {
                            if let Expression::Binary(lbin_op) = p_expr_l.get_expression() {
                                if let Expression::Binary(rbin_op) =
                                    p_expr_r.get_expression()
                                {
                                    // TODO: Check _op

                                    if let Expression::Parens(left_binop) =
                                        lbin_op.get_right_expression()
                                    {
                                        if let Expression::Parens(right_binop) =
                                            rbin_op.get_right_expression()
                                        {
                                            if let Expression::Binary(bin_op1) =
                                                left_binop.get_expression()
                                            {
                                                if let Expression::Binary(bin_op2) =
                                                    right_binop.get_expression()
                                                {
                                                    // TODO: Check _op
                                                    if bin_op1.get_left_expression() != bin_op2.get_left_expression()/*|| *opl != BinOp::Greater || *opr != BinOp::LowerEq */|| bin_op1.get_right_expression() != bin_op2.get_right_expression()
                                                    {
                                                        return None;
                                                    }
                                                    return Some((
                                                        for_label.get_label().clone(),
                                                        bin_op1
                                                            .get_left_expression()
                                                            .to_string(),
                                                        breakout_label.clone(),
                                                        bin_op1
                                                            .get_right_expression()
                                                            .clone(),
                                                    ));
                                                }
                                            }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
        }
        _ => return None,
    }

    None
}


fn scan_possible_breaks(block: &mut [Statement], break_label: &unicase::Ascii<String>) {
    for cur_stmt in block {
        match cur_stmt {
            Statement::If(if_stmt) => {
                if let Statement::Goto(label) = if_stmt.get_statement() {
                    if label.get_label() == break_label {
                        *cur_stmt = IfStatement::create_empty_statement(
                            if_stmt.get_condition().clone(),
                            BreakStatement::create_empty_statement(),
                        );
                    }
                }
            }
            Statement::Goto(label) => {
                if label.get_label() == break_label {
                    *cur_stmt = BreakStatement::create_empty_statement();
                }
            }
            _ => {}
        }
    }
}

#[allow(clippy::needless_range_loop)]
fn scan_possible_continues(block: &mut [Statement], continue_label: &unicase::Ascii<String>) {
    for cur_stmt in block {
        match cur_stmt {
            Statement::If(if_stmt) => {
                if let Statement::Goto(label) = if_stmt.get_statement() {
                    if label.get_label() == continue_label {
                        *cur_stmt = IfStatement::create_empty_statement(
                            if_stmt.get_condition().clone(),
                            ContinueStatement::create_empty_statement(),
                        );
                    }
                }
            }
            Statement::Goto(label) => {
                if *label.get_label() == continue_label {
                    *cur_stmt = ContinueStatement::create_empty_statement();
                }
            }
            _ => {}
        }
    }
}
