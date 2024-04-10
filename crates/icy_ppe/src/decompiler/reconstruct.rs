use crate::ast::RenameVisitor;

use super::{
    constant_scan_visitor::ConstantScanVisitor, rename_visitor::RenameScanVistitor, Ast,
    Expression, Statement,
};

pub fn reconstruct_block(_statements: &[Statement]) {
    // optimize_block(statements);
}

fn _optimize_argument(arg: &mut Expression) {
    if let Expression::Parens(expr) = arg {
        *arg = expr.get_expression().clone();
    }
}

/*
fn optimize_loops(statements: &mut Vec<Statement>) {
    scan_for_next(statements);
    scan_do_while(statements);
}

fn optimize_ifs(statements: &mut Vec<Statement>) {
    scan_if_else(statements);
    scan_if(statements);
}

fn optimize_block(statements: &mut Vec<Statement>) {
    optimize_loops(statements);
    optimize_ifs(statements);
    scan_select_statements(statements);

    strip_unused_labels(statements);
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
}

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

fn scan_if_else(statements: &mut Vec<Statement>) {
    let mut i = 0;
    while i < statements.len() {
        if let Statement::If(if_stmt) = statements[i].clone() {
            if let Statement::Goto(else_label) = if_stmt.get_statement() {
                let else_label_index = get_label_index(
                    statements,
                    i as i32 + 1,
                    statements.len() as i32,
                    else_label.get_label(),
                );
                if else_label_index.is_none() {
                    i += 1;
                    continue;
                }

                let endif_label;
                let endif_label_index = if let Statement::Goto(end_label) =
                    &statements[else_label_index.unwrap() - 1]
                {
                    endif_label = end_label.get_label().clone();
                    get_label_index(
                        statements,
                        else_label_index.unwrap() as i32 + 1,
                        statements.len() as i32,
                        end_label.get_label(),
                    )
                } else {
                    endif_label = unicase::Ascii::new(String::new());
                    None
                };

                if endif_label_index.is_none() {
                    i += 1;
                    continue;
                }

                let endif_label_index = endif_label_index.unwrap();

                let mut elseif_blocks = Vec::new();
                let mut else_block: Vec<Statement>;
                let if_block: Vec<Statement>;

                if let Some(else_label_index) = else_label_index {
                    else_block = statements
                        .drain(else_label_index..endif_label_index)
                        .collect();

                    if_block = statements.drain((i + 1)..(else_label_index - 1)).collect();

                    while let Some(Statement::If(if_stmt)) = get_first(&else_block) {
                        if let Statement::Goto(label) = if_stmt.get_statement() {
                            let label_idx = if *label.get_label() == endif_label {
                                // last elseif case.
                                else_block.len()
                            } else {
                                let label_idx = get_label_index(
                                    &else_block,
                                    1,
                                    else_block.len() as i32,
                                    label.get_label(),
                                );
                                if label_idx.is_none() {
                                    break;
                                }
                                let mut label_idx = label_idx.unwrap();

                                if let Statement::Goto(label2) = &else_block[label_idx - 1] {
                                    if *endif_label != *label2.get_label() {
                                        break;
                                    }
                                    label_idx -= 1;
                                }
                                label_idx
                            };
                            let cond = if_stmt.get_condition().negate_expression();
                            let mut block: Vec<Statement> =
                                else_block.drain(0..label_idx).collect();

                            let a = if let Statement::Label(_) = block[0] {
                                1
                            } else {
                                0
                            };
                            block.remove(a);

                            elseif_blocks.push(ElseIfBlock::empty(cond, block));

                            if !else_block.is_empty() {
                                if let Statement::Goto(label2) = &else_block[0] {
                                    if *endif_label == *label2.get_label() {
                                        else_block.remove(0);
                                    }
                                }
                            }
                        } else {
                            break;
                        }
                    }

                    if !else_block.is_empty() {
                        optimize_loops(&mut else_block);
                        optimize_ifs(&mut else_block);
                    }
                } else {
                    else_block = Vec::new();
                    if_block = statements.drain((i + 1)..(endif_label_index - 1)).collect();
                }

                for block in &mut elseif_blocks {
                    optimize_loops(block.get_statements_mut());
                    optimize_ifs(block.get_statements_mut());
                }

                if if_block.is_empty() {
                    statements[i] = IfThenStatement::create_empty_statement(
                        if_stmt.get_condition().negate_expression(),
                        else_block,
                        elseif_blocks,
                        if if_block.is_empty() {
                            None
                        } else {
                            Some(ElseBlock::empty(if_block))
                        },
                    ); // replace if with if…then
                } else {
                    statements[i] = IfThenStatement::create_empty_statement(
                        if_stmt.get_condition().clone(),
                        if_block,
                        elseif_blocks,
                        if else_block.is_empty() {
                            None
                        } else {
                            Some(ElseBlock::empty(else_block))
                        },
                    ); // replace if with if…then
                }

                statements.remove(i + 1); // remove goto
                                          // do not remove labels they may be needed to analyze other constructs
            }
        }

        i += 1;
    }
}

fn scan_if(statements: &mut Vec<Statement>) {
    let mut i = 0;
    while i < statements.len() {
        if let Statement::If(if_stmt) = statements[i].clone() {
            if let Statement::Goto(endif_label) = if_stmt.get_statement() {
                let endif_label_index = get_label_index(
                    statements,
                    i as i32 + 1,
                    statements.len() as i32,
                    endif_label.get_label(),
                );
                if endif_label_index.is_none() {
                    i += 1;
                    continue;
                }

                let mut statements2 = statements
                    .drain((i + 1)..(endif_label_index.unwrap() as usize))
                    .collect();
                optimize_loops(&mut statements2);
                optimize_ifs(&mut statements2);

                if statements2.len() == 1 {
                    statements[i] = IfStatement::create_empty_statement(
                        if_stmt.get_condition().negate_expression(),
                        statements2.pop().unwrap(),
                    );
                } else {
                    statements[i] = IfThenStatement::create_empty_statement(
                        if_stmt.get_condition().negate_expression(),
                        statements2,
                        Vec::new(),
                        None,
                    );
                }
                // replace if with if…then
                // do not remove labels they may be needed to analyze other constructs
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

    match if_statement {
        Statement::If(if_stmt) => {
            match if_stmt.get_statement() {
                Statement::Goto(goto_stmt) => {
                    // todo: match expression
                    breakout_label = goto_stmt.get_label();
                }
                _ => return None,
            }

            if let Expression::Unary(not_expr) = if_stmt.get_condition() {
                if not_expr.get_op() == crate::ast::UnaryOp::Not {
                    if let Expression::Parens(p_expr) = not_expr.get_expression() {
                        if let Expression::Binary(bin_op) = p_expr.get_expression() {
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
                }
            }
        }
        _ => return None,
    }

    None
}

fn scan_for_next(statements: &mut Vec<Statement>) {
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
                optimize_loops(&mut statements2);
                scan_possible_breaks(&mut statements2, &breakout_label);
                // there needs to be a better way to handle that
                if !statements2.is_empty() {
                    if let Statement::Label(lbl) = statements2.last().unwrap().clone() {
                        scan_possible_continues(&mut statements2, lbl.get_label());
                    }
                }
                optimize_ifs(&mut statements2);

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

fn scan_do_while(statements: &mut Vec<Statement>) {
    scan_do_while2(statements);

    let mut i = 0;
    'main: while i < statements.len() {
        let cur = &statements[i];
        if let Statement::Label(label) = cur {
            i += 1;
            if i >= statements.len() {
                break;
            }
            if let Statement::If(if_stmt) = statements[i].clone() {
                if let Statement::Goto(break_label) = if_stmt.get_statement() {
                    // search matching goto
                    let mut j = i + 1;
                    let mut matching_goto = -1;
                    while j < statements.len() {
                        if let Statement::Goto(next_label) = &statements[j] {
                            if next_label.get_label() == label.get_label() {
                                if j + 1 >= statements.len() {
                                    i += 1;
                                    continue 'main;
                                }
                                if let Statement::Label(next_label) = &statements[j + 1] {
                                    if next_label.get_label() == break_label.get_label() {
                                        matching_goto = j as i32;
                                        break;
                                    }
                                }
                                j += 1;
                                continue;
                            }
                        }
                        j += 1;
                    }
                    if matching_goto < 0 {
                        i += 1;
                        continue;
                    }
                    let label_cp = break_label.get_label();

                    let mut statements2 = statements
                        .drain((i + 1)..(matching_goto as usize))
                        .collect();
                    optimize_loops(&mut statements2);

                    scan_possible_breaks(&mut statements2, label_cp);
                    // there needs to be a better way to handle that
                    if let Statement::Label(lbl) = &statements2.last().unwrap().clone() {
                        scan_possible_continues(&mut statements2, lbl.get_label());
                    }
                    optimize_ifs(&mut statements2);

                    statements[i] = WhileDoStatement::create_empty_statement(
                        if_stmt.get_condition().negate_expression(),
                        statements2,
                    );
                    statements.remove(i + 1);
                    statements.remove(i - 1);

                    i = 0;
                    continue;
                }
            }
        }
        i += 1;
    }
}

fn scan_do_while2(_statements: &mut [Statement]) {
    /*
        let mut i = 0;

        while i < statements.len() {
            let cur = &statements[i];
            if let Statement::Label(while_label) = cur {
                i += 1;
                if i >= statements.len() {
                    break;
                }

                // search matching goto
                let mut j = i + 1;

                let mut matching_goto = 0;
                while j < statements.len() {
                    if let Statement::Goto(check_label) = &statements[j] {
                        if j + 1 >= statements.len() {
                            break;
                        }
                        if check_label == while_label {
                            matching_goto = j;
                            break;
                        }
                    }
                    j += 1;
                }
                if matching_goto <= i {
                    i += 1;
                    continue;
                }
                // test for break label
                if let Statement::If(exp, stmt) = statements[matching_goto - 1].clone() {
                    if let Statement::Goto(break_label) = &*stmt {
                        if let Statement::Label(next_label) = &statements[matching_goto + 1] {
                            // check break label
                            if next_label != break_label {
                                i += 1;
                                continue;
                            }
                            let label_cp = break_label.clone();
                            let mut block: Vec<Statement> =
                                statements.drain((i + 1)..=matching_goto).collect();
                            block.pop(); // pop goto start of while
                            block.pop(); // pop goto start of while
                            optimize_loops(&mut block);
                            scan_possible_breaks(&mut block, label_cp.as_str());
                            // there needs to be a better way to handle that
                            let mut continue_label = String::new();
                            if let Some(Statement::Label(lbl)) = &block.last() {
                                continue_label = lbl.clone();
                            }
                            if !continue_label.is_empty() {
                                scan_possible_continues(&mut block, continue_label.as_str());
                            }
                            optimize_ifs(&mut block);
                            statements[i] = Statement::DoWhile(
                                Box::new(Expression::UnaryExpression(
                                    crate::ast::UnaryOp::Not,
                                    exp.clone(),
                                )),
                                block,
                            );
                            i = 0;
                            continue;
                        }
                    }
                }
            }
            i += 1;
        }

    */
}

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
