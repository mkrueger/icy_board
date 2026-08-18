use icy_board_engine::{
    ast::{Ast, AstVisitor, IdentifierExpression, PredefinedCallStatement, constant::BUILTIN_CONSTS, walk_predefined_call_statement},
    executable::{FUNCTION_DEFINITIONS, STATEMENT_DEFINITIONS, StatementSignature},
    parser::{FIRST_BOARD_OBJECT_LANGUAGE_VERSION, built_in_type_names, lexer::KEYWORDS},
    semantic::{ReferenceType, SemanticVisitor},
};
use tower_lsp::lsp_types::{CompletionItem, CompletionItemKind, Documentation, HoverContents, InsertTextFormat};

use crate::{
    context::{CursorContext, cursor_context},
    documentation::{get_const_hover, get_function_hover, get_statement_hover},
    type_lookup::{MemberKind, members_of, type_name, type_of_chain},
};

pub enum ImCompleteCompletionItem {
    Variable(String),
    Function(String, Vec<String>),
}

/// Words the parser reads by name instead of as a token, with the version that gave
/// them their meaning. EXIT is the statement END used to be.
const CONTEXTUAL_WORDS: &[(&str, u16)] = &[("EXIT", 400)];

/// return (need_to_continue_search, founded reference)
pub fn get_completion(ast: &Ast, semantic_visitor: &SemanticVisitor, line_before_cursor: &str, offset: usize) -> Vec<CompletionItem> {
    match cursor_context(line_before_cursor) {
        CursorContext::Nothing => return Vec::new(),
        CursorContext::Member(path) => return member_completion(semantic_visitor, &path),
        CursorContext::RecordLiteralField { type_name, named_fields } => {
            return record_literal_completion(semantic_visitor, &type_name, &named_fields);
        }
        CursorContext::Other => {}
    }

    let mut map = CompletionVisitor::new(offset);
    ast.visit(&mut map);

    if map.items.is_empty() {
        for keyword in KEYWORDS.iter().filter(|keyword| keyword.since <= ast.language_version) {
            map.items.push(CompletionItem {
                label: keyword.name.to_ascii_uppercase(),
                insert_text: Some(keyword.name.to_ascii_uppercase()),
                kind: Some(tower_lsp::lsp_types::CompletionItemKind::KEYWORD),
                insert_text_format: Some(tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }
        for (word, _) in CONTEXTUAL_WORDS.iter().filter(|(_, since)| *since <= ast.language_version) {
            map.items.push(CompletionItem {
                label: word.to_string(),
                insert_text: Some(word.to_string()),
                kind: Some(tower_lsp::lsp_types::CompletionItemKind::KEYWORD),
                insert_text_format: Some(tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }
        for stmt in built_in_type_names(ast.language_version) {
            map.items.push(CompletionItem {
                label: stmt.to_string(),
                insert_text: Some(stmt.to_string()),
                kind: Some(tower_lsp::lsp_types::CompletionItemKind::CLASS),
                insert_text_format: Some(tower_lsp::lsp_types::InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }

        for name in declared_type_names(semantic_visitor, ast.language_version) {
            map.items.push(CompletionItem {
                label: name.clone(),
                insert_text: Some(name),
                kind: Some(CompletionItemKind::STRUCT),
                insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
                ..Default::default()
            });
        }

        for stmt in STATEMENT_DEFINITIONS.iter() {
            if stmt.sig == StatementSignature::Invalid || stmt.version > ast.language_version {
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

        for (rt, r) in &semantic_visitor.references {
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

/// The names of the record types the program declares plus, from the version that
/// brought them, the board objects.
fn declared_type_names(visitor: &SemanticVisitor, lang_version: u16) -> Vec<String> {
    let mut names: Vec<String> = visitor.type_registry.user_types().iter().map(|def| def.name.to_string()).collect();
    if lang_version >= FIRST_BOARD_OBJECT_LANGUAGE_VERSION {
        names.extend(visitor.type_registry.registered_types.keys().map(|name| name.to_string()));
    }
    names.sort();
    names
}

/// What may follow the `.` of a member chain.
fn member_completion(visitor: &SemanticVisitor, path: &[String]) -> Vec<CompletionItem> {
    let Some(var_type) = type_of_chain(visitor, path) else {
        return Vec::new();
    };
    members_of(&visitor.type_registry, var_type)
        .into_iter()
        .map(|member| CompletionItem {
            label: member.name.clone(),
            insert_text: Some(member.name),
            kind: Some(match member.kind {
                MemberKind::Field => CompletionItemKind::FIELD,
                MemberKind::Method => CompletionItemKind::METHOD,
            }),
            detail: Some(member.detail),
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
            ..Default::default()
        })
        .collect()
}

/// The fields a record literal has not named yet.
fn record_literal_completion(visitor: &SemanticVisitor, type_name_of_literal: &str, named_fields: &[String]) -> Vec<CompletionItem> {
    let Some(def) = visitor.type_registry.get_user_type(&unicase::Ascii::new(type_name_of_literal.to_string())) else {
        return Vec::new();
    };
    def.fields
        .iter()
        .filter(|(name, _)| !named_fields.iter().any(|named| named.eq_ignore_ascii_case(name.as_ref())))
        .map(|(name, field_type)| CompletionItem {
            label: name.to_string(),
            insert_text: Some(format!("{name} = ")),
            kind: Some(CompletionItemKind::FIELD),
            detail: Some(type_name(&visitor.type_registry, *field_type)),
            insert_text_format: Some(InsertTextFormat::PLAIN_TEXT),
            ..Default::default()
        })
        .collect()
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
