//! The outline of a file: its types, routines and top level variables.

use icy_board_engine::ast::{Ast, AstNode, Statement};
use ropey::Rope;
use tower_lsp::lsp_types::{DocumentSymbol, Range, SymbolKind};

use crate::offset_to_position;

fn range(rope: &Rope, span: &std::ops::Range<usize>) -> Option<Range> {
    Some(Range::new(offset_to_position(span.start, rope)?, offset_to_position(span.end, rope)?))
}

#[allow(deprecated)] // `deprecated` is a field of DocumentSymbol, not a warning about it.
fn symbol(name: String, detail: Option<String>, kind: SymbolKind, full: Range, selection: Range, children: Vec<DocumentSymbol>) -> DocumentSymbol {
    DocumentSymbol {
        name,
        detail,
        kind,
        tags: None,
        deprecated: None,
        range: full,
        selection_range: selection,
        children: if children.is_empty() { None } else { Some(children) },
    }
}

/// The symbols of one file, nested the way they are written.
pub fn get_document_symbols(ast: &Ast, rope: &Rope) -> Vec<DocumentSymbol> {
    let mut symbols = Vec::new();

    for node in &ast.nodes {
        match node {
            AstNode::TypeDeclaration(declaration) => {
                let full = range(rope, &(declaration.get_type_token().span.start..declaration.get_endtype_token().span.end));
                let selection = range(rope, &declaration.get_identifier_token().span);
                let (Some(full), Some(selection)) = (full, selection) else {
                    continue;
                };
                let fields = declaration
                    .get_fields()
                    .iter()
                    .filter_map(|field| {
                        let span = field.get_type_token().span.start..field.get_specifier().get_identifier_token().span.end;
                        let full = range(rope, &span)?;
                        let selection = range(rope, &field.get_specifier().get_identifier_token().span)?;
                        Some(symbol(
                            field.get_identifier().to_string(),
                            Some(field.get_variable_type().to_string()),
                            SymbolKind::FIELD,
                            full,
                            selection,
                            Vec::new(),
                        ))
                    })
                    .collect();
                symbols.push(symbol(
                    declaration.get_identifier().to_string(),
                    None,
                    SymbolKind::STRUCT,
                    full,
                    selection,
                    fields,
                ));
            }

            AstNode::Function(function) => {
                let full = range(rope, &(function.get_function_token().span.start..function.get_endfunc_token().span.end));
                let selection = range(rope, &function.get_identifier_token().span);
                if let (Some(full), Some(selection)) = (full, selection) {
                    symbols.push(symbol(
                        function.get_identifier().to_string(),
                        Some(function.get_return_type().to_string()),
                        SymbolKind::FUNCTION,
                        full,
                        selection,
                        Vec::new(),
                    ));
                }
            }

            AstNode::Procedure(procedure) => {
                let full = range(rope, &(procedure.get_procedure_token().span.start..procedure.get_endproc_token().span.end));
                let selection = range(rope, &procedure.get_identifier_token().span);
                if let (Some(full), Some(selection)) = (full, selection) {
                    symbols.push(symbol(
                        procedure.get_identifier().to_string(),
                        None,
                        SymbolKind::METHOD,
                        full,
                        selection,
                        Vec::new(),
                    ));
                }
            }

            AstNode::TopLevelStatement(Statement::VariableDeclaration(declaration)) => {
                for specifier in declaration.get_variables() {
                    let span = declaration.get_type_token().span.start..specifier.get_identifier_token().span.end;
                    let (Some(full), Some(selection)) = (range(rope, &span), range(rope, &specifier.get_identifier_token().span)) else {
                        continue;
                    };
                    symbols.push(symbol(
                        specifier.get_identifier().to_string(),
                        Some(declaration.get_variable_type().to_string()),
                        SymbolKind::VARIABLE,
                        full,
                        selection,
                        Vec::new(),
                    ));
                }
            }

            AstNode::EnumDeclaration(declaration) => {
                let full = range(rope, &(declaration.get_enum_token().span.start..declaration.get_endenum_token().span.end));
                let selection = range(rope, &declaration.get_identifier_token().span);
                if let (Some(full), Some(selection)) = (full, selection) {
                    let children = declaration
                        .get_variants()
                        .iter()
                        .filter_map(|variant| {
                            let selection = range(rope, &variant.get_identifier_token().span)?;
                            Some(symbol(
                                variant.get_identifier().to_string(),
                                Some(variant.get_value().to_string()),
                                SymbolKind::ENUM_MEMBER,
                                selection,
                                selection,
                                Vec::new(),
                            ))
                        })
                        .collect();
                    symbols.push(symbol(
                        declaration.get_identifier().to_string(),
                        Some("INTEGER".to_string()),
                        SymbolKind::ENUM,
                        full,
                        selection,
                        children,
                    ));
                }
            }

            _ => {}
        }
    }

    let Some(module) = &ast.module else { return symbols };
    if module.is_implicit() {
        return symbols;
    }
    let full = range(rope, &(module.module_token.span.start..module.endmodule_token.span.end));
    let selection = range(rope, &module.name_token.span);
    match (full, selection) {
        (Some(full), Some(selection)) => vec![symbol(module.name().to_string(), None, SymbolKind::NAMESPACE, full, selection, symbols)],
        _ => symbols,
    }
}
