use std::collections::HashSet;

use unicase::Ascii;

use crate::ast::{AstVisitorMut, LabelStatement, Statement};

pub struct RemoveLabelVisitor {
    remove_labels: HashSet<Ascii<String>>,
}

impl RemoveLabelVisitor {
    pub fn new(remove_labels: HashSet<Ascii<String>>) -> Self {
        Self { remove_labels }
    }
}

impl AstVisitorMut for RemoveLabelVisitor {
    fn visit_label_statement(&mut self, label: &LabelStatement) -> Statement {
        if self.remove_labels.contains(&label.get_label()) {
            Statement::Empty
        } else {
            Statement::Label(label.clone())
        }
    }
}
