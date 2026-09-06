use crate::{
    ast::{AstVisitorMut, Constant, ConstantExpression, Expression, FunctionCallExpression, MemberReferenceExpression},
    parser::UserTypeRegistry,
};

pub struct EnumLoweringVisitor<'a> {
    registry: &'a UserTypeRegistry,
}

impl<'a> EnumLoweringVisitor<'a> {
    pub fn new(registry: &'a UserTypeRegistry) -> Self {
        Self { registry }
    }
}

impl AstVisitorMut for EnumLoweringVisitor<'_> {
    fn visit_binary_expression(&mut self, binary: &crate::ast::BinaryExpression) -> Expression {
        let mut lowered = binary.clone();
        *lowered.get_left_expression_mut() = binary.get_left_expression().visit_mut(self);
        *lowered.get_right_expression_mut() = binary.get_right_expression().visit_mut(self);
        Expression::Binary(lowered)
    }

    fn visit_function_call_expression(&mut self, call: &FunctionCallExpression) -> Expression {
        Expression::FunctionCall(call.preserving_id(
            call.get_expression().visit_mut(self),
            call.get_arguments().iter().map(|argument| argument.visit_mut(self)).collect(),
        ))
    }

    fn visit_member_reference_expression(&mut self, member: &MemberReferenceExpression) -> Expression {
        if let Expression::Identifier(base) = member.get_expression()
            && let Some(definition) = self.registry.get_enum(base.get_identifier())
            && let Some(value) = definition.value(member.get_identifier())
        {
            return ConstantExpression::create_empty_expression(Constant::Integer(value, crate::ast::constant::NumberFormat::Default));
        }
        Expression::MemberReference(MemberReferenceExpression::new(
            member.get_expression().visit_mut(self),
            member.get_dot_token().clone(),
            member.get_identifier_token().clone(),
        ))
    }
}
