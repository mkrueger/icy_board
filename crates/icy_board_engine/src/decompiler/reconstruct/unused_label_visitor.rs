use std::collections::HashSet;

use unicase::Ascii;

use crate::ast::AstVisitor;

#[derive(Default)]
pub struct UnusedLabelVisitor {
    defined_labels: HashSet<unicase::Ascii<String>>,
    used_labels: HashSet<unicase::Ascii<String>>,
}

impl UnusedLabelVisitor {
    pub fn get_unused_labels(&self) -> HashSet<Ascii<String>> {
        self.defined_labels.difference(&self.used_labels).cloned().collect()
    }
}

impl AstVisitor<()> for UnusedLabelVisitor {
    fn visit_label_statement(&mut self, label_stmt: &crate::ast::LabelStatement) {
        self.defined_labels.insert(unicase::Ascii::new(label_stmt.get_label().to_string()));
    }

    fn visit_goto_statement(&mut self, goto_stmt: &crate::ast::GotoStatement) {
        self.used_labels.insert(unicase::Ascii::new(goto_stmt.get_label().to_string()));
    }

    fn visit_gosub_statement(&mut self, gosub: &crate::ast::GosubStatement) -> () {
        self.used_labels.insert(unicase::Ascii::new(gosub.get_label().to_string()));
    }
}
