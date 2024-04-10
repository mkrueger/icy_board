use icy_ppe::{ast::Ast, parser::lexer::Spanned};

use icy_ppe::semantic::SemanticVisitor;

#[derive(Debug, Clone)]
pub enum ReferenceSymbol {
    Founded(Spanned<String>),
    Founding(usize),
}
pub fn get_reference(ast: &Ast, offset: usize, include_self: bool) -> Vec<Spanned<String>> {
    let mut reference_list = vec![];
    let mut semantic_visitor = SemanticVisitor::default();
    ast.visit(&mut semantic_visitor);

    for (_, refs) in &semantic_visitor.references {
        if refs.contains(offset) {
            if let Some(decl) = &refs.declaration {
                if include_self || !decl.span.contains(&offset) {
                    reference_list.push(decl.clone());
                }
            }
            if let Some(decl) = &refs.implementation {
                if include_self || !decl.span.contains(&offset) {
                    reference_list.push(decl.clone());
                }
            }
            for r in &refs.usages {
                if include_self || !r.span.contains(&offset) {
                    reference_list.push(r.clone());
                }
            }

            for r in &refs.return_types {
                if include_self || !r.span.contains(&offset) {
                    reference_list.push(r.clone());
                }
            }
        }
    }
    reference_list
}
