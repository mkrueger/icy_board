use std::sync::{Arc, Mutex};

use icy_board_engine::{
    ast::Ast,
    executable::LAST_PPLC,
    parser::{lexer::Spanned, ErrorReporter, UserTypeRegistry},
    semantic::SemanticVisitor,
};

pub fn get_definition(ast: &Ast, offset: usize) -> Option<Spanned<String>> {
    let mut reg = UserTypeRegistry::default();
    let mut semantic_visitor = SemanticVisitor::new(LAST_PPLC, Arc::new(Mutex::new(ErrorReporter::default())), &mut reg);
    ast.visit(&mut semantic_visitor);
    semantic_visitor.finish();

    for (_, refs) in &semantic_visitor.references {
        if refs.contains(offset) {
            if let Some(decl) = &refs.implementation {
                return Some(decl.clone());
            }
            if let Some(decl) = &refs.declaration {
                return Some(decl.clone());
            }
        }
    }
    None
}
