use std::path::PathBuf;

use icy_board_engine::{ast::Ast, parser::lexer::Spanned, semantic::SemanticVisitor};

pub fn get_definition(ast: &Ast, visitor: &SemanticVisitor, offset: usize) -> Option<(PathBuf, Spanned<String>)> {
    for (_, refs) in &visitor.references {
        if refs.contains(&ast.file_name, offset) {
            if let Some((path, decl)) = &refs.implementation {
                return Some((path.clone(), decl.clone()));
            }
            if let Some((path, decl)) = &refs.declaration {
                return Some((path.clone(), decl.clone()));
            }
        }
    }
    None
}
