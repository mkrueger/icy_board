use crate::{
    ast::{Constant, Expression, OnErrorMode, Statement},
    executable::OpCode,
};
use std::collections::HashMap;

pub(crate) fn constant_boolean(expression: &Expression) -> Option<bool> {
    match expression {
        Expression::Const(constant) => match constant.get_constant_value() {
            Constant::Boolean(value) => Some(*value),
            _ => None,
        },
        _ => None,
    }
}

/// Flattens lowered statements, threads unconditional jump chains, and removes blocks
/// unreachable from the routine entry or an installed error handler.
pub fn optimize_statements(statements: &[Statement]) -> Vec<Statement> {
    let mut flat = Vec::new();
    flatten(statements, &mut flat);
    resolve_decided_conditions(&mut flat);
    thread_jump_chains(&mut flat);
    remove_jumps_to_the_following_statement(&mut flat);
    ControlFlowGraph::build(&flat).retain_reachable(&mut flat);
    remove_jumps_to_the_following_statement(&mut flat);
    flat
}

pub(crate) fn statement_reachability(statements: &[Statement]) -> Vec<(&Statement, bool)> {
    let mut original = Vec::new();
    flatten_references(statements, &mut original);
    let cfg = ControlFlowGraph::build_references(&original);
    original
        .into_iter()
        .enumerate()
        .map(|(index, statement)| (statement, cfg.statement_is_reachable(index)))
        .collect()
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

fn flatten_references<'a>(statements: &'a [Statement], result: &mut Vec<&'a Statement>) {
    for statement in statements {
        if let Statement::Block(block) = statement {
            flatten_references(block.get_statements(), result);
        } else {
            result.push(statement);
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
        let Some(taken) = constant_boolean(if_stmt.get_condition()) else {
            continue;
        };
        *statement = if taken { if_stmt.get_statement().clone() } else { Statement::Empty };
    }
}

fn remove_jumps_to_the_following_statement(statements: &mut Vec<Statement>) {
    let mut preceding_code = vec![None; statements.len()];
    let mut previous = None;
    for (index, statement) in statements.iter().enumerate() {
        preceding_code[index] = previous;
        if !emits_no_code(statement) {
            previous = Some(index);
        }
    }
    let labels: HashMap<_, _> = statements
        .iter()
        .enumerate()
        .filter_map(|(index, statement)| match statement {
            Statement::Label(label) => Some((label.get_label(), index)),
            _ => None,
        })
        .collect();
    let redundant: Vec<_> = statements
        .iter()
        .enumerate()
        .map(|(index, statement)| match statement {
            Statement::Goto(goto) => labels.get(goto.get_label()).is_some_and(|target| preceding_code[*target] == Some(index)),
            _ => false,
        })
        .collect();
    let mut index = 0;
    statements.retain(|_| {
        index += 1;
        !redundant[index - 1]
    });
}

fn thread_jump_chains(statements: &mut [Statement]) {
    let labels: HashMap<_, _> = statements
        .iter()
        .enumerate()
        .filter_map(|(index, statement)| match statement {
            Statement::Label(label) => Some((label.get_label(), index)),
            _ => None,
        })
        .collect();
    let mut next_code = vec![None; statements.len()];
    let mut next = None;
    for index in (0..statements.len()).rev() {
        next_code[index] = next;
        if !emits_no_code(&statements[index]) {
            next = Some(index);
        }
    }
    let mut direct_targets = vec![None; statements.len()];
    for index in labels.values().copied() {
        direct_targets[index] = next_code[index].and_then(|next| match &statements[next] {
            Statement::Goto(goto) => labels.get(goto.get_label()).copied(),
            _ => None,
        });
    }
    let mut resolved_targets = vec![None; statements.len()];
    let mut resolving = vec![false; statements.len()];
    for label in labels.values().copied() {
        resolve_jump_target(label, &direct_targets, &mut resolved_targets, &mut resolving);
    }
    let replacements: Vec<_> = statements
        .iter()
        .enumerate()
        .filter_map(|(index, statement)| {
            let Statement::Goto(goto) = statement else { return None };
            let target = labels.get(goto.get_label()).and_then(|target| resolved_targets[*target])?;
            let Statement::Label(label) = &statements[target] else { return None };
            Some((index, label.get_label().clone()))
        })
        .collect();
    for (index, target) in replacements {
        if let Statement::Goto(goto) = &mut statements[index] {
            goto.set_label(target);
        }
    }
}

fn resolve_jump_target(start: usize, direct_targets: &[Option<usize>], resolved_targets: &mut [Option<usize>], resolving: &mut [bool]) -> Option<usize> {
    if resolved_targets[start].is_some() {
        return resolved_targets[start];
    }
    let mut path = Vec::new();
    let mut target = start;
    while let Some(next) = direct_targets[target] {
        if let Some(resolved) = resolved_targets[target] {
            target = resolved;
            break;
        }
        if resolving[target] {
            for label in path {
                resolving[label] = false;
            }
            return None;
        }
        resolving[target] = true;
        path.push(target);
        target = next;
    }
    for label in path {
        resolving[label] = false;
        resolved_targets[label] = Some(target);
    }
    resolved_targets[start]
}

#[derive(Debug)]
struct BasicBlock {
    range: core::ops::Range<usize>,
    successors: Vec<usize>,
    predecessors: Vec<usize>,
    reachable: bool,
}

#[derive(Debug)]
struct ControlFlowGraph {
    blocks: Vec<BasicBlock>,
    statement_blocks: Vec<usize>,
}

impl ControlFlowGraph {
    fn build(statements: &[Statement]) -> Self {
        Self::build_references(&statements.iter().collect::<Vec<_>>())
    }

    fn build_references(statements: &[&Statement]) -> Self {
        if statements.is_empty() {
            return Self {
                blocks: Vec::new(),
                statement_blocks: Vec::new(),
            };
        }

        let mut leaders = vec![0];
        for (index, statement) in statements.iter().copied().enumerate() {
            if matches!(statement, Statement::Label(_)) {
                leaders.push(index);
            }
            if (ends_the_flow(statement) || matches!(statement, Statement::If(_) | Statement::Gosub(_))) && index + 1 < statements.len() {
                leaders.push(index + 1);
            }
        }
        leaders.sort_unstable();
        leaders.dedup();

        let mut blocks: Vec<_> = leaders
            .iter()
            .enumerate()
            .map(|(index, start)| BasicBlock {
                range: *start..leaders.get(index + 1).copied().unwrap_or(statements.len()),
                successors: Vec::new(),
                predecessors: Vec::new(),
                reachable: false,
            })
            .collect();
        let mut statement_blocks = vec![0; statements.len()];
        for (block, basic_block) in blocks.iter().enumerate() {
            statement_blocks[basic_block.range.clone()].fill(block);
        }
        let labels: HashMap<_, _> = statements
            .iter()
            .copied()
            .enumerate()
            .filter_map(|(index, statement)| match statement {
                Statement::Label(label) => Some((label.get_label(), statement_blocks[index])),
                _ => None,
            })
            .collect();

        for block in 0..blocks.len() {
            let range = blocks[block].range.clone();
            for statement in statements[range.clone()].iter().copied() {
                if let Statement::OnError(on_error) = statement
                    && matches!(on_error.get_mode(), OnErrorMode::Goto | OnErrorMode::Gosub)
                    && let Some(target) = on_error.get_target().and_then(|target| labels.get(target))
                {
                    blocks[block].successors.push(*target);
                }
            }
            let next = (block + 1 < blocks.len()).then_some(block + 1);
            match statements.get(range.end.saturating_sub(1)).copied() {
                Some(Statement::Goto(goto)) => add_target(&mut blocks[block].successors, &labels, goto.get_label()),
                Some(Statement::Gosub(gosub)) => {
                    add_target(&mut blocks[block].successors, &labels, gosub.get_label());
                    blocks[block].successors.extend(next);
                }
                Some(Statement::If(if_statement)) => {
                    let branch = match constant_boolean(if_statement.get_condition()) {
                        Some(true) => (true, false),
                        Some(false) => (false, true),
                        None => (true, true),
                    };
                    if branch.0
                        && let Statement::Goto(goto) = if_statement.get_statement()
                    {
                        add_target(&mut blocks[block].successors, &labels, goto.get_label());
                    }
                    if branch.1 {
                        blocks[block].successors.extend(next);
                    }
                }
                Some(statement) if ends_the_flow(statement) => {}
                Some(_) => blocks[block].successors.extend(next),
                None => {}
            }
            blocks[block].successors.sort_unstable();
            blocks[block].successors.dedup();
        }
        for source in 0..blocks.len() {
            for target in blocks[source].successors.clone() {
                blocks[target].predecessors.push(source);
            }
        }
        let mut pending = vec![0];
        while let Some(block) = pending.pop() {
            if blocks[block].reachable {
                continue;
            }
            blocks[block].reachable = true;
            pending.extend(blocks[block].successors.iter().copied());
        }
        Self { blocks, statement_blocks }
    }

    fn retain_reachable(&self, statements: &mut Vec<Statement>) {
        let mut keep = vec![false; statements.len()];
        for block in &self.blocks {
            if block.reachable {
                keep[block.range.clone()].fill(true);
            }
        }
        for (index, statement) in statements.iter().enumerate() {
            if matches!(statement, Statement::VariableDeclaration(_) | Statement::ConstDeclaration(_)) {
                keep[index] = true;
            }
        }
        let mut index = 0;
        statements.retain(|_| {
            let retain = keep[index];
            index += 1;
            retain
        });
    }

    fn statement_is_reachable(&self, statement: usize) -> bool {
        self.statement_blocks
            .get(statement)
            .is_some_and(|block| self.blocks[*block].reachable)
    }
}

fn add_target(successors: &mut Vec<usize>, labels: &HashMap<&unicase::Ascii<String>, usize>, target: &unicase::Ascii<String>) {
    if let Some(block) = labels.get(target) {
        successors.push(*block);
    }
}

#[cfg(test)]
mod tests {
    use super::optimize_statements;
    use crate::{
        ast::{
            Constant, ConstantExpression, GosubStatement, GotoStatement, IfStatement, LabelStatement, OnErrorMode, OnErrorStatement, PredefinedCallStatement,
            Statement, VariableDeclarationStatement, VariableSpecifier,
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

    #[test]
    fn cfg_records_conditional_predecessors_and_successors() {
        let statements = vec![
            IfStatement::create_empty_statement(
                ConstantExpression::create_empty_expression(Constant::Integer(1, crate::ast::constant::NumberFormat::Default)),
                GotoStatement::create_empty_statement(label("TARGET")),
            ),
            print("fallthrough"),
            LabelStatement::create_empty_statement(label("TARGET")),
            print("target"),
        ];
        let cfg = super::ControlFlowGraph::build(&statements);

        assert_eq!(vec![1, 2], cfg.blocks[0].successors);
        assert_eq!(vec![0], cfg.blocks[1].predecessors);
        assert_eq!(vec![0, 1], cfg.blocks[2].predecessors);
    }

    #[test]
    fn cfg_keeps_both_a_gosub_target_and_its_continuation() {
        let statements = vec![
            GosubStatement::create_empty_statement(label("WORKER")),
            GotoStatement::create_empty_statement(label("DONE")),
            LabelStatement::create_empty_statement(label("WORKER")),
            Statement::Return(crate::ast::ReturnStatement::empty(None)),
            LabelStatement::create_empty_statement(label("DONE")),
            print("done"),
        ];
        let cfg = super::ControlFlowGraph::build(&statements);

        assert_eq!(vec![1, 2], cfg.blocks[0].successors);
        assert!(cfg.blocks.iter().all(|block| block.reachable));
    }

    #[test]
    fn cfg_keeps_an_installed_on_error_handler() {
        let statements = vec![
            Statement::OnError(OnErrorStatement::empty(OnErrorMode::Goto, label("FAILED"))),
            GotoStatement::create_empty_statement(label("DONE")),
            LabelStatement::create_empty_statement(label("FAILED")),
            Statement::Return(crate::ast::ReturnStatement::empty(None)),
            LabelStatement::create_empty_statement(label("DONE")),
            print("done"),
        ];
        let cfg = super::ControlFlowGraph::build(&statements);

        assert_eq!(vec![1, 2], cfg.blocks[0].successors);
        assert!(cfg.blocks.iter().all(|block| block.reachable));
    }

    #[test]
    fn an_unreferenced_label_does_not_revive_dead_code() {
        let result = optimize_statements(&[
            GotoStatement::create_empty_statement(label("LIVE")),
            LabelStatement::create_empty_statement(label("DEAD")),
            print("dead"),
            LabelStatement::create_empty_statement(label("LIVE")),
            print("live"),
        ]);

        assert_eq!(1, result.iter().filter(|statement| is_print(statement)).count());
        assert!(
            !result
                .iter()
                .any(|statement| matches!(statement, Statement::Label(label) if label.get_label().as_ref() == "DEAD"))
        );
    }

    #[test]
    fn jumps_are_threaded_but_gosubs_are_not() {
        let result = optimize_statements(&[
            GosubStatement::create_empty_statement(label("CALL")),
            GotoStatement::create_empty_statement(label("FIRST")),
            LabelStatement::create_empty_statement(label("CALL")),
            GosubStatement::create_empty_statement(label("FIRST")),
            Statement::Return(crate::ast::ReturnStatement::empty(None)),
            LabelStatement::create_empty_statement(label("FIRST")),
            GotoStatement::create_empty_statement(label("LAST")),
            LabelStatement::create_empty_statement(label("LAST")),
            print("last"),
        ]);

        assert!(
            result
                .iter()
                .any(|statement| matches!(statement, Statement::Goto(goto) if goto.get_label().as_ref() == "LAST"))
        );
        assert!(
            result
                .iter()
                .any(|statement| matches!(statement, Statement::Gosub(gosub) if gosub.get_label().as_ref() == "FIRST"))
        );
    }

    #[test]
    fn a_long_jump_chain_is_resolved_without_recursion() {
        let mut statements = Vec::new();
        for index in 0..1_000 {
            statements.push(LabelStatement::create_empty_statement(label(&format!("L{index}"))));
            statements.push(GotoStatement::create_empty_statement(label(&format!("L{}", index + 1))));
        }
        statements.push(LabelStatement::create_empty_statement(label("L1000")));
        statements.push(print("done"));

        let mut threaded = statements.clone();
        super::thread_jump_chains(&mut threaded);
        assert!(matches!(&threaded[1], Statement::Goto(goto) if goto.get_label().as_ref() == "L1000"));

        let result = optimize_statements(&statements);

        assert!(!result.iter().any(|statement| matches!(statement, Statement::Goto(_))));
        assert_eq!(1, result.iter().filter(|statement| is_print(statement)).count());
    }
}
