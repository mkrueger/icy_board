use std::collections::HashMap;

use crate::ast::AstVisitorMut;
#[derive(Default)]
pub struct RenameVisitor {
    pub rename_map: HashMap<unicase::Ascii<String>, unicase::Ascii<String>>,
}

impl RenameVisitor {
    pub fn new(rename_map: HashMap<unicase::Ascii<String>, unicase::Ascii<String>>) -> Self {
        Self { rename_map }
    }
}

impl AstVisitorMut for RenameVisitor {
    fn visit_identifier(&mut self, id: &unicase::Ascii<String>) -> unicase::Ascii<String> {
        if let Some(rename) = self.rename_map.get(id) {
            rename.clone()
        } else {
            id.clone()
        }
    }
}
