use icy_ppe::{
    ast::{walk_predefined_call_statement, Ast, AstVisitor},
    executable::{StatementSignature, FUNCTION_DEFINITIONS, STATEMENT_DEFINITIONS},
};
use tower_lsp::lsp_types::CompletionItem;

use icy_ppe::semantic::{self, SemanticVisitor};

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
const TYPES: [&str; 26] = [
    "BOOLEAN", "DATE", "DDATE", "INTEGER", "SDWORD", "LONG", "MONEY", "STRING", "TIME", "BIGSTR",
    "EDATE", "REAL", "FLOAT", "DREAL", "DOUBLE", "UNSIGNED", "DWORD", "UDWORD", "BYTE", "UBYTE",
    "WORD", "UWORD", "SBYTE", "SHORT", "SWORD", "INT",
];

/// return (need_to_continue_search, founded reference)
pub fn get_completion(ast: &Ast, offset: usize) -> Vec<CompletionItem> {
    let mut map = CompletionVisitor::new(offset);
    let mut semantic_visitor = SemanticVisitor::default();
    ast.visit(&mut semantic_visitor);

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
                kind: Some(tower_lsp::lsp_types::CompletionItemKind::KEYWORD),
                insert_text_format: Some(tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }

        for stmt in &STATEMENT_DEFINITIONS {
            if stmt.sig == StatementSignature::Invalid {
                continue;
            }
            map.items.push(CompletionItem {
                label: stmt.name.to_string(),
                insert_text: Some(stmt.name.to_string()),
                kind: Some(tower_lsp::lsp_types::CompletionItemKind::FUNCTION),
                insert_text_format: Some(tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }

        for (rt, r) in semantic_visitor.references {
            if matches!(rt, semantic::ReferenceType::Procedure(_)) {
                if let Some(decl) = &r.declaration {
                    map.items.push(CompletionItem {
                        label: decl.token.to_string(),
                        insert_text: Some(decl.token.to_string()),
                        kind: Some(tower_lsp::lsp_types::CompletionItemKind::FUNCTION),
                        insert_text_format: Some(
                            tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT,
                        ),
                        ..Default::default()
                    });
                }
            }
            if matches!(rt, semantic::ReferenceType::Variable(_)) {
                if let Some(decl) = &r.declaration {
                    map.items.push(CompletionItem {
                        label: decl.token.to_string(),
                        insert_text: Some(decl.token.to_string()),
                        kind: Some(tower_lsp::lsp_types::CompletionItemKind::VARIABLE),
                        insert_text_format: Some(
                            tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT,
                        ),
                        ..Default::default()
                    });
                }
            }
        }
    } else {
        for (rt, r) in &semantic_visitor.references {
            if matches!(rt, semantic::ReferenceType::Function(_)) {
                if let Some(decl) = &r.declaration {
                    map.items.push(CompletionItem {
                        label: decl.token.to_string(),
                        insert_text: Some(decl.token.to_string()),
                        kind: Some(tower_lsp::lsp_types::CompletionItemKind::FUNCTION),
                        insert_text_format: Some(
                            tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT,
                        ),
                        ..Default::default()
                    });
                }
            }

            if matches!(rt, semantic::ReferenceType::Variable(_)) {
                if let Some(decl) = &r.declaration {
                    map.items.push(CompletionItem {
                        label: decl.token.to_string(),
                        insert_text: Some(decl.token.to_string()),
                        kind: Some(tower_lsp::lsp_types::CompletionItemKind::VARIABLE),
                        insert_text_format: Some(
                            tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT,
                        ),
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
        Self {
            offset,
            items: Vec::new(),
        }
    }

    fn add_functions(&mut self) {
        for stmt in &FUNCTION_DEFINITIONS {
            self.items.push(CompletionItem {
                label: stmt.name.to_string(),
                insert_text: Some(stmt.name.to_string()),
                kind: Some(tower_lsp::lsp_types::CompletionItemKind::FUNCTION),
                insert_text_format: Some(tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }
    }
}

impl AstVisitor<()> for CompletionVisitor {
    fn visit_identifier_expression(&mut self, identifier: &icy_ppe::ast::IdentifierExpression) {
        if identifier.get_identifier_token().span.end == self.offset {
            self.add_functions();
        }
    }

    fn visit_predefined_call_statement(&mut self, call: &icy_ppe::ast::PredefinedCallStatement) {
        walk_predefined_call_statement(self, call);
    }
}
