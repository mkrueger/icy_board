use icy_ppe::{ast::Ast, parser::lexer::Spanned};

use icy_ppe::semantic::SemanticVisitor;

pub fn get_definition(ast: &Ast, offset: usize) -> Option<Spanned<String>> {
    let mut semantic_visitor = SemanticVisitor::default();
    ast.visit(&mut semantic_visitor);

    for (_, refs) in &semantic_visitor.references {
        if refs.contains(offset) {
            if let Some(decl) = &refs.declaration {
                return Some(decl.clone());
            }
        }
    }
    None
}
