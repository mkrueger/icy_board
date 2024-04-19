use super::{AstNode, AstVisitor, AstVisitorMut};
use std::{fmt, path::PathBuf};

#[derive(Debug)]
pub struct Ast {
    pub nodes: Vec<AstNode>,
    pub file_name: PathBuf,

    pub require_user_variables: bool,
}

impl Ast {
    pub fn new() -> Self {
        Ast {
            nodes: vec![],
            file_name: PathBuf::new(),
            require_user_variables: false,
        }
    }

    pub fn visit<T: Default, V: AstVisitor<T>>(&self, visitor: &mut V) {
        visitor.visit_ast(self);
    }

    #[must_use]
    pub fn visit_mut<V: AstVisitorMut>(&self, visitor: &mut V) -> Self {
        visitor.visit_ast(self)
    }
}

impl Default for Ast {
    fn default() -> Self {
        Self::new()
    }
}

impl fmt::Display for Ast {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut output_visitor = crate::ast::output_visitor::OutputVisitor::default();
        self.visit(&mut output_visitor);

        write!(f, "{}", output_visitor.output)
    }
}
