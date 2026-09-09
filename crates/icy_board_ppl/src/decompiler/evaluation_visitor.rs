use crate::{
    ast::{AstVisitor, AstVisitorMut, BinOp, BinaryExpression, Constant, ConstantExpression, Expression, UnaryExpression, UnaryOp, constant::NumberFormat},
    executable::{VariableType, VariableValue},
};

#[derive(Default)]
pub struct EvaluationVisitor {}

impl AstVisitor<Option<VariableValue>> for EvaluationVisitor {
    fn visit_constant_expression(&mut self, constant: &crate::ast::ConstantExpression) -> Option<VariableValue> {
        match constant.get_constant_value() {
            Constant::Boolean(b) => Some(VariableValue::new_bool(*b)),
            Constant::Integer(i, _) => Some(VariableValue::new_int(*i)),
            Constant::String(s) => Some(VariableValue::new_string(s.clone())),
            Constant::Double(f) => Some(VariableValue::new_double(*f)),
            Constant::Money(m) => Some(VariableValue::new_int(*m)),
            Constant::Unsigned(u, _) => Some(VariableValue::new_unsigned(*u)),
            Constant::Builtin(b) => Some(VariableValue::new_int(b.value)),
        }
    }

    fn visit_unary_expression(&mut self, unary: &crate::ast::UnaryExpression) -> Option<VariableValue> {
        if let Some(expr) = unary.get_expression().visit(self) {
            match unary.get_op() {
                UnaryOp::Not => Some(expr.not()),
                UnaryOp::Minus => Some(-expr),
                UnaryOp::Plus => Some(expr),
            }
        } else {
            None
        }
    }

    fn visit_binary_expression(&mut self, binary: &crate::ast::BinaryExpression) -> Option<VariableValue> {
        let left = binary.get_left_expression().visit(self);
        if let Some(result) = left.as_ref().and_then(|value| binary.get_op().short_circuit_result(value.as_bool())) {
            return Some(VariableValue::new_bool(result));
        }
        let right = binary.get_right_expression().visit(self);

        if left.is_none() || right.is_none() {
            if let Some(left_value) = &left {
                if binary.get_op() == BinOp::Div && left_value.as_int() == 0 {
                    return Some(VariableValue::new_int(0));
                }
                partial_evaluate(binary.get_op(), left_value)
            } else if let Some(right_value) = &right {
                partial_evaluate(binary.get_op(), right_value)
            } else {
                None
            }
        } else if let (Some(left_value), Some(right_value)) = (left, right) {
            match binary.get_op() {
                BinOp::Add => Some(left_value + right_value),
                BinOp::Sub => Some(left_value - right_value),
                BinOp::Mul => Some(left_value * right_value),
                BinOp::Div => Some(left_value / right_value),
                BinOp::Mod => Some(left_value % right_value),
                BinOp::PoW => Some(left_value.pow(right_value)),
                BinOp::Eq => Some(VariableValue::new_bool(left_value == right_value)),
                BinOp::NotEq => Some(VariableValue::new_bool(left_value != right_value)),
                BinOp::Or | BinOp::ShortOr => Some(VariableValue::new_bool(left_value.as_bool() || right_value.as_bool())),
                BinOp::And | BinOp::ShortAnd => Some(VariableValue::new_bool(left_value.as_bool() && right_value.as_bool())),
                BinOp::Lower => Some(VariableValue::new_bool(left_value < right_value)),
                BinOp::LowerEq => Some(VariableValue::new_bool(left_value <= right_value)),
                BinOp::Greater => Some(VariableValue::new_bool(left_value > right_value)),
                BinOp::GreaterEq => Some(VariableValue::new_bool(left_value >= right_value)),
            }
        } else {
            None
        }
    }
}

fn partial_evaluate(get_op: BinOp, val: &VariableValue) -> Option<VariableValue> {
    match get_op {
        BinOp::Mul => {
            if val.as_int() == 0 {
                return Some(VariableValue::new_int(0));
            }
        }
        BinOp::Or => {
            if val.as_bool() {
                return Some(VariableValue::new_bool(true));
            }
        }
        BinOp::And if !val.as_bool() => {
            return Some(VariableValue::new_bool(false));
        }
        _ => {}
    }
    None
}

#[derive(Default)]
pub struct OptimizationVisitor {}

impl AstVisitorMut for OptimizationVisitor {
    fn visit_unary_expression(&mut self, unary: &crate::ast::UnaryExpression) -> crate::ast::Expression {
        if let Some(value) = EvaluationVisitor::default().visit_unary_expression(unary)
            && let Some(value) = value_to_expression(&value)
        {
            return value;
        }
        Expression::Unary(UnaryExpression::empty(unary.get_op(), unary.get_expression().visit_mut(self)))
    }

    fn visit_binary_expression(&mut self, binary: &crate::ast::BinaryExpression) -> crate::ast::Expression {
        let left_value = binary.get_left_expression().visit(&mut EvaluationVisitor::default());
        let right_value = binary.get_right_expression().visit(&mut EvaluationVisitor::default());
        if left_value.is_none() || right_value.is_none() {
            let val = if let Some(val) = &left_value {
                val
            } else if let Some(val) = &right_value {
                val
            } else {
                return Expression::Binary(BinaryExpression::empty(
                    binary.get_left_expression().visit_mut(self),
                    binary.get_op(),
                    binary.get_right_expression().visit_mut(self),
                ));
            };

            match binary.get_op() {
                BinOp::Mul => {
                    if val.as_int() == 0 {
                        return ConstantExpression::create_empty_expression(Constant::Integer(0, NumberFormat::Default));
                    }
                }
                BinOp::Or => {
                    if val.as_bool() {
                        return ConstantExpression::create_empty_expression(Constant::Boolean(true));
                    }
                    if left_value.is_none() {
                        return binary.get_left_expression().visit_mut(self);
                    }
                    return binary.get_right_expression().visit_mut(self);
                }
                BinOp::And => {
                    if !val.as_bool() {
                        return ConstantExpression::create_empty_expression(Constant::Boolean(false));
                    }
                    if left_value.is_none() {
                        return binary.get_left_expression().visit_mut(self);
                    }
                    return binary.get_right_expression().visit_mut(self);
                }
                _ => {}
            }
        } else if let Some(value) = value_to_expression(&EvaluationVisitor::default().visit_binary_expression(binary).unwrap()) {
            return value;
        }
        let left = binary.get_left_expression().visit_mut(self);
        let right = binary.get_right_expression().visit_mut(self);
        Expression::Binary(BinaryExpression::empty(left, binary.get_op(), right))
    }
}

/// A named constant is left alone so that it still reads as its name once decompiled.
fn is_foldable_constant(expr: &Expression) -> bool {
    match expr {
        Expression::Const(constant) => !matches!(constant.get_constant_value(), Constant::Builtin(_)),
        Expression::Parens(parens) => is_foldable_constant(parens.get_expression()),
        Expression::Unary(unary) => is_foldable_constant(unary.get_expression()),
        Expression::Binary(binary) => is_foldable_constant(binary.get_left_expression()) && is_foldable_constant(binary.get_right_expression()),
        _ => false,
    }
}

/// Replaces subexpressions that are made of constants with the value they evaluate to.
/// Unlike `OptimizationVisitor` it touches nothing that still has an operand to evaluate,
/// because that operand may be a call that the program relies on for its side effect.
#[derive(Default)]
pub struct ConstantFolder {}

impl AstVisitorMut for ConstantFolder {
    fn visit_unary_expression(&mut self, unary: &crate::ast::UnaryExpression) -> Expression {
        if is_foldable_constant(unary.get_expression())
            && let Some(value) = EvaluationVisitor::default().visit_unary_expression(unary)
            && let Some(expr) = value_to_expression(&value)
        {
            return expr;
        }
        Expression::Unary(UnaryExpression::empty(unary.get_op(), unary.get_expression().visit_mut(self)))
    }

    fn visit_binary_expression(&mut self, binary: &crate::ast::BinaryExpression) -> Expression {
        if is_foldable_constant(binary.get_left_expression())
            && is_foldable_constant(binary.get_right_expression())
            && let Some(value) = EvaluationVisitor::default().visit_binary_expression(binary)
            && let Some(expr) = value_to_expression(&value)
        {
            return expr;
        }
        let left = binary.get_left_expression().visit_mut(self);
        let right = binary.get_right_expression().visit_mut(self);
        Expression::Binary(BinaryExpression::empty(left, binary.get_op(), right))
    }
}

fn value_to_expression(value: &VariableValue) -> Option<Expression> {
    match value.get_type() {
        VariableType::Boolean => return Some(ConstantExpression::create_empty_expression(Constant::Boolean(value.as_bool()))),
        VariableType::Integer => {
            return Some(ConstantExpression::create_empty_expression(Constant::Integer(
                value.as_int(),
                NumberFormat::Default,
            )));
        }
        VariableType::String => return Some(ConstantExpression::create_empty_expression(Constant::String(value.as_string()))),
        VariableType::Double => {
            return Some(ConstantExpression::create_empty_expression(Constant::Double(unsafe {
                value.data.double_value
            })));
        }
        VariableType::Unsigned => {
            return Some(ConstantExpression::create_empty_expression(Constant::Unsigned(
                unsafe { value.data.unsigned_value },
                crate::ast::constant::NumberFormat::Default,
            )));
        }
        _ => {}
    }
    None
}

/// Simplify only an IF's boolean context, never an arbitrary value expression.
/// This recognizes the constant direction guards emitted by legacy FOR loops
/// without treating e.g. `0.5 * value` as zero or dropping a call/getter/indexer.
/// An identity may expose a non-boolean operand here because IF converts the
/// result to boolean anyway; the same replacement in PRINT/LET would be wrong.
pub(super) fn simplify_condition(expression: &Expression) -> Expression {
    fn boolean(value: bool) -> Expression {
        ConstantExpression::create_empty_expression(Constant::Boolean(value))
    }
    fn truth(expression: &Expression) -> Option<bool> {
        match expression {
            Expression::Const(value) => Some(value.get_constant_value().get_value().as_bool()),
            _ => None,
        }
    }
    // Discard only plainly total operations. Arithmetic may overflow or fault;
    // calls, members and subscripts can have effects or raise runtime errors.
    fn scalar(expression: &Expression) -> bool {
        match expression {
            Expression::Const(_) | Expression::Identifier(_) => true,
            Expression::Parens(value) => scalar(value.get_expression()),
            _ => false,
        }
    }
    fn total(expression: &Expression) -> bool {
        if scalar(expression) {
            return true;
        }
        match expression {
            Expression::Parens(value) => total(value.get_expression()),
            Expression::Unary(value) if value.get_op() == UnaryOp::Not => total(value.get_expression()),
            Expression::Binary(value) => match value.get_op() {
                BinOp::And | BinOp::Or => total(value.get_left_expression()) && total(value.get_right_expression()),
                BinOp::Eq | BinOp::NotEq | BinOp::Lower | BinOp::LowerEq | BinOp::Greater | BinOp::GreaterEq => {
                    scalar(value.get_left_expression()) && scalar(value.get_right_expression())
                }
                _ => false,
            },
            _ => false,
        }
    }
    match expression {
        Expression::Parens(value) => simplify_condition(value.get_expression()),
        Expression::Unary(value) if value.get_op() == UnaryOp::Not => {
            let inner = simplify_condition(value.get_expression());
            if let Some(value) = truth(&inner) {
                boolean(!value)
            } else {
                let inner = if matches!(inner, Expression::Binary(_)) {
                    crate::ast::ParensExpression::create_empty_expression(inner)
                } else {
                    inner
                };
                UnaryExpression::create_empty_expression(UnaryOp::Not, inner)
            }
        }
        Expression::Binary(value) if matches!(value.get_op(), BinOp::And | BinOp::Or) => {
            let left = simplify_condition(value.get_left_expression());
            let right = simplify_condition(value.get_right_expression());
            let absorbing = value.get_op() == BinOp::Or;
            for (constant, other) in [(&left, &right), (&right, &left)] {
                if let Some(value) = truth(constant) {
                    if value != absorbing {
                        return other.clone();
                    }
                    if total(other) {
                        return boolean(absorbing);
                    }
                }
            }
            BinaryExpression::create_empty_expression(
                value.get_op(),
                super::add_parens_if_required(value.get_op(), left, false),
                super::add_parens_if_required(value.get_op(), right, true),
            )
        }
        _ => expression.clone(),
    }
}
