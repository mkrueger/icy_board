use unicase::Ascii;

use crate::{
    ast::{AstVisitor, BinOp, Constant, ConstantExpression, Expression, UnaryOp, constant::NumberFormat},
    executable::{VariableType, VariableValue},
};

/// What a constant expression is worth, or `None` if it takes anything the compiler
/// cannot know. Names are asked for, so one constant may be written in terms of an
/// earlier one.
pub fn const_value(expr: &Expression, lookup: &dyn Fn(&Ascii<String>) -> Option<VariableValue>) -> Option<VariableValue> {
    expr.visit(&mut ConstEvaluator { lookup })
}

/// The literal a value is written as, in the type its constant was declared with.
pub fn const_expression(value: &VariableValue, variable_type: VariableType) -> Option<Expression> {
    let value = value.clone().convert_to(variable_type);
    let constant = match variable_type {
        VariableType::Boolean => Constant::Boolean(value.as_bool()),
        VariableType::String | VariableType::BigStr => Constant::String(value.as_string()),
        VariableType::Double | VariableType::Float => Constant::Double(value.as_double()),
        VariableType::Money => Constant::Money(value.as_int()),
        VariableType::Unsigned | VariableType::Byte | VariableType::Word | VariableType::DDate => Constant::Unsigned(value.as_unsigned()),
        VariableType::Integer | VariableType::SByte | VariableType::SWord | VariableType::Date | VariableType::EDate | VariableType::Time => {
            Constant::Integer(value.as_int(), NumberFormat::Default)
        }
        _ => return None,
    };
    Some(ConstantExpression::create_empty_expression(constant))
}

struct ConstEvaluator<'a> {
    lookup: &'a dyn Fn(&Ascii<String>) -> Option<VariableValue>,
}

impl AstVisitor<Option<VariableValue>> for ConstEvaluator<'_> {
    fn visit_identifier_expression(&mut self, identifier: &crate::ast::IdentifierExpression) -> Option<VariableValue> {
        (self.lookup)(identifier.get_identifier())
    }

    fn visit_constant_expression(&mut self, constant: &ConstantExpression) -> Option<VariableValue> {
        match constant.get_constant_value() {
            Constant::Boolean(b) => Some(VariableValue::new_bool(*b)),
            Constant::Integer(i, _) => Some(VariableValue::new_int(*i)),
            Constant::String(s) => Some(VariableValue::new_string(s.clone())),
            Constant::Double(f) => Some(VariableValue::new_double(*f)),
            Constant::Money(m) => Some(VariableValue::new_int(*m)),
            Constant::Unsigned(u) => Some(VariableValue::new_unsigned(*u)),
            Constant::Builtin(b) => Some(VariableValue::new_int(b.value)),
        }
    }

    fn visit_unary_expression(&mut self, unary: &crate::ast::UnaryExpression) -> Option<VariableValue> {
        let value = unary.get_expression().visit(self)?;
        Some(match unary.get_op() {
            UnaryOp::Not => value.not(),
            UnaryOp::Minus => -value,
            UnaryOp::Plus => value,
        })
    }

    fn visit_binary_expression(&mut self, binary: &crate::ast::BinaryExpression) -> Option<VariableValue> {
        let left = binary.get_left_expression().visit(self)?;
        let right = binary.get_right_expression().visit(self)?;
        Some(match binary.get_op() {
            BinOp::Add => left + right,
            BinOp::Sub => left - right,
            BinOp::Mul => left * right,
            BinOp::Div => left / right,
            BinOp::Mod => left % right,
            BinOp::PoW => left.pow(right),
            BinOp::Eq => VariableValue::new_bool(left == right),
            BinOp::NotEq => VariableValue::new_bool(left != right),
            BinOp::Or => VariableValue::new_bool(left.as_bool() || right.as_bool()),
            BinOp::And => VariableValue::new_bool(left.as_bool() && right.as_bool()),
            BinOp::Lower => VariableValue::new_bool(left < right),
            BinOp::LowerEq => VariableValue::new_bool(left <= right),
            BinOp::Greater => VariableValue::new_bool(left > right),
            BinOp::GreaterEq => VariableValue::new_bool(left >= right),
        })
    }
}
