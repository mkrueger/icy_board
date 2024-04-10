use super::{
    AstVisitorMut, BinaryExpression, Constant, ConstantExpression, Expression,
    FunctionCallExpression, ParensExpression, PredefinedFunctionCallExpression, UnaryExpression,
    UnaryOp,
};

#[derive(Default)]
pub struct NegateExpressionVisitor {}

impl AstVisitorMut for NegateExpressionVisitor {
    fn visit_identifier_expression(&mut self, expr: &super::IdentifierExpression) -> Expression {
        UnaryExpression::create_empty_expression(UnaryOp::Not, Expression::Identifier(expr.clone()))
    }

    fn visit_constant_expression(&mut self, expr: &ConstantExpression) -> Expression {
        match expr.get_constant_value() {
            Constant::Boolean(b) => {
                ConstantExpression::create_empty_expression(Constant::Boolean(!b))
            }
            _ => UnaryExpression::create_empty_expression(
                UnaryOp::Not,
                Expression::Const(expr.clone()),
            ),
        }
    }

    fn visit_predefined_function_call_expression(
        &mut self,
        expr: &PredefinedFunctionCallExpression,
    ) -> Expression {
        UnaryExpression::create_empty_expression(
            UnaryOp::Not,
            Expression::PredefinedFunctionCall(expr.clone()),
        )
    }

    fn visit_function_call_expression(&mut self, expr: &FunctionCallExpression) -> Expression {
        UnaryExpression::create_empty_expression(
            UnaryOp::Not,
            Expression::FunctionCall(expr.clone()),
        )
    }

    fn visit_parens_expression(&mut self, expr: &ParensExpression) -> Expression {
        UnaryExpression::create_empty_expression(UnaryOp::Not, Expression::Parens(expr.clone()))
    }

    fn visit_unary_expression(&mut self, unary: &UnaryExpression) -> Expression {
        let expr = unary.get_expression().clone();
        if unary.get_op() == UnaryOp::Not {
            expr
        } else {
            UnaryExpression::create_empty_expression(UnaryOp::Not, expr)
        }
    }

    fn visit_binary_expression(&mut self, expr: &super::BinaryExpression) -> Expression {
        match expr.get_op() {
            super::BinOp::PoW
            | super::BinOp::Mul
            | super::BinOp::Div
            | super::BinOp::Mod
            | super::BinOp::Add
            | super::BinOp::Sub => UnaryExpression::create_empty_expression(
                UnaryOp::Not,
                Expression::Binary(expr.clone()),
            ),

            super::BinOp::Eq => BinaryExpression::create_empty_expression(
                super::BinOp::NotEq,
                expr.get_left_expression().clone(),
                expr.get_right_expression().clone(),
            ),
            super::BinOp::NotEq => BinaryExpression::create_empty_expression(
                super::BinOp::Eq,
                expr.get_left_expression().clone(),
                expr.get_right_expression().clone(),
            ),
            super::BinOp::Lower => BinaryExpression::create_empty_expression(
                super::BinOp::GreaterEq,
                expr.get_left_expression().clone(),
                expr.get_right_expression().clone(),
            ),
            super::BinOp::LowerEq => BinaryExpression::create_empty_expression(
                super::BinOp::Greater,
                expr.get_left_expression().clone(),
                expr.get_right_expression().clone(),
            ),
            super::BinOp::Greater => BinaryExpression::create_empty_expression(
                super::BinOp::LowerEq,
                expr.get_left_expression().clone(),
                expr.get_right_expression().clone(),
            ),

            super::BinOp::GreaterEq => BinaryExpression::create_empty_expression(
                super::BinOp::Lower,
                expr.get_left_expression().clone(),
                expr.get_right_expression().clone(),
            ),
            super::BinOp::And => BinaryExpression::create_empty_expression(
                super::BinOp::Or,
                expr.get_left_expression()
                    .visit_mut(&mut NegateExpressionVisitor::default()),
                expr.get_right_expression()
                    .visit_mut(&mut NegateExpressionVisitor::default()),
            ),
            super::BinOp::Or => BinaryExpression::create_empty_expression(
                super::BinOp::And,
                expr.get_left_expression()
                    .visit_mut(&mut NegateExpressionVisitor::default()),
                expr.get_right_expression()
                    .visit_mut(&mut NegateExpressionVisitor::default()),
            ),
        }
    }
}
