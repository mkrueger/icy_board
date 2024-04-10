use super::{
    AstVisitor, BinaryExpression, ConstantExpression, FunctionCallExpression, ParensExpression,
    PredefinedFunctionCallExpression, UnaryExpression,
};

#[derive(Default)]
pub struct ExpressionDepthVisitor {}

impl AstVisitor<usize> for ExpressionDepthVisitor {
    fn visit_identifier_expression(&mut self, _: &super::IdentifierExpression) -> usize {
        1
    }

    fn visit_constant_expression(&mut self, _: &ConstantExpression) -> usize {
        1
    }

    fn visit_binary_expression(&mut self, binary: &BinaryExpression) -> usize {
        1 + binary
            .get_left_expression()
            .visit(self)
            .max(binary.get_right_expression().visit(self))
    }

    fn visit_unary_expression(&mut self, unary: &UnaryExpression) -> usize {
        1 + unary.get_expression().visit(self)
    }

    fn visit_predefined_function_call_expression(
        &mut self,
        _: &PredefinedFunctionCallExpression,
    ) -> usize {
        1
    }

    fn visit_function_call_expression(&mut self, _: &FunctionCallExpression) -> usize {
        1
    }

    fn visit_parens_expression(&mut self, parens: &ParensExpression) -> usize {
        1 + parens.get_expression().visit(self)
    }
}
