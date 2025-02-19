use std::path::PathBuf;

use icy_board_engine::{ast::Ast, parser::lexer::Spanned, semantic::SemanticVisitor};

#[derive(Debug, Clone)]
pub enum ReferenceSymbol {
    Founded(Spanned<String>),
    Founding(usize),
}
pub fn get_reference(ast: &Ast, offset: usize, semantic_visitor: &SemanticVisitor, include_self: bool) -> Vec<(PathBuf, Spanned<String>)> {
    let mut reference_list = vec![];

    for (_, refs) in &semantic_visitor.references {
        if refs.contains(&ast.file_name, offset) {
            if let Some((p, decl)) = &refs.declaration {
                if include_self || !decl.span.contains(&offset) {
                    reference_list.push((p.clone(), decl.clone()));
                }
            }
            if let Some((p, decl)) = &refs.implementation {
                if include_self || !decl.span.contains(&offset) {
                    reference_list.push((p.clone(), decl.clone()));
                }
            }
            for (p, r) in &refs.usages {
                if include_self || !r.span.contains(&offset) {
                    reference_list.push((p.clone(), r.clone()));
                }
            }

            for (p, r) in &refs.return_types {
                if include_self || !r.span.contains(&offset) {
                    reference_list.push((p.clone(), r.clone()));
                }
            }
        }
    }
    reference_list
}
