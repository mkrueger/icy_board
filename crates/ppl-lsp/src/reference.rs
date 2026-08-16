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
        if refs.contains_pos(&ast.file_name, offset) {
            if let Some((p, decl)) = &refs.declaration {
                if include_self || !decl.span.contains(&offset) || p != &ast.file_name {
                    reference_list.push((p.clone(), decl.clone()));
                }
            }
            if let Some((p, decl)) = &refs.implementation {
                if include_self || !decl.span.contains(&offset) || p != &ast.file_name {
                    reference_list.push((p.clone(), decl.clone()));
                }
            }
            for (p, r) in &refs.usages {
                if include_self || !r.span.contains(&offset) || p != &ast.file_name {
                    reference_list.push((p.clone(), r.clone()));
                }
            }

            for (p, r) in &refs.return_types {
                if include_self || !r.span.contains(&offset) || p != &ast.file_name {
                    reference_list.push((p.clone(), r.clone()));
                }
            }

            break;
        }
    }
    reference_list.sort_by(|a, b| if a.0 == b.0 { b.1.span.start.cmp(&a.1.span.start) } else { a.0.cmp(&b.0) });
    // A spot can be recorded more than once - an argument counts both as a value
    // and as what it is bound to. Naming it twice would make rename ask the editor
    // to change the same range twice, which is not a valid edit.
    reference_list.dedup_by(|a, b| a.0 == b.0 && a.1.span == b.1.span);
    reference_list
}
