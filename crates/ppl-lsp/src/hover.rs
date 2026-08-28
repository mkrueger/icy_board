//! Hover for the names a program declares itself - variables, routines, labels,
//! record types and their fields. The built-ins are answered by `documentation`.

use icy_board_engine::{
    ast::{Ast, AstVisitor, MemberReferenceExpression, VariableDeclarationStatement, walk_variable_declaration_statement},
    executable::VariableType,
    semantic::{ReferenceType, SemanticVisitor},
};
use tower_lsp::lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind};

use crate::{
    documentation::get_member_documentation,
    signature_help::routine_signature,
    type_lookup::{record_field_type_name, type_name, type_of_member},
};
use std::fmt::Write as _;

fn hover(text: String) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value: format!("```PPL\n{text}\n```"),
        }),
        range: None,
    }
}

fn documented_hover(signature: String, documentation: Option<String>) -> Hover {
    let value = match documentation {
        Some(documentation) => format!("```PPL\n{signature}\n```\n\n{documentation}"),
        None => format!("```PPL\n{signature}\n```"),
    };
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::Markdown,
            value,
        }),
        range: None,
    }
}

/// What a name the program declared is, in one line.
pub fn get_user_hover(ast: &Ast, visitor: &SemanticVisitor, offset: usize) -> Option<Hover> {
    for (reference_type, reference) in &visitor.references {
        if !reference.contains_pos(&ast.file_name, offset) {
            continue;
        }
        let name = reference
            .declaration
            .as_ref()
            .or(reference.implementation.as_ref())
            .map(|(_, decl)| decl.token.clone())
            .or_else(|| reference.usages.first().map(|(_, usage)| usage.token.clone()))?;

        return match reference_type {
            ReferenceType::Function(_) | ReferenceType::Procedure(_) => Some(hover(routine_signature(visitor, &name)?)),
            ReferenceType::Label(_) => Some(hover(format!(":{}", name.trim_start_matches(':')))),
            ReferenceType::Variable(_) => {
                let mut text = format!("{} {}", type_name(&visitor.type_registry, reference.variable_type), name);
                if let Some(header) = &reference.header {
                    let dimensions = match header.dim {
                        1 => vec![header.vector_size],
                        2 => vec![header.vector_size, header.matrix_size],
                        3 => vec![header.vector_size, header.matrix_size, header.cube_size],
                        _ => Vec::new(),
                    };
                    if !dimensions.is_empty() {
                        let _ = write!(text, "({})", dimensions.iter().map(usize::to_string).collect::<Vec<_>>().join(", "));
                    }
                }
                Some(hover(text))
            }
            _ => None,
        };
    }

    let mut member_visitor = MemberHoverVisitor { visitor, offset, hover: None };
    ast.visit(&mut member_visitor);
    member_visitor.hover
}

/// Answers a record type where it is named and a field where it is used.
struct MemberHoverVisitor<'a> {
    visitor: &'a SemanticVisitor,
    offset: usize,
    hover: Option<Hover>,
}

impl<'a> MemberHoverVisitor<'a> {
    fn record_hover(&self, var_type: VariableType) -> Option<Hover> {
        let VariableType::UserData(id) = var_type else {
            return None;
        };
        let definition = self.visitor.type_registry.get_user_type_from_id(id)?;
        let mut text = format!("TYPE {}", definition.name);
        for (name, field) in &definition.fields {
            let _ = write!(text, "\n    {} {}", record_field_type_name(&self.visitor.type_registry, *field), name);
        }
        text.push_str("\nENDTYPE");
        Some(hover(text))
    }
}

impl<'a> AstVisitor<()> for MemberHoverVisitor<'a> {
    fn visit_member_reference_expression(&mut self, member: &MemberReferenceExpression) {
        member.get_expression().visit(self);
        let token = member.get_identifier_token();
        if !token.span.contains(&self.offset) {
            return;
        }
        let Some(type_id) = self.visitor.user_type_lookup.get(&token.span.start) else {
            return;
        };
        let object = VariableType::UserData(*type_id);
        if let Some(field_type) = type_of_member(&self.visitor.type_registry, object, member.get_identifier().as_ref()) {
            let signature = format!(
                "{} {}.{}",
                type_name(&self.visitor.type_registry, field_type),
                type_name(&self.visitor.type_registry, object),
                member.get_identifier()
            );
            self.hover = Some(documented_hover(signature, get_member_documentation(object, member.get_identifier().as_ref())));
        }
    }

    fn visit_variable_declaration_statement(&mut self, declaration: &VariableDeclarationStatement) {
        if declaration.get_type_token().span.contains(&self.offset) {
            self.hover = self.record_hover(declaration.get_variable_type());
            if self.hover.is_some() {
                return;
            }
        }
        walk_variable_declaration_statement(self, declaration);
    }
}
