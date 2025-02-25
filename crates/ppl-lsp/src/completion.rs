use std::sync::{Arc, Mutex};

use icy_board_engine::{
    ast::{constant::BUILTIN_CONSTS, walk_predefined_call_statement, Ast, AstVisitor, IdentifierExpression, PredefinedCallStatement},
    compiler::workspace::Workspace,
    executable::{StatementSignature, FUNCTION_DEFINITIONS, STATEMENT_DEFINITIONS},
    parser::{ErrorReporter, UserTypeRegistry},
    semantic::{ReferenceType, SemanticVisitor},
};
use tower_lsp::lsp_types::{CompletionItem, Documentation, HoverContents};

use crate::documentation::{get_const_hover, get_function_hover, get_statement_hover};

pub enum ImCompleteCompletionItem {
    Variable(String),
    Function(String, Vec<String>),
}

const KEYWORDS: [&str; 18] = [
    "LET",
    "GOTO",
    "GOSUB",
    "WHILE",
    "ENDWHILE",
    "IF",
    "ENDIF",
    "ELSE",
    "RETURN",
    "BREAK",
    "CONTINUE",
    "SELECT",
    "ENDSELECT",
    "DECLARE",
    "FUNCTION",
    "PROCEDURE",
    "ENDPROC",
    "ENDFUNC",
];

const TYPES: [&str; 27] = [
    "BOOLEAN",
    "DATE",
    "DDATE",
    "INTEGER",
    "SDWORD",
    "LONG",
    "MONEY",
    "STRING",
    "TIME",
    "BIGSTR",
    "EDATE",
    "REAL",
    "FLOAT",
    "DREAL",
    "DOUBLE",
    "UNSIGNED",
    "DWORD",
    "UDWORD",
    "BYTE",
    "UBYTE",
    "WORD",
    "UWORD",
    "SBYTE",
    "SHORT",
    "SWORD",
    "INT",
    "MSGAREAID",
];

/// return (need_to_continue_search, founded reference)
pub fn get_completion(ast: &Ast, offset: usize) -> Vec<CompletionItem> {
    let mut map = CompletionVisitor::new(offset);
    let reg = UserTypeRegistry::default();
    let mut semantic_visitor = SemanticVisitor::new(&Workspace::default(), Arc::new(Mutex::new(ErrorReporter::default())), reg);
    ast.visit(&mut semantic_visitor);
    semantic_visitor.finish();

    ast.visit(&mut map);

    if map.items.is_empty() {
        for stmt in KEYWORDS {
            map.items.push(CompletionItem {
                label: stmt.to_string(),
                insert_text: Some(stmt.to_string()),
                kind: Some(tower_lsp::lsp_types::CompletionItemKind::KEYWORD),
                insert_text_format: Some(tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }
        for stmt in TYPES {
            map.items.push(CompletionItem {
                label: stmt.to_string(),
                insert_text: Some(stmt.to_string()),
                kind: Some(tower_lsp::lsp_types::CompletionItemKind::CLASS),
                insert_text_format: Some(tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }

        for stmt in STATEMENT_DEFINITIONS.iter() {
            if stmt.sig == StatementSignature::Invalid {
                continue;
            }
            let content = if let Some(hover) = get_statement_hover(stmt) {
                if let HoverContents::Markup(content) = hover.contents {
                    Some(Documentation::MarkupContent(content))
                } else {
                    None
                }
            } else {
                None
            };

            map.items.push(CompletionItem {
                label: stmt.name.to_string(),
                insert_text: Some(stmt.name.to_string()),
                kind: Some(tower_lsp::lsp_types::CompletionItemKind::METHOD),
                insert_text_format: Some(tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT),
                documentation: content,
                ..Default::default()
            });
        }

        for (rt, r) in semantic_visitor.references {
            if matches!(rt, ReferenceType::Procedure(_)) {
                if let Some((_, decl)) = &r.declaration {
                    map.items.push(CompletionItem {
                        label: decl.token.to_string(),
                        insert_text: Some(decl.token.to_string()),
                        kind: Some(tower_lsp::lsp_types::CompletionItemKind::METHOD),
                        insert_text_format: Some(tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT),
                        ..Default::default()
                    });
                }
            }
            if matches!(rt, ReferenceType::Variable(_)) {
                if let Some((_, decl)) = &r.declaration {
                    map.items.push(CompletionItem {
                        label: decl.token.to_string(),
                        insert_text: Some(decl.token.to_string()),
                        kind: Some(tower_lsp::lsp_types::CompletionItemKind::VARIABLE),
                        insert_text_format: Some(tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT),
                        ..Default::default()
                    });
                }
            }
        }
    } else {
        for (rt, r) in &semantic_visitor.references {
            if matches!(rt, ReferenceType::Function(_)) {
                if let Some((_, decl)) = &r.declaration {
                    map.items.push(CompletionItem {
                        label: decl.token.to_string(),
                        insert_text: Some(decl.token.to_string()),
                        kind: Some(tower_lsp::lsp_types::CompletionItemKind::FUNCTION),
                        insert_text_format: Some(tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT),
                        ..Default::default()
                    });
                }
            }

            if matches!(rt, ReferenceType::Variable(_)) {
                if let Some((_, decl)) = &r.declaration {
                    map.items.push(CompletionItem {
                        label: decl.token.to_string(),
                        insert_text: Some(decl.token.to_string()),
                        kind: Some(tower_lsp::lsp_types::CompletionItemKind::VARIABLE),
                        insert_text_format: Some(tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT),
                        ..Default::default()
                    });
                }
            }
        }
    }

    map.items
}

#[derive(Default)]
struct CompletionVisitor {
    offset: usize,
    pub items: Vec<CompletionItem>,
}

impl CompletionVisitor {
    pub fn new(offset: usize) -> Self {
        Self { offset, items: Vec::new() }
    }

    fn add_functions(&mut self) {
        for c in BUILTIN_CONSTS.iter() {
            let content = if let Some(hover) = get_const_hover(c) {
                if let HoverContents::Markup(content) = hover.contents {
                    Some(Documentation::MarkupContent(content))
                } else {
                    None
                }
            } else {
                None
            };

            self.items.push(CompletionItem {
                label: c.name.to_string(),
                insert_text: Some(c.name.to_string()),
                kind: Some(tower_lsp::lsp_types::CompletionItemKind::CONSTANT),
                insert_text_format: Some(tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT),
                documentation: content,
                ..Default::default()
            });
        }

        for func in FUNCTION_DEFINITIONS.iter() {
            let content = if let Some(hover) = get_function_hover(func) {
                if let HoverContents::Markup(content) = hover.contents {
                    Some(Documentation::MarkupContent(content))
                } else {
                    None
                }
            } else {
                None
            };
            self.items.push(CompletionItem {
                label: func.name.to_string(),
                insert_text: Some(func.name.to_string()),
                kind: Some(tower_lsp::lsp_types::CompletionItemKind::FUNCTION),
                insert_text_format: Some(tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT),
                documentation: content,
                ..Default::default()
            });
        }
    }
}

impl AstVisitor<()> for CompletionVisitor {
    fn visit_identifier_expression(&mut self, identifier: &IdentifierExpression) {
        if identifier.get_identifier_token().span.end == self.offset {
            self.add_functions();
        }
    }

    fn visit_predefined_call_statement(&mut self, call: &PredefinedCallStatement) {
        walk_predefined_call_statement(self, call);
    }
}
