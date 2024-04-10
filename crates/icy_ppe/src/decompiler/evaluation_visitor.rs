use crate::{
    ast::{
        AstVisitor, AstVisitorMut, BinOp, BinaryExpression, Constant, ConstantExpression,
        Expression, UnaryExpression, UnaryOp,
    },
    executable::{VariableType, VariableValue},
};

#[derive(Default)]
pub struct EvaluationVisitor {}

impl AstVisitor<Option<VariableValue>> for EvaluationVisitor {
    fn visit_constant_expression(
        &mut self,
        constant: &crate::ast::ConstantExpression,
    ) -> Option<VariableValue> {
        match constant.get_constant_value() {
            Constant::Boolean(b) => Some(VariableValue::new_bool(*b)),
            Constant::Integer(i) => Some(VariableValue::new_int(*i)),
            Constant::String(s) => Some(VariableValue::new_string(s.clone())),
            Constant::Double(f) => Some(VariableValue::new_double(*f)),
            Constant::Money(m) => Some(VariableValue::new_int(*m)),
            Constant::Unsigned(u) => Some(VariableValue::new_unsigned(*u)),
            Constant::Builtin(b) => Some(VariableValue::new_int(b.value)),
        }
    }

    fn visit_unary_expression(
        &mut self,
        unary: &crate::ast::UnaryExpression,
    ) -> Option<VariableValue> {
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

    fn visit_binary_expression(
        &mut self,
        binary: &crate::ast::BinaryExpression,
    ) -> Option<VariableValue> {
        let left = binary.get_left_expression().visit(self);
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
                BinOp::Or => Some(VariableValue::new_bool(
                    left_value.as_bool() || right_value.as_bool(),
                )),
                BinOp::And => Some(VariableValue::new_bool(
                    left_value.as_bool() && right_value.as_bool(),
                )),
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
        BinOp::And => {
            if !val.as_bool() {
                return Some(VariableValue::new_bool(false));
            }
        }
        _ => {}
    }
    None
}

#[derive(Default)]
pub struct OptimizationVisitor {}

impl AstVisitorMut for OptimizationVisitor {
    fn visit_unary_expression(
        &mut self,
        unary: &crate::ast::UnaryExpression,
    ) -> crate::ast::Expression {
        if let Some(value) = EvaluationVisitor::default().visit_unary_expression(unary) {
            if let Some(value) = value_to_expression(&value) {
                return value;
            }
        }
        Expression::Unary(UnaryExpression::empty(
            unary.get_op(),
            unary.get_expression().visit_mut(self),
        ))
    }

    fn visit_binary_expression(
        &mut self,
        binary: &crate::ast::BinaryExpression,
    ) -> crate::ast::Expression {
        let left_value = binary
            .get_left_expression()
            .visit(&mut EvaluationVisitor::default());
        let right_value = binary
            .get_right_expression()
            .visit(&mut EvaluationVisitor::default());
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
                        return ConstantExpression::create_empty_expression(Constant::Integer(0));
                    }
                }
                BinOp::Or => {
                    if val.as_bool() {
                        return ConstantExpression::create_empty_expression(Constant::Boolean(
                            true,
                        ));
                    }
                    if left_value.is_none() {
                        return binary.get_left_expression().visit_mut(self);
                    }
                    return binary.get_right_expression().visit_mut(self);
                }
                BinOp::And => {
                    if !val.as_bool() {
                        return ConstantExpression::create_empty_expression(Constant::Boolean(
                            false,
                        ));
                    }
                    if left_value.is_none() {
                        return binary.get_left_expression().visit_mut(self);
                    }
                    return binary.get_right_expression().visit_mut(self);
                }
                _ => {}
            }
        } else if let Some(value) = value_to_expression(
            &EvaluationVisitor::default()
                .visit_binary_expression(binary)
                .unwrap(),
        ) {
            return value;
        }
        let left = binary.get_left_expression().visit_mut(self);
        let right = binary.get_right_expression().visit_mut(self);
        Expression::Binary(BinaryExpression::empty(left, binary.get_op(), right))
    }
}

fn value_to_expression(value: &VariableValue) -> Option<Expression> {
    match value.get_type() {
        VariableType::Boolean => {
            return Some(ConstantExpression::create_empty_expression(
                Constant::Boolean(value.as_bool()),
            ))
        }
        VariableType::Integer => {
            return Some(ConstantExpression::create_empty_expression(
                Constant::Integer(value.as_int()),
            ))
        }
        VariableType::String => {
            return Some(ConstantExpression::create_empty_expression(
                Constant::String(value.as_string()),
            ))
        }
        VariableType::Double => {
            return Some(ConstantExpression::create_empty_expression(
                Constant::Double(unsafe { value.data.double_value }),
            ))
        }
        VariableType::Unsigned => {
            return Some(ConstantExpression::create_empty_expression(
                Constant::Unsigned(unsafe { value.data.unsigned_value }),
            ))
        }
        _ => {}
    }
    None
}
