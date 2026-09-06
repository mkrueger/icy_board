use for_next::scan_for_next;
use select_case::scan_select_statements;
use unicase::Ascii;

use crate::{
    ast::{AstVisitor, AstVisitorMut, BreakStatement, ContinueStatement, IfStatement, IfThenStatement, RenameVisitor},
    executable::OpCode,
    semantic::{ReferenceType, SemanticVisitor},
};

use self::while_do::scan_do_while;

use super::{Ast, Expression, Statement, rename_visitor::RenameScanVisitor};

pub mod for_next;
mod if_else;
mod loop_endloop;
mod remove_label_visitor;
mod repeat_until;
mod select_case;
mod unused_label_visitor;
mod while_do;

pub fn reconstruct_block(visitor: &SemanticVisitor, statements: &mut Vec<Statement>, lang_version: u16) {
    optimize_block(visitor, statements, lang_version);
}

fn _optimize_argument(arg: &mut Expression) {
    if let Expression::Parens(expr) = arg {
        *arg = expr.get_expression().clone();
    }
}

pub fn optimize_loops(visitor: &SemanticVisitor, statements: &mut Vec<Statement>, lang_version: u16) {
    scan_for_next(visitor, statements, lang_version);
    scan_do_while(visitor, statements, lang_version);
    if lang_version >= 350 {
        repeat_until::scan_repeat_until(visitor, statements, lang_version);
        loop_endloop::scan_loop(visitor, statements, lang_version);
    }
}

fn optimize_block(visitor: &SemanticVisitor, statements: &mut Vec<Statement>, lang_version: u16) {
    // FOREACH comes structured out of the bytecode, so its body is reached here rather
    // than where a pass builds one.
    for statement in statements.iter_mut() {
        if let Statement::ForEach(foreach_stmt) = statement {
            optimize_block(visitor, foreach_stmt.get_statements_mut(), lang_version);
        }
    }
    optimize_loops(visitor, statements, lang_version);
    optimize_ifs(visitor, statements, lang_version);
    if lang_version >= 200 {
        scan_select_statements(statements);
    }
}

fn optimize_ifs(visitor: &SemanticVisitor, statements: &mut Vec<Statement>, lang_version: u16) {
    scan_negated_if(visitor, statements);
    scan_if(visitor, statements, lang_version);
    if_else::scan_if_else(visitor, statements, lang_version);
}

fn scan_label(statements: &[Statement], from: usize, label: &unicase::Ascii<String>) -> Option<usize> {
    for (j, stmt) in statements.iter().enumerate().skip(from) {
        if let Statement::Label(label_stmt) = stmt
            && label_stmt.get_label() == label
        {
            return Some(j);
        }
    }
    None
}

/// A CONTINUE jumps to the loop head just like the back edge does, so the back edge is
/// the one the break label follows.
fn scan_loop_back_edge(statements: &[Statement], from: usize, head: &unicase::Ascii<String>, break_label: &unicase::Ascii<String>) -> Option<usize> {
    for j in from..statements.len().saturating_sub(1) {
        let Statement::Goto(goto_stmt) = &statements[j] else {
            continue;
        };
        if goto_stmt.get_label() != head {
            continue;
        }
        if let Statement::Label(label_stmt) = &statements[j + 1]
            && label_stmt.get_label() == break_label
        {
            return Some(j);
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
        }
    }
}

fn scan_if(visitor: &SemanticVisitor, statements: &mut Vec<Statement>, lang_version: u16) {
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
                if let Some((_, decl)) = &r.declaration
                    && decl.token == end_label
                {
                    return r.usages.len() == 1;
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
        optimize_block(visitor, &mut statements2, lang_version);
        if statements2.len() == 1 && is_simple_statement(&statements[0]) {
            statements[i] = IfStatement::create_empty_statement(if_stmt.get_condition().negate_expression(), statements2.pop().unwrap());
        } else {
            statements[i] = IfThenStatement::create_empty_statement(if_stmt.get_condition().negate_expression(), statements2, Vec::new(), None);
        }
    }
}

fn is_simple_statement(statements: &Statement) -> bool {
    match statements {
        Statement::Gosub(_) | Statement::Goto(_) => true,
        Statement::PredifinedCall(pcall) => matches!(
            pcall.get_func().opcode,
            OpCode::RETURN | OpCode::END | OpCode::STOP | OpCode::PRINT | OpCode::PRINTLN
        ),
        _ => false,
    }
}

fn get_label_index(statements: &[Statement], from: i32, to: i32, label: &String) -> Option<usize> {
    for j in from..to {
        if let Statement::Label(next_label) = &statements[j as usize]
            && next_label.get_label() == label
        {
            return Some(j as usize);
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
        Ascii::new(String::new())
    }
}

#[derive(Default)]
struct LabelReferences {
    labels: Vec<Ascii<String>>,
    targets: Vec<Ascii<String>>,
    backward_jump: bool,
}

impl AstVisitor<()> for LabelReferences {
    fn visit_label_statement(&mut self, label: &crate::ast::LabelStatement) {
        self.labels.push(label.get_label().clone());
    }

    fn visit_goto_statement(&mut self, goto: &crate::ast::GotoStatement) {
        self.backward_jump |= self.labels.contains(goto.get_label());
        self.targets.push(goto.get_label().clone());
    }

    fn visit_gosub_statement(&mut self, gosub: &crate::ast::GosubStatement) {
        self.targets.push(gosub.get_label().clone());
    }

    fn visit_on_error_statement(&mut self, statement: &crate::ast::OnErrorStatement) {
        if let Some(target) = statement.get_target() {
            self.targets.push(target.clone());
        }
    }
}

fn collect_labels(statements: &[Statement]) -> LabelReferences {
    let mut references = LabelReferences::default();
    for statement in statements {
        statement.visit(&mut references);
    }
    references
}

fn label_references(statements: &[Statement], label: &Ascii<String>) -> usize {
    collect_labels(statements).targets.iter().filter(|target| *target == label).count()
}

/// Moving an externally entered region into a loop can change initialization
/// and scope. Also consult the original semantic references for callers outside
/// the current (possibly already extracted) block.
fn has_external_entries(visitor: &SemanticVisitor, statements: &[Statement], region: std::ops::Range<usize>) -> bool {
    let inside = collect_labels(&statements[region]);
    let all = collect_labels(statements);
    inside.labels.iter().any(|label| {
        let local_count = inside.targets.iter().filter(|target| *target == label).count();
        all.targets.iter().filter(|target| *target == label).count() > local_count
            || visitor.references.iter().any(|(kind, reference)| {
                matches!(kind, ReferenceType::Label(_))
                    && reference
                        .declaration
                        .as_ref()
                        .is_some_and(|(_, declaration)| declaration.token.eq_ignore_ascii_case(&format!(":{label}")) && reference.usages.len() > local_count)
            })
    })
}

// Kept as `&mut Vec<Statement>`: while_do.rs's caller later moves the same
// binding by value into `Vec`-typed APIs, and narrowing this to `&mut [Statement]`
// makes that binding's type ambiguous to infer (confirmed: caused a build break).
#[allow(clippy::ptr_arg)]
pub fn handle_break_continue(break_label: Ascii<String>, continue_label: Ascii<String>, statements: &mut Vec<Statement>) {
    // Failed inner reconstruction may leave a raw cycle. Rewriting outer jumps
    // now risks a later pass capturing BREAK/CONTINUE in that inner loop.
    if collect_labels(statements).backward_jump {
        return;
    }
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
    // BREAK and CONTINUE always bind to the innermost loop. Keep cross-loop
    // GOTOs intact, including in FOREACH bodies already structured by decoding.
    fn visit_while_statement(&mut self, statement: &crate::ast::WhileStatement) -> Statement {
        Statement::While(statement.clone())
    }

    fn visit_while_do_statement(&mut self, statement: &crate::ast::WhileDoStatement) -> Statement {
        Statement::WhileDo(statement.clone())
    }

    fn visit_repeat_until_statement(&mut self, statement: &crate::ast::RepeatUntilStatement) -> Statement {
        Statement::RepeatUntil(statement.clone())
    }

    fn visit_loop_statement(&mut self, statement: &crate::ast::LoopStatement) -> Statement {
        Statement::Loop(statement.clone())
    }

    fn visit_for_statement(&mut self, statement: &crate::ast::ForStatement) -> Statement {
        Statement::For(statement.clone())
    }

    fn visit_foreach_statement(&mut self, statement: &crate::ast::ForEachStatement) -> Statement {
        Statement::ForEach(statement.clone())
    }

    fn visit_goto_statement(&mut self, goto: &crate::ast::GotoStatement) -> Statement {
        if !self.break_label.is_empty() && goto.get_label() == &self.break_label {
            BreakStatement::create_empty_statement()
        } else if !self.continue_label.is_empty() && goto.get_label() == &self.continue_label {
            ContinueStatement::create_empty_statement()
        } else {
            Statement::Goto(goto.clone())
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ast::{
            BinOp, BinaryExpression, Constant, ConstantExpression, ForEachStatement, ForStatement, GotoStatement, IdentifierExpression, LabelStatement,
            LetStatement, LoopStatement, RepeatUntilStatement, WhileDoStatement, WhileStatement, constant::NumberFormat,
        },
        compiler::workspace::Workspace,
        parser::{ErrorReporter, UserTypeRegistry, lexer::Token},
    };
    use std::sync::{Arc, Mutex};

    pub(super) fn visitor() -> SemanticVisitor {
        SemanticVisitor::new(
            &Workspace::default(),
            Arc::new(Mutex::new(ErrorReporter::default())),
            UserTypeRegistry::default(),
        )
    }

    pub(super) fn name(value: &str) -> Ascii<String> {
        Ascii::new(value.to_string())
    }

    pub(super) fn var(value: &str) -> Expression {
        IdentifierExpression::create_empty_expression(name(value))
    }

    pub(super) fn int(value: i32) -> Expression {
        ConstantExpression::create_empty_expression(Constant::Integer(value, NumberFormat::Default))
    }

    pub(super) fn bin(op: BinOp, left: Expression, right: Expression) -> Expression {
        BinaryExpression::create_empty_expression(op, left, right)
    }

    pub(super) fn assign(target: &str, value: Expression) -> Statement {
        LetStatement::create_empty_statement(name(target), Token::Eq, Vec::new(), value)
    }

    pub(super) fn label(value: &str) -> Statement {
        LabelStatement::create_empty_statement(name(value))
    }

    pub(super) fn goto(value: &str) -> Statement {
        GotoStatement::create_empty_statement(name(value))
    }

    pub(super) fn foreach(body: Vec<Statement>) -> Statement {
        Statement::ForEach(ForEachStatement::empty(name("item"), var("items"), body))
    }

    pub(super) fn raw_while(head: &str, exit: &str, body: Vec<Statement>, modern: bool) -> Vec<Statement> {
        let mut result = vec![label(head)];
        if modern {
            result.extend([IfStatement::create_empty_statement(var("condition"), goto("body")), goto(exit), label("body")]);
        } else {
            result.push(IfStatement::create_empty_statement(var("condition"), goto(exit)));
        }
        result.extend(body);
        result.extend([goto(head), label(exit)]);
        result
    }

    #[test]
    fn outer_rewrite_skips_every_structured_loop_but_visits_conditionals() {
        let body = vec![goto("exit"), goto("head")];
        let loops = vec![
            foreach(body.clone()),
            WhileStatement::create_empty_statement(int(1), goto("exit")),
            WhileDoStatement::create_empty_statement(int(1), body.clone()),
            RepeatUntilStatement::create_empty_statement(int(0), body.clone()),
            LoopStatement::create_empty_statement(body.clone()),
            ForStatement::create_empty_statement(name("i"), int(1), int(5), None, body),
        ];
        let mut statements = loops.clone();
        statements.extend([
            IfStatement::create_empty_statement(var("stop"), goto("exit")),
            IfThenStatement::create_empty_statement(var("skip"), vec![goto("head")], Vec::new(), None),
        ]);
        handle_break_continue(name("exit"), name("head"), &mut statements);
        assert_eq!(&statements[..loops.len()], loops.as_slice());
        let Statement::If(statement) = &statements[loops.len()] else {
            panic!("expected IF")
        };
        assert!(matches!(statement.get_statement(), Statement::Break(_)));
        let Statement::IfThen(statement) = &statements[loops.len() + 1] else {
            panic!("expected IF THEN")
        };
        assert!(matches!(statement.get_statements()[0], Statement::Continue(_)));
    }

    #[test]
    fn raw_inner_cycles_do_not_capture_outer_break_or_continue() {
        let mut body = vec![label("inner"), goto("exit"), goto("head"), goto("inner")];
        let original = body.clone();
        handle_break_continue(name("exit"), name("head"), &mut body);
        assert_eq!(body, original);
    }

    #[test]
    fn inner_reconstruction_precedes_outer_rewrite_in_all_loop_passes() {
        for kind in 0..4 {
            let inner = raw_while("inner", "inner_exit", vec![goto("exit"), goto("head"), assign("x", int(1))], false);
            let mut statements = match kind {
                0 | 1 => raw_while("head", "exit", inner, kind == 1),
                2 => {
                    let mut result = vec![label("head")];
                    result.extend(inner);
                    result.extend([label("test"), IfStatement::create_empty_statement(var("again"), goto("head")), label("exit")]);
                    result
                }
                _ => {
                    let mut result = vec![label("head")];
                    result.extend(inner);
                    result.extend([goto("head"), label("exit")]);
                    result
                }
            };
            match kind {
                0 | 1 => while_do::scan_do_while(&visitor(), &mut statements, 400),
                2 => repeat_until::scan_repeat_until(&visitor(), &mut statements, 400),
                _ => loop_endloop::scan_loop(&visitor(), &mut statements, 400),
            }
            assert!(
                matches!(statements[1], Statement::WhileDo(_) | Statement::RepeatUntil(_) | Statement::Loop(_)),
                "{statements:?}"
            );
            assert_eq!(
                label_references(&statements, &name("exit")),
                1,
                "outer exit was captured by inner loop, kind {kind}"
            );
            assert_eq!(
                label_references(&statements, &name("head")),
                1,
                "outer head was captured by inner loop, kind {kind}"
            );
        }
    }

    #[test]
    fn foreach_cross_loop_jumps_survive_while_reconstruction() {
        for modern in [false, true] {
            let nested = foreach(vec![goto("exit"), goto("head")]);
            let mut statements = raw_while("head", "exit", vec![nested.clone()], modern);
            while_do::scan_do_while(&visitor(), &mut statements, 400);
            let Statement::WhileDo(statement) = &statements[1] else {
                panic!("expected WHILE DO: {statements:?}")
            };
            assert!(statement.get_statements().contains(&nested));
            assert_eq!(label_references(&statements, &name("exit")), 1);
            assert_eq!(label_references(&statements, &name("head")), 1);
            assert_eq!(statements[0], label("head"));
            assert_eq!(statements.last(), Some(&label("exit")));
        }
    }

    #[test]
    fn modern_while_head_and_back_edge_tail_are_continue_targets() {
        let mut statements = raw_while("head", "exit", vec![goto("head"), goto("tail"), label("tail")], true);
        while_do::scan_do_while(&visitor(), &mut statements, 400);
        let Statement::WhileDo(statement) = &statements[1] else {
            panic!("expected WHILE DO")
        };
        assert_eq!(statement.get_statements()[0], label("body"));
        assert!(matches!(statement.get_statements()[1], Statement::Continue(_)));
        assert!(matches!(statement.get_statements()[2], Statement::Continue(_)));
        assert_eq!(statement.get_statements()[3], label("tail"));
    }

    #[test]
    fn external_body_entries_prevent_loop_reconstruction() {
        for modern in [false, true] {
            let mut statements = vec![goto("entry")];
            statements.extend(raw_while("head", "exit", vec![label("entry"), assign("x", int(1))], modern));
            let original = statements.clone();
            optimize_loops(&visitor(), &mut statements, 400);
            assert_eq!(statements, original);
        }
    }
}
