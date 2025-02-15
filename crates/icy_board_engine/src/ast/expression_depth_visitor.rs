use super::{AstVisitor, BinaryExpression, ConstantExpression, FunctionCallExpression, ParensExpression, UnaryExpression};

#[derive(Default)]
pub struct ExpressionDepthVisitor {}

impl AstVisitor<usize> for ExpressionDepthVisitor {
    fn visit_identifier_expression(&mut self, _: &super::IdentifierExpression) -> usize {
        0
    }

    fn visit_member_reference_expression(&mut self, _: &super::MemberReferenceExpression) -> usize {
        1
    }

    fn visit_constant_expression(&mut self, _: &ConstantExpression) -> usize {
        1
    }

    fn visit_binary_expression(&mut self, binary: &BinaryExpression) -> usize {
        1 + binary.get_left_expression().visit(self) + binary.get_right_expression().visit(self)
    }

    fn visit_unary_expression(&mut self, unary: &UnaryExpression) -> usize {
        1 + unary.get_expression().visit(self)
    }

    fn visit_function_call_expression(&mut self, _: &FunctionCallExpression) -> usize {
        1
    }

    fn visit_indexer_expression(&mut self, _: &super::IndexerExpression) -> usize {
        1
    }

    fn visit_parens_expression(&mut self, parens: &ParensExpression) -> usize {
        parens.get_expression().visit(self)
    }
}
