use crate::{
    ast::{Constant, Expression, Statement},
    executable::OpCode,
};

/// Flattens the nested blocks that the lowering leaves behind and then tidies up the
/// control flow: a branch whose condition is already decided loses the test, a jump to
/// the statement that follows it is dropped, and so is everything that no jump and no
/// fall through can reach.
///
/// Jumps are deliberately not threaded through labels that only hold another jump. That
/// measured no smaller on either corpus and it takes the shape of a loop apart far enough
/// that the decompiler stops recognising one.
pub fn optimize_statements(statements: &[Statement]) -> Vec<Statement> {
    let mut flat = Vec::new();
    flatten(statements, &mut flat);
    // Removing one jump can leave the one before it pointing at the next statement, so the
    // passes repeat until the stream stops shrinking.
    loop {
        let length = flat.len();
        resolve_decided_conditions(&mut flat);
        remove_jumps_to_the_following_statement(&mut flat);
        remove_unreachable_statements(&mut flat);
        if flat.len() == length {
            return flat;
        }
    }
}

fn flatten(statements: &[Statement], result: &mut Vec<Statement>) {
    for statement in statements {
        if let Statement::Block(block) = statement {
            flatten(block.get_statements(), result);
        } else {
            result.push(statement.clone());
        }
    }
}

/// True for statements that the code generator turns into nothing at all.
fn emits_no_code(statement: &Statement) -> bool {
    matches!(
        statement,
        Statement::Empty | Statement::Comment(_) | Statement::Label(_) | Statement::VariableDeclaration(_) | Statement::ConstDeclaration(_)
    )
}

/// True for statements after which control never simply moves on to the next one.
fn ends_the_flow(statement: &Statement) -> bool {
    match statement {
        Statement::Goto(_) | Statement::Return(_) => true,
        Statement::PredifinedCall(call) => matches!(call.get_func().opcode, OpCode::END | OpCode::STOP | OpCode::RETURN),
        _ => false,
    }
}

/// A conditional jump whose condition folded down to a constant either always jumps or
/// never does, which in turn lets the unreachable pass see the branch it guards.
fn resolve_decided_conditions(statements: &mut [Statement]) {
    for statement in statements.iter_mut() {
        let Statement::If(if_stmt) = statement else {
            continue;
        };
        let Expression::Const(constant) = if_stmt.get_condition() else {
            continue;
        };
        let Constant::Boolean(taken) = constant.get_constant_value() else {
            continue;
        };
        *statement = if *taken { if_stmt.get_statement().clone() } else { Statement::Empty };
    }
}

fn remove_jumps_to_the_following_statement(statements: &mut Vec<Statement>) {
    let mut redundant = vec![false; statements.len()];
    for (index, statement) in statements.iter().enumerate() {
        let Statement::Goto(goto) = statement else {
            continue;
        };
        for next in statements.iter().skip(index + 1) {
            match next {
                Statement::Label(label) if label.get_label() == goto.get_label() => {
                    redundant[index] = true;
                    break;
                }
                _ if emits_no_code(next) => {}
                _ => break,
            }
        }
    }
    let mut index = 0;
    statements.retain(|_| {
        index += 1;
        !redundant[index - 1]
    });
}

fn remove_unreachable_statements(statements: &mut Vec<Statement>) {
    let mut reachable = true;
    statements.retain(|statement| {
        if matches!(statement, Statement::Label(_)) {
            reachable = true;
        }
        // A declaration may sit in a part of the program that is never entered and the
        // variable it introduces is still expected to exist.
        let keep = reachable || matches!(statement, Statement::VariableDeclaration(_) | Statement::ConstDeclaration(_));
        if reachable && ends_the_flow(statement) {
            reachable = false;
        }
        keep
    });
}

#[cfg(test)]
mod tests {
    use super::optimize_statements;
    use crate::{
        ast::{
            Constant, ConstantExpression, GotoStatement, IfStatement, LabelStatement, PredefinedCallStatement, Statement, VariableDeclarationStatement,
            VariableSpecifier,
        },
        executable::{OpCode, VariableType},
    };

    fn label(name: &str) -> unicase::Ascii<String> {
        unicase::Ascii::new(name.to_string())
    }

    fn print(text: &str) -> Statement {
        PredefinedCallStatement::create_empty_statement(
            OpCode::PRINTLN.get_definition(),
            vec![ConstantExpression::create_empty_expression(Constant::String(text.to_string()))],
        )
    }

    fn declare(name: &str) -> Statement {
        Statement::VariableDeclaration(VariableDeclarationStatement::empty(
            VariableType::Integer,
            vec![VariableSpecifier::empty(label(name), Vec::new())],
        ))
    }

    fn is_print(statement: &Statement) -> bool {
        matches!(statement, Statement::PredifinedCall(call) if call.get_func().opcode == OpCode::PRINTLN)
    }

    #[test]
    fn test_statements_between_a_jump_and_the_next_label_are_dropped() {
        let result = optimize_statements(&[
            GotoStatement::create_empty_statement(label("SKIP")),
            print("never"),
            LabelStatement::create_empty_statement(label("SKIP")),
            print("always"),
        ]);
        assert_eq!(result.iter().filter(|s| is_print(s)).count(), 1);
    }

    #[test]
    fn test_a_declaration_survives_where_nothing_reaches_it() {
        let result = optimize_statements(&[
            GotoStatement::create_empty_statement(label("SKIP")),
            declare("I"),
            LabelStatement::create_empty_statement(label("SKIP")),
        ]);
        assert!(result.iter().any(|s| matches!(s, Statement::VariableDeclaration(_))));
    }

    #[test]
    fn test_a_jump_to_the_statement_that_follows_it_is_dropped() {
        let result = optimize_statements(&[
            GotoStatement::create_empty_statement(label("NEXT")),
            LabelStatement::create_empty_statement(label("NEXT")),
            print("always"),
        ]);
        assert!(!result.iter().any(|s| matches!(s, Statement::Goto(_))));
    }

    #[test]
    fn test_a_branch_that_always_jumps_takes_its_body_with_it() {
        let result = optimize_statements(&[
            IfStatement::create_empty_statement(
                ConstantExpression::create_empty_expression(Constant::Boolean(true)),
                GotoStatement::create_empty_statement(label("EXIT")),
            ),
            print("never"),
            LabelStatement::create_empty_statement(label("EXIT")),
        ]);
        assert!(!result.iter().any(|s| is_print(s) || matches!(s, Statement::If(_))));
    }

    #[test]
    fn test_a_branch_that_never_jumps_keeps_its_body() {
        let result = optimize_statements(&[
            IfStatement::create_empty_statement(
                ConstantExpression::create_empty_expression(Constant::Boolean(false)),
                GotoStatement::create_empty_statement(label("EXIT")),
            ),
            print("always"),
            LabelStatement::create_empty_statement(label("EXIT")),
        ]);
        assert!(!result.iter().any(|s| matches!(s, Statement::If(_))));
        assert_eq!(result.iter().filter(|s| is_print(s)).count(), 1);
    }
}
