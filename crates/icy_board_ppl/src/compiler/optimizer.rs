use crate::{
    ast::{Constant, Expression, OnErrorMode, Statement},
    executable::OpCode,
};
use std::collections::HashMap;

pub(crate) fn constant_boolean(expression: &Expression) -> Option<bool> {
    match expression {
        Expression::Const(constant) => match constant.get_constant_value() {
            Constant::Boolean(value) => Some(*value),
            // The parser represents TRUE/FALSE as named builtins, which the
            // decompiler's constant folder deliberately leaves intact.
            Constant::Builtin(value) if matches!(value.name, "TRUE" | "FALSE") => Some(value.value != 0),
            _ => None,
        },
        Expression::Parens(value) => constant_boolean(value.get_expression()),
        Expression::Unary(value) if value.get_op() == crate::ast::UnaryOp::Not => constant_boolean(value.get_expression()).map(|value| !value),
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

#[cfg(test)]
fn statement_reachability(statements: &[Statement]) -> Vec<(&Statement, bool)> {
    let mut original = Vec::new();
    flatten_references(statements, &mut original);
    let reachability = SourceReachability::build(statements);
    original
        .into_iter()
        .map(|statement| (statement, reachability.statement_is_reachable(statement).unwrap()))
        .collect()
}

/// Routine-wide source control flow, without rewriting the AST or visiting semantics.
///
/// Build once for the complete main/routine body, then query while doing the ordinary
/// semantic walk (including dead code). Nested sequences must use this same result:
/// labels, GOSUB and ON ERROR can cross structured boundaries. A child's flag is NOT
/// ANDed with its parent's entry flag: a jump can enter a dead parent's body directly.
/// Only the enclosing routine's liveness scope should be combined with these flags.
///
/// Expression flags cover separately executed control expressions: IF/ELSEIF/WHILE
/// tests, REPEAT's trailing test, FOR bounds/step and SELECT cases. Other expressions
/// inherit their statement's flag. `None` means the node was not registered, not dead.
/// In particular, synthetic semantic-check expressions should inherit the source
/// expression's scope, rather than looking up their own address.
///
/// Keys are opaque node addresses, never dereferenced. Keep the source AST unchanged
/// and alive during use; discard this result before cloning, moving or lowering it.
#[derive(Debug, Default)]
pub(crate) struct SourceReachability {
    statements: HashMap<usize, bool>,
    expressions: HashMap<usize, bool>,
    falls_through: bool,
}

impl SourceReachability {
    pub(crate) fn build(statements: &[Statement]) -> Self {
        let mut graph = SourceControlFlowGraph::default();
        let exit = graph.node();
        let entry = graph.sequence(statements, exit, None);
        for (source, label) in &graph.jumps {
            if let Some(target) = graph.labels.get(label) {
                graph.edges[*source].push(*target);
            }
        }
        let mut live = vec![false; graph.edges.len()];
        let mut pending = vec![entry];
        while let Some(node) = pending.pop() {
            if !live[node] {
                live[node] = true;
                pending.extend(graph.edges[node].iter().copied());
            }
        }
        Self {
            statements: graph.statements.into_iter().map(|(key, node)| (key, live[node])).collect(),
            expressions: graph
                .expressions
                .into_iter()
                .map(|(key, nodes)| (key, nodes.into_iter().any(|node| live[node])))
                .collect(),
            falls_through: live[exit],
        }
    }

    pub(crate) fn statement_is_reachable(&self, statement: &Statement) -> Option<bool> {
        self.statements.get(&(std::ptr::from_ref(statement) as usize)).copied()
    }

    pub(crate) fn expression_is_reachable(&self, expression: &Expression) -> Option<bool> {
        self.expressions.get(&(std::ptr::from_ref(expression) as usize)).copied()
    }

    /// Whether control can reach the end of this sequence. This is not a proof
    /// that a function result was assigned on every returning path.
    pub(crate) fn falls_through(&self) -> bool {
        self.falls_through
    }
}

#[derive(Clone, Copy)]
struct LoopTargets {
    continue_to: usize,
    break_to: usize,
}

#[derive(Default)]
struct SourceControlFlowGraph {
    edges: Vec<Vec<usize>>,
    statements: HashMap<usize, usize>,
    expressions: HashMap<usize, Vec<usize>>,
    labels: HashMap<unicase::Ascii<String>, usize>,
    jumps: Vec<(usize, unicase::Ascii<String>)>,
}

impl SourceControlFlowGraph {
    fn node(&mut self) -> usize {
        let node = self.edges.len();
        self.edges.push(Vec::new());
        node
    }

    fn expression(&mut self, expression: &Expression, node: usize) {
        self.expressions.entry(std::ptr::from_ref(expression) as usize).or_default().push(node);
    }

    fn branch(&mut self, node: usize, condition: Option<bool>, taken: usize, not_taken: usize) {
        if condition != Some(false) {
            self.edges[node].push(taken);
        }
        if condition != Some(true) {
            self.edges[node].push(not_taken);
        }
    }

    fn condition(&mut self, expression: &Expression, taken: usize, not_taken: usize, inverted_by_lowering: bool) -> usize {
        let node = self.node();
        self.expression(expression, node);
        self.branch(node, source_boolean(expression, inverted_by_lowering), taken, not_taken);
        node
    }

    fn sequence(&mut self, statements: &[Statement], next: usize, loop_targets: Option<LoopTargets>) -> usize {
        statements
            .iter()
            .rev()
            .fold(next, |next, statement| self.statement(statement, next, loop_targets))
    }

    fn statement(&mut self, statement: &Statement, next: usize, loop_targets: Option<LoopTargets>) -> usize {
        let entry = self.node();
        self.statements.insert(std::ptr::from_ref(statement) as usize, entry);
        match statement {
            Statement::Block(block) => {
                let body = self.sequence(block.get_statements(), next, loop_targets);
                self.edges[entry].push(body);
            }
            Statement::If(value) => {
                let body = self.statement(value.get_statement(), next, loop_targets);
                self.expression(value.get_condition(), entry);
                self.branch(
                    entry,
                    source_boolean(value.get_condition(), !matches!(value.get_statement(), Statement::Goto(_))),
                    body,
                    next,
                );
            }
            Statement::IfThen(value) => {
                let mut alternative = value
                    .get_else_block()
                    .as_ref()
                    .map_or(next, |block| self.sequence(block.get_statements(), next, loop_targets));
                for block in value.get_else_if_blocks().iter().rev() {
                    let body = self.sequence(block.get_statements(), next, loop_targets);
                    alternative = self.condition(block.get_condition(), body, alternative, true);
                }
                let body = self.sequence(value.get_statements(), next, loop_targets);
                self.expression(value.get_condition(), entry);
                self.branch(entry, source_boolean(value.get_condition(), true), body, alternative);
            }
            Statement::While(value) => {
                let targets = Some(LoopTargets {
                    continue_to: entry,
                    break_to: next,
                });
                let body = self.statement(value.get_statement(), entry, targets);
                self.expression(value.get_condition(), entry);
                self.branch(entry, source_boolean(value.get_condition(), true), body, next);
            }
            Statement::WhileDo(value) => {
                let targets = Some(LoopTargets {
                    continue_to: entry,
                    break_to: next,
                });
                let body = self.sequence(value.get_statements(), entry, targets);
                self.expression(value.get_condition(), entry);
                self.branch(entry, source_boolean(value.get_condition(), true), body, next);
            }
            Statement::RepeatUntil(value) => {
                let test = self.node();
                let targets = Some(LoopTargets {
                    continue_to: test,
                    break_to: next,
                });
                let body = self.sequence(value.get_statements(), test, targets);
                self.edges[entry].push(body);
                self.expression(value.get_condition(), test);
                self.branch(test, source_boolean(value.get_condition(), true), next, body);
            }
            Statement::Loop(value) => {
                let targets = Some(LoopTargets {
                    continue_to: entry,
                    break_to: next,
                });
                let body = self.sequence(value.get_statements(), entry, targets);
                self.edges[entry].push(body);
            }
            Statement::For(value) => {
                // Lowering does not propagate the initializer into the counter's
                // comparisons: even constant bounds retain both body and exit.
                let test = self.node();
                let increment = self.node();
                let targets = Some(LoopTargets {
                    continue_to: increment,
                    break_to: next,
                });
                let body = self.sequence(value.get_statements(), increment, targets);
                self.expression(value.get_start_expr(), entry);
                self.expression(value.get_end_expr(), test);
                if let Some(step) = value.get_step_expr() {
                    self.expression(step, test);
                    self.expression(step, increment);
                }
                self.edges[entry].push(test);
                self.branch(test, None, body, next);
                self.edges[increment].push(test);
            }
            Statement::ForEach(value) => {
                let test = self.node();
                let targets = Some(LoopTargets {
                    continue_to: test,
                    break_to: next,
                });
                let body = self.sequence(value.get_statements(), test, targets);
                self.expression(value.get_collection(), entry);
                self.edges[entry].push(test);
                // A collection may be empty; its expression is evaluated only
                // on entry, not each time CONTINUE advances the iterator.
                self.branch(test, None, body, next);
            }
            Statement::Select(value) => {
                let mut alternative = self.sequence(value.get_default_statements(), next, loop_targets);
                // A selector with no CASE is validated but never emitted by
                // lowering. Register an explicitly dead evaluation for that case.
                self.expressions.entry(std::ptr::from_ref(value.get_expression()) as usize).or_default();
                for block in value.get_case_blocks().iter().rev() {
                    let body = self.sequence(block.get_statements(), next, loop_targets);
                    let test = self.node();
                    for specifier in block.get_case_specifiers() {
                        self.expression(value.get_expression(), test);
                        match specifier {
                            crate::ast::CaseSpecifier::Expression(expression) => self.expression(expression, test),
                            crate::ast::CaseSpecifier::FromTo(from, to) => {
                                self.expression(from, test);
                                self.expression(to, test);
                            }
                        }
                    }
                    // SELECT's generated comparisons are not constant-folded
                    // by current lowering. Preserve that conservative policy.
                    let unconditional = block.get_case_specifiers().is_empty().then_some(true);
                    self.branch(test, unconditional, body, alternative);
                    alternative = test;
                }
                self.edges[entry].push(alternative);
            }
            Statement::Break(_) => self.edges[entry].push(loop_targets.map_or(next, |targets| targets.break_to)),
            Statement::Continue(_) => self.edges[entry].push(loop_targets.map_or(next, |targets| targets.continue_to)),
            Statement::Goto(value) => self.jumps.push((entry, value.get_label().clone())),
            Statement::Gosub(value) => {
                self.jumps.push((entry, value.get_label().clone()));
                // As in the lowered CFG, retain the return continuation even
                // when the subroutine's own termination cannot be established.
                self.edges[entry].push(next);
            }
            Statement::OnError(value) => {
                if matches!(value.get_mode(), OnErrorMode::Goto | OnErrorMode::Gosub)
                    && let Some(target) = value.get_target()
                {
                    self.jumps.push((entry, target.clone()));
                }
                self.edges[entry].push(next);
            }
            Statement::Label(value) => {
                self.labels.insert(value.get_label().clone(), entry);
                self.edges[entry].push(next);
            }
            Statement::Return(_) => {}
            Statement::PredifinedCall(_) if ends_the_flow(statement) => {}
            Statement::Empty
            | Statement::Comment(_)
            | Statement::Let(_)
            | Statement::Call(_)
            | Statement::PredifinedCall(_)
            | Statement::MemberCall(_)
            | Statement::VariableDeclaration(_)
            | Statement::ConstDeclaration(_) => self.edges[entry].push(next),
        }
        entry
    }
}

/// Use the same pure constant folder and implicit negation as structural lowering.
/// Do not use EvaluationVisitor's partial evaluation: FALSE AND a call must still
/// evaluate that call. In particular, no name resolution or semantic pass runs here.
fn source_boolean(expression: &Expression, inverted_by_lowering: bool) -> Option<bool> {
    let condition = if inverted_by_lowering {
        expression.negate_expression()
    } else {
        expression.clone()
    };
    let folded = condition.visit_mut(&mut crate::decompiler::evaluation_visitor::ConstantFolder::default());
    constant_boolean(&folded).map(|value| if inverted_by_lowering { !value } else { value })
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

#[cfg(test)]
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
}

impl ControlFlowGraph {
    fn build(statements: &[Statement]) -> Self {
        Self::build_references(&statements.iter().collect::<Vec<_>>())
    }

    fn build_references(statements: &[&Statement]) -> Self {
        if statements.is_empty() {
            return Self { blocks: Vec::new() };
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
        Self { blocks }
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

#[cfg(test)]
mod source_reachability_tests {
    use super::{SourceReachability, statement_reachability};
    use crate::{
        ast::{
            BinOp, BinaryExpression, BlockStatement, BreakStatement, CaseBlock, CaseSpecifier, Constant, ConstantExpression, ContinueStatement, ElseBlock,
            ElseIfBlock, Expression, ForEachStatement, ForStatement, FunctionCallExpression, GosubStatement, GotoStatement, IdentifierExpression, IfStatement,
            IfThenStatement, LabelStatement, LoopStatement, OnErrorMode, OnErrorStatement, PredefinedCallStatement, RepeatUntilStatement, ReturnStatement,
            SelectStatement, Statement, WhileDoStatement, WhileStatement, constant::NumberFormat,
        },
        executable::OpCode,
    };

    fn name(value: &str) -> unicase::Ascii<String> {
        unicase::Ascii::new(value.to_string())
    }

    fn boolean(value: bool) -> Expression {
        ConstantExpression::create_empty_expression(Constant::Boolean(value))
    }

    fn integer(value: i32) -> Expression {
        ConstantExpression::create_empty_expression(Constant::Integer(value, NumberFormat::Default))
    }

    fn call() -> Expression {
        FunctionCallExpression::create_empty_expression(IdentifierExpression::create_empty_expression(name("Probe")), Vec::new())
    }

    fn work() -> Statement {
        PredefinedCallStatement::create_empty_statement(OpCode::PRINTLN.get_definition(), vec![call()])
    }

    fn returns() -> Statement {
        ReturnStatement::create_empty_statement(None)
    }

    fn flags(reachability: &SourceReachability, statements: &[Statement], expected: &[bool]) {
        assert_eq!(statements.len(), expected.len());
        for (statement, expected) in statements.iter().zip(expected) {
            assert_eq!(Some(*expected), reachability.statement_is_reachable(statement), "{statement:?}");
        }
    }

    fn with_parsed_body(source: &str, check: impl FnOnce(&[Statement])) {
        use crate::{
            ast::AstNode,
            compiler::workspace::Workspace,
            parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
        };
        let mut workspace = Workspace::default();
        workspace.set_default_language_version(Some(400));
        let errors = std::sync::Arc::new(std::sync::Mutex::new(ErrorReporter::default()));
        let ast = parse_ast(
            std::path::PathBuf::from("source-cfg.pps"),
            errors.clone(),
            source,
            &UserTypeRegistry::icy_board_registry(),
            Encoding::Utf8,
            &workspace,
        );
        assert!(
            errors.lock().unwrap().errors.is_empty(),
            "{source}: {:?}",
            errors.lock().unwrap().errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>()
        );
        let original = ast.nodes.clone();
        let body = ast
            .nodes
            .iter()
            .find_map(|node| match node {
                AstNode::Main(body) => Some(body.get_statements()),
                _ => None,
            })
            .expect("main body");
        check(body);
        assert_eq!(original, ast.nodes, "CFG queries must not rewrite source nodes");
    }

    #[test]
    fn parsed_booleans_and_wrappers_decide_structured_control_flow() {
        for (condition, taken) in [
            ("TRUE", true),
            ("FALSE", false),
            ("((TRUE))", true),
            ("((FALSE))", false),
            ("!FALSE", true),
            ("!TRUE", false),
            ("!(!TRUE)", true),
            ("((1 = 2))", false),
        ] {
            with_parsed_body(
                &format!("IF ({condition}) THEN\nPRINT 1\nELSEIF (Probe()) THEN\nPRINT 2\nENDIF\nPRINT 3\n"),
                |statements| {
                    let reachability = SourceReachability::build(statements);
                    let Statement::IfThen(branch) = &statements[0] else {
                        panic!("{statements:?}")
                    };
                    flags(&reachability, statements, &[true, true]);
                    flags(&reachability, branch.get_statements(), &[taken]);
                    let alternative = &branch.get_else_if_blocks()[0];
                    flags(&reachability, alternative.get_statements(), &[!taken]);
                    assert_eq!(Some(!taken), reachability.expression_is_reachable(alternative.get_condition()), "{condition}");
                },
            );
            with_parsed_body(&format!("WHILE ({condition}) DO\nCONTINUE\nPRINT 1\nENDWHILE\nPRINT 2\n"), |statements| {
                let reachability = SourceReachability::build(statements);
                let Statement::WhileDo(body) = &statements[0] else { panic!("{statements:?}") };
                flags(&reachability, statements, &[true, !taken]);
                flags(&reachability, body.get_statements(), &[taken, false]);
                assert_eq!(!taken, reachability.falls_through(), "{condition}");
            });
            with_parsed_body(&format!("REPEAT\nCONTINUE\nPRINT 1\nUNTIL ({condition})\nPRINT 2\n"), |statements| {
                let reachability = SourceReachability::build(statements);
                let Statement::RepeatUntil(body) = &statements[0] else {
                    panic!("{statements:?}")
                };
                flags(&reachability, statements, &[true, taken]);
                flags(&reachability, body.get_statements(), &[true, false]);
                assert_eq!(Some(true), reachability.expression_is_reachable(body.get_condition()));
                assert_eq!(taken, reachability.falls_through(), "{condition}");
            });
        }
    }

    #[test]
    fn parsed_boolean_gotos_agree_with_lowered_optimizer() {
        for (condition, taken) in [("TRUE", true), ("FALSE", false), ("((!FALSE))", true), ("((!TRUE))", false)] {
            with_parsed_body(&format!("IF ({condition}) GOTO done\nPRINT 1\n:done\nPRINT 2\n"), |statements| {
                let reachability = SourceReachability::build(statements);
                flags(&reachability, statements, &[true, !taken, true, true]);
                let optimized = super::optimize_statements(statements);
                assert_eq!(!taken, optimized.contains(&statements[1]), "{condition}: {optimized:?}");
                assert!(optimized.contains(&statements[3]));
            });
        }
    }

    #[test]
    fn parsed_named_booleans_do_not_hide_call_operands() {
        for condition in ["FALSE & Probe()", "TRUE | Probe()", "!(FALSE & Probe())", "0 * Probe()"] {
            with_parsed_body(&format!("IF ({condition}) THEN\nPRINT 1\nENDIF\nPRINT 2\n"), |statements| {
                let reachability = SourceReachability::build(statements);
                let Statement::IfThen(branch) = &statements[0] else {
                    panic!("{statements:?}")
                };
                flags(&reachability, statements, &[true, true]);
                flags(&reachability, branch.get_statements(), &[true]);
                assert_eq!(Some(true), reachability.expression_is_reachable(branch.get_condition()));
            });
        }
    }

    #[test]
    fn source_if_visits_dead_bodies_without_losing_the_continuation() {
        for taken in [false, true] {
            let statements = vec![IfStatement::create_empty_statement(boolean(taken), work()), work()];
            let reachability = SourceReachability::build(&statements);
            let Statement::If(branch) = &statements[0] else { unreachable!() };
            flags(&reachability, &statements, &[true, true]);
            assert_eq!(Some(taken), reachability.statement_is_reachable(branch.get_statement()));
            assert_eq!(Some(true), reachability.expression_is_reachable(branch.get_condition()));
            assert!(reachability.falls_through());
        }
    }

    #[test]
    fn source_elseif_tests_are_separate_execution_points() {
        let statements = vec![
            IfThenStatement::create_empty_statement(
                boolean(false),
                vec![work()],
                vec![
                    ElseIfBlock::empty(boolean(true), vec![returns(), work()]),
                    ElseIfBlock::empty(call(), vec![work()]),
                ],
                Some(ElseBlock::empty(vec![work()])),
            ),
            work(),
        ];
        let reachability = SourceReachability::build(&statements);
        let Statement::IfThen(branch) = &statements[0] else { unreachable!() };
        flags(&reachability, &statements, &[true, false]);
        flags(&reachability, branch.get_statements(), &[false]);
        flags(&reachability, branch.get_else_if_blocks()[0].get_statements(), &[true, false]);
        flags(&reachability, branch.get_else_if_blocks()[1].get_statements(), &[false]);
        flags(&reachability, branch.get_else_block().as_ref().unwrap().get_statements(), &[false]);
        assert_eq!(Some(true), reachability.expression_is_reachable(branch.get_else_if_blocks()[0].get_condition()));
        assert_eq!(
            Some(false),
            reachability.expression_is_reachable(branch.get_else_if_blocks()[1].get_condition())
        );
        assert!(!reachability.falls_through());
    }

    #[test]
    fn source_unknown_if_merges_both_returning_branches() {
        for has_else in [false, true] {
            let statements = vec![
                IfThenStatement::create_empty_statement(call(), vec![returns()], Vec::new(), has_else.then(|| ElseBlock::empty(vec![returns()]))),
                work(),
            ];
            let reachability = SourceReachability::build(&statements);
            flags(&reachability, &statements, &[true, !has_else]);
            assert_eq!(!has_else, reachability.falls_through());
        }
    }

    #[test]
    fn source_while_false_skips_body_and_true_continue_never_exits() {
        for taken in [false, true] {
            let statements = vec![
                WhileDoStatement::create_empty_statement(boolean(taken), vec![ContinueStatement::create_empty_statement(), work()]),
                work(),
            ];
            let reachability = SourceReachability::build(&statements);
            let Statement::WhileDo(value) = &statements[0] else { unreachable!() };
            flags(&reachability, &statements, &[true, !taken]);
            flags(&reachability, value.get_statements(), &[taken, false]);
            assert_eq!(!taken, reachability.falls_through());
        }
        let statements = vec![
            WhileStatement::create_empty_statement(boolean(true), BreakStatement::create_empty_statement()),
            work(),
        ];
        flags(&SourceReachability::build(&statements), &statements, &[true, true]);
    }

    #[test]
    fn source_repeat_condition_runs_after_continue_but_not_break_or_return() {
        for (transfer, condition_live, continuation_live) in [
            (ContinueStatement::create_empty_statement(), true, true),
            (BreakStatement::create_empty_statement(), false, true),
            (returns(), false, false),
        ] {
            let statements = vec![RepeatUntilStatement::create_empty_statement(call(), vec![transfer, work()]), work()];
            let reachability = SourceReachability::build(&statements);
            let Statement::RepeatUntil(value) = &statements[0] else { unreachable!() };
            flags(&reachability, &statements, &[true, continuation_live]);
            flags(&reachability, value.get_statements(), &[true, false]);
            assert_eq!(Some(condition_live), reachability.expression_is_reachable(value.get_condition()));
        }
        for until in [false, true] {
            let statements = vec![RepeatUntilStatement::create_empty_statement(boolean(until), vec![work()]), work()];
            flags(&SourceReachability::build(&statements), &statements, &[true, until]);
        }
    }

    #[test]
    fn source_nested_loop_break_does_not_escape_outer_loop() {
        let statements = vec![
            LoopStatement::create_empty_statement(vec![
                LoopStatement::create_empty_statement(vec![BreakStatement::create_empty_statement(), work()]),
                ContinueStatement::create_empty_statement(),
                work(),
            ]),
            work(),
        ];
        let reachability = SourceReachability::build(&statements);
        let Statement::Loop(outer) = &statements[0] else { unreachable!() };
        let Statement::Loop(inner) = &outer.get_statements()[0] else { unreachable!() };
        flags(&reachability, &statements, &[true, false]);
        flags(&reachability, outer.get_statements(), &[true, true, false]);
        flags(&reachability, inner.get_statements(), &[true, false]);
        assert!(!reachability.falls_through());
    }

    #[test]
    fn source_for_keeps_zero_iteration_path_and_visits_bounds_once() {
        let statements = vec![
            Statement::For(ForStatement::empty(name("I"), call(), call(), Some(Box::new(call())), vec![returns(), work()])),
            work(),
        ];
        let reachability = SourceReachability::build(&statements);
        let Statement::For(value) = &statements[0] else { unreachable!() };
        flags(&reachability, &statements, &[true, true]);
        flags(&reachability, value.get_statements(), &[true, false]);
        for expression in [value.get_start_expr(), value.get_end_expr(), value.get_step_expr().as_deref().unwrap()] {
            assert_eq!(Some(true), reachability.expression_is_reachable(expression));
        }
        // STEP still executes in the header even when no path reaches increment.
        assert!(reachability.falls_through());
    }

    #[test]
    fn source_foreach_is_a_loop_boundary_with_a_possible_empty_collection() {
        let statements = vec![
            LoopStatement::create_empty_statement(vec![
                ForEachStatement::create_empty_statement(
                    name("Item"),
                    call(),
                    vec![
                        WhileStatement::create_empty_statement(boolean(true), BreakStatement::create_empty_statement()),
                        ContinueStatement::create_empty_statement(),
                        work(),
                    ],
                ),
                BreakStatement::create_empty_statement(),
                work(),
            ]),
            work(),
        ];
        let reachability = SourceReachability::build(&statements);
        let Statement::Loop(outer) = &statements[0] else { unreachable!() };
        let Statement::ForEach(value) = &outer.get_statements()[0] else {
            unreachable!()
        };
        flags(&reachability, &statements, &[true, true]);
        flags(&reachability, outer.get_statements(), &[true, true, false]);
        flags(&reachability, value.get_statements(), &[true, true, false]);
        assert_eq!(Some(true), reachability.expression_is_reachable(value.get_collection()));

        let statements = vec![ForEachStatement::create_empty_statement(name("Item"), call(), vec![returns(), work()]), work()];
        flags(&SourceReachability::build(&statements), &statements, &[true, true]);
    }

    #[test]
    fn source_select_merges_cases_and_default_without_fallthrough_between_cases() {
        let statements = vec![
            SelectStatement::create_empty_statement(
                call(),
                vec![
                    CaseBlock::empty(vec![CaseSpecifier::Expression(Box::new(call()))], vec![returns(), work()]),
                    CaseBlock::empty(vec![CaseSpecifier::FromTo(Box::new(call()), Box::new(call()))], vec![returns(), work()]),
                ],
                vec![returns(), work()],
            ),
            work(),
        ];
        let reachability = SourceReachability::build(&statements);
        let Statement::Select(value) = &statements[0] else { unreachable!() };
        flags(&reachability, &statements, &[true, false]);
        assert_eq!(Some(true), reachability.expression_is_reachable(value.get_expression()));
        for block in value.get_case_blocks() {
            flags(&reachability, block.get_statements(), &[true, false]);
            for specifier in block.get_case_specifiers() {
                match specifier {
                    CaseSpecifier::Expression(value) => assert_eq!(Some(true), reachability.expression_is_reachable(value)),
                    CaseSpecifier::FromTo(from, to) => {
                        assert_eq!(Some(true), reachability.expression_is_reachable(from));
                        assert_eq!(Some(true), reachability.expression_is_reachable(to));
                    }
                }
            }
        }
        flags(&reachability, value.get_default_statements(), &[true, false]);
        assert!(!reachability.falls_through());
    }

    #[test]
    fn source_select_without_default_can_exit_and_without_cases_does_not_evaluate_selector() {
        let statements = vec![
            SelectStatement::create_empty_statement(
                integer(1),
                vec![CaseBlock::empty(vec![CaseSpecifier::Expression(Box::new(integer(1)))], vec![returns()])],
                Vec::new(),
            ),
            SelectStatement::create_empty_statement(call(), Vec::new(), vec![work()]),
            work(),
        ];
        let reachability = SourceReachability::build(&statements);
        flags(&reachability, &statements, &[true, true, true]);
        let Statement::Select(value) = &statements[1] else { unreachable!() };
        assert_eq!(Some(false), reachability.expression_is_reachable(value.get_expression()));
        flags(&reachability, value.get_default_statements(), &[true]);
    }

    #[test]
    fn source_goto_can_enter_a_dead_parent_and_exit_the_entire_construct() {
        let statements = vec![
            GotoStatement::create_empty_statement(name("Inside")),
            IfThenStatement::create_empty_statement(
                call(),
                vec![
                    work(),
                    LabelStatement::create_empty_statement(name("Inside")),
                    GotoStatement::create_empty_statement(name("Done")),
                    work(),
                ],
                Vec::new(),
                Some(ElseBlock::empty(vec![work()])),
            ),
            work(),
            LabelStatement::create_empty_statement(name("Done")),
            work(),
        ];
        let reachability = SourceReachability::build(&statements);
        let Statement::IfThen(value) = &statements[1] else { unreachable!() };
        flags(&reachability, &statements, &[true, false, false, true, true]);
        flags(&reachability, value.get_statements(), &[false, true, true, false]);
        flags(&reachability, value.get_else_block().as_ref().unwrap().get_statements(), &[false]);
        assert_eq!(Some(false), reachability.expression_is_reachable(value.get_condition()));
    }

    #[test]
    fn source_jump_into_for_body_does_not_revive_its_initializer() {
        let statements = vec![
            GotoStatement::create_empty_statement(name("Inside")),
            Statement::For(ForStatement::empty(
                name("I"),
                call(),
                call(),
                Some(Box::new(call())),
                vec![
                    LabelStatement::create_empty_statement(name("Inside")),
                    ContinueStatement::create_empty_statement(),
                    work(),
                ],
            )),
            work(),
        ];
        let reachability = SourceReachability::build(&statements);
        let Statement::For(value) = &statements[1] else { unreachable!() };
        flags(&reachability, &statements, &[true, false, true]);
        flags(&reachability, value.get_statements(), &[true, true, false]);
        assert_eq!(Some(false), reachability.expression_is_reachable(value.get_start_expr()));
        assert_eq!(Some(true), reachability.expression_is_reachable(value.get_end_expr()));
        assert_eq!(Some(true), reachability.expression_is_reachable(value.get_step_expr().as_deref().unwrap()));
    }

    #[test]
    fn source_gosub_and_reachable_error_installations_retain_nested_targets() {
        for mode in [None, Some(OnErrorMode::Goto), Some(OnErrorMode::Gosub)] {
            let transfer = mode.map_or_else(
                || GosubStatement::create_empty_statement(name("handler")),
                |mode| OnErrorStatement::create_empty_statement(mode, name("handler")),
            );
            let statements = vec![
                transfer,
                returns(),
                WhileDoStatement::create_empty_statement(call(), vec![LabelStatement::create_empty_statement(name("HANDLER")), work(), returns()]),
                work(),
            ];
            let reachability = SourceReachability::build(&statements);
            let Statement::WhileDo(value) = &statements[2] else { unreachable!() };
            flags(&reachability, &statements, &[true, true, false, false]);
            flags(&reachability, value.get_statements(), &[true, true, true]);
            assert_eq!(Some(false), reachability.expression_is_reachable(value.get_condition()));
            assert!(!reachability.falls_through());
        }
        let statements = vec![
            returns(),
            OnErrorStatement::create_empty_statement(OnErrorMode::Goto, name("Handler")),
            LabelStatement::create_empty_statement(name("Handler")),
            work(),
        ];
        flags(&SourceReachability::build(&statements), &statements, &[true, false, false, false]);
    }

    #[test]
    fn source_terminal_builtins_and_outside_loop_transfers_preserve_legacy_policy() {
        for opcode in [OpCode::END, OpCode::STOP, OpCode::RETURN] {
            let statements = vec![
                BreakStatement::create_empty_statement(),
                ContinueStatement::create_empty_statement(),
                PredefinedCallStatement::create_empty_statement(opcode.get_definition(), Vec::new()),
                LabelStatement::create_empty_statement(name("Unreferenced")),
                work(),
            ];
            let reachability = SourceReachability::build(&statements);
            flags(&reachability, &statements, &[true, true, true, false, false]);
            assert!(!reachability.falls_through());
        }
    }

    #[test]
    fn source_boolean_folding_is_pure_and_does_not_discard_calls() {
        let statements = vec![
            IfStatement::create_empty_statement(BinaryExpression::create_empty_expression(BinOp::Eq, integer(1), integer(2)), work()),
            IfStatement::create_empty_statement(BinaryExpression::create_empty_expression(BinOp::And, boolean(false), call()), work()),
            work(),
        ];
        let before = statements.clone();
        let reachability = SourceReachability::build(&statements);
        flags(&reachability, &statements, &[true, true, true]);
        for (statement, expected) in statements[..2].iter().zip([false, true]) {
            let Statement::If(value) = statement else { unreachable!() };
            assert_eq!(Some(expected), reachability.statement_is_reachable(value.get_statement()));
            assert_eq!(Some(true), reachability.expression_is_reachable(value.get_condition()));
        }
        assert_eq!(before, statements);
    }

    #[test]
    fn source_legacy_sequence_api_flattens_only_blocks_and_keeps_original_nodes() {
        let statements = vec![BlockStatement::create_empty_statement(vec![
            IfStatement::create_empty_statement(boolean(true), returns()),
            work(),
        ])];
        let Statement::Block(block) = &statements[0] else { unreachable!() };
        let flags = statement_reachability(&statements);
        assert_eq!(2, flags.len());
        assert!(std::ptr::eq(flags[0].0, &block.get_statements()[0]));
        assert!(std::ptr::eq(flags[1].0, &block.get_statements()[1]));
        assert!(flags[0].1);
        assert!(!flags[1].1);
        assert!(SourceReachability::build(&[]).falls_through());
        assert_eq!(None, SourceReachability::default().statement_is_reachable(&statements[0]));
        assert_eq!(None, SourceReachability::default().expression_is_reachable(&call()));
    }
}
