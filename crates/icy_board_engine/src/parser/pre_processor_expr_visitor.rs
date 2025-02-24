use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
};

use crate::{
    ast::{AstVisitor, BinOp, Constant, ConstantExpression, UnaryOp},
    executable::VariableValue,
};

use super::{
    lexer::{Spanned, Token},
    ErrorReporter,
};

pub struct PreProcessorVisitor<'a> {
    pub define_table: &'a HashMap<unicase::Ascii<String>, Constant>,
    pub errors: Arc<Mutex<ErrorReporter>>,
}

impl<'a> AstVisitor<Option<VariableValue>> for PreProcessorVisitor<'a> {
    fn visit_constant_expression(&mut self, constant: &crate::ast::ConstantExpression) -> Option<VariableValue> {
        match constant.get_constant_value() {
            Constant::Boolean(b) => Some(VariableValue::new_bool(*b)),
            Constant::Integer(i, _) => Some(VariableValue::new_int(*i)),
            Constant::String(s) => Some(VariableValue::new_string(s.clone())),
            Constant::Double(f) => Some(VariableValue::new_double(*f)),
            Constant::Money(m) => Some(VariableValue::new_int(*m)),
            Constant::Unsigned(u) => Some(VariableValue::new_unsigned(*u)),
            _ => None,
        }
    }

    fn visit_identifier_expression(&mut self, identifier: &crate::ast::IdentifierExpression) -> Option<VariableValue> {
        if let Some(value) = self.define_table.get(identifier.get_identifier()) {
            self.visit_constant_expression(&ConstantExpression::new(Spanned::new(Token::Eol, 0..0), value.clone()))
        } else {
            None
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
                BinOp::Or => Some(VariableValue::new_bool(left_value.as_bool() || right_value.as_bool())),
                BinOp::And => Some(VariableValue::new_bool(left_value.as_bool() && right_value.as_bool())),
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
