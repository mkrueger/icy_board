use crate::ast::{Ast, AstVisitor, AstVisitorMut, GosubStatement, GotoStatement, LabelStatement};
use unicase::Ascii;

struct GatherLabelVisitor {
    labels: Vec<Ascii<String>>,
}

impl AstVisitor<()> for GatherLabelVisitor {
    fn visit_label_statement(&mut self, label_stmt: &crate::ast::LabelStatement) {
        self.labels.push(unicase::Ascii::new(label_stmt.get_label().to_string()));
    }
}

struct TransformLabelVisitor {
    labels: Vec<Ascii<String>>,
}

impl TransformLabelVisitor {
    fn new(labels: Vec<Ascii<String>>) -> Self {
        Self { labels }
    }

    fn get_label(&self, get_label: &Ascii<String>) -> Ascii<String> {
        if let Some((index, _)) = self.labels.iter().enumerate().find(|(_i, label)| *label == get_label) {
            Ascii::new(format!("LABEL{:03}", index + 1))
        } else {
            get_label.clone()
        }
    }
}
impl AstVisitorMut for TransformLabelVisitor {
    fn visit_goto_statement(&mut self, goto_stmt: &crate::ast::GotoStatement) -> crate::ast::Statement {
        let label = self.get_label(&goto_stmt.get_label());
        GotoStatement::create_empty_statement(label)
    }

    fn visit_gosub_statement(&mut self, gosub: &crate::ast::GosubStatement) -> crate::ast::Statement {
        let label = self.get_label(&gosub.get_label());
        GosubStatement::create_empty_statement(label)
    }

    fn visit_label_statement(&mut self, label_stmt: &crate::ast::LabelStatement) -> crate::ast::Statement {
        let label = self.get_label(&label_stmt.get_label());
        LabelStatement::create_empty_statement(label)
    }
}
pub fn relabel_ast(ast: &mut Ast) -> Ast {
    let mut gather_label_visitor = GatherLabelVisitor { labels: Vec::new() };
    ast.visit(&mut gather_label_visitor);
    let mut transform_label_visitor = TransformLabelVisitor::new(gather_label_visitor.labels);
    ast.visit_mut(&mut transform_label_visitor)
}
