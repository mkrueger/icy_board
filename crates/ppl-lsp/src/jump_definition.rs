use std::sync::{Arc, Mutex};

use icy_board_engine::{
    ast::Ast,
    executable::LAST_PPLC,
    parser::{lexer::Spanned, ErrorRepoter, UserTypeRegistry},
    semantic::SemanticVisitor,
};

pub fn get_definition(ast: &Ast, offset: usize) -> Option<Spanned<String>> {
    let mut reg = UserTypeRegistry::default();
    let mut semantic_visitor = SemanticVisitor::new(LAST_PPLC, Arc::new(Mutex::new(ErrorRepoter::default())), &mut reg);
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
