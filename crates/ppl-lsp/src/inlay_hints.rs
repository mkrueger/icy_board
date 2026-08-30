//! Optional parameter-name inlay hints for PPL 400 object API calls.

use icy_board_engine::{
    ast::{Ast, AstVisitor, Expression, FunctionCallExpression, walk_function_call_expression},
    executable::VariableType,
    semantic::SemanticVisitor,
};
use ropey::Rope;
use tower_lsp::lsp_types::{InlayHint, InlayHintKind, InlayHintLabel, Range};

use crate::{offset_to_position, type_lookup::static_type_of_name};

struct HintVisitor<'a> {
    semantic: &'a SemanticVisitor,
    rope: &'a Rope,
    range: Range,
    hints: Vec<InlayHint>,
}

impl HintVisitor<'_> {
    fn receiver_type(&self, member: &icy_board_engine::ast::MemberReferenceExpression) -> Option<VariableType> {
        self.semantic
            .member_receiver_type_lookup
            .get(&member.get_identifier_token().span.start)
            .copied()
            .or_else(|| {
                let Expression::Identifier(identifier) = member.get_expression() else {
                    return None;
                };
                static_type_of_name(self.semantic, identifier.get_identifier().as_ref())
            })
    }
}

impl AstVisitor<()> for HintVisitor<'_> {
    fn visit_function_call_expression(&mut self, call: &FunctionCallExpression) {
        if let Expression::MemberReference(member) = call.get_expression()
            && let Some(VariableType::UserData(id)) = self.receiver_type(member)
            && let Some(object) = self.semantic.type_registry.get_type_from_id(id)
            && let Some(function) = object.functions.get(member.get_identifier())
        {
            for (argument, name) in call.get_arguments().iter().zip(&function.parameter_names) {
                let Some(position) = offset_to_position(argument.get_span().start, self.rope) else {
                    continue;
                };
                if position < self.range.start || position > self.range.end {
                    continue;
                }
                self.hints.push(InlayHint {
                    position,
                    label: InlayHintLabel::String(format!("{name}:")),
                    kind: Some(InlayHintKind::PARAMETER),
                    text_edits: None,
                    tooltip: None,
                    padding_left: None,
                    padding_right: Some(true),
                    data: None,
                });
            }
        }
        walk_function_call_expression(self, call);
    }
}

pub fn get_inlay_hints(ast: &Ast, semantic: &SemanticVisitor, rope: &Rope, range: Range) -> Vec<InlayHint> {
    let mut visitor = HintVisitor {
        semantic,
        rope,
        range,
        hints: Vec::new(),
    };
    ast.visit(&mut visitor);
    visitor.hints
}
