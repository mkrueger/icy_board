use std::sync::{Arc, Mutex};

use icy_board_engine::{
    ast::Ast,
    executable::LAST_PPLC,
    parser::{lexer::Spanned, ErrorRepoter, UserTypeRegistry},
    semantic::SemanticVisitor,
};

#[derive(Debug, Clone)]
pub enum ReferenceSymbol {
    Founded(Spanned<String>),
    Founding(usize),
}
pub fn get_reference(ast: &Ast, offset: usize, include_self: bool) -> Vec<Spanned<String>> {
    let mut reference_list = vec![];
    let mut reg = UserTypeRegistry::default();
    let mut semantic_visitor = SemanticVisitor::new(LAST_PPLC, Arc::new(Mutex::new(ErrorRepoter::default())), &mut reg);
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
