use unicase::Ascii;

use crate::{
    ast::{AstVisitor, BinOp, Constant, ConstantExpression, Expression, FunctionCallExpression, MemberReferenceExpression, UnaryOp, constant::NumberFormat},
    executable::{VariableType, VariableValue},
};

/// What a name is worth while compiling.
type ConstantLookup<'a> = &'a dyn Fn(&Ascii<String>) -> Option<VariableValue>;

/// What a member of a named type is worth while compiling.
type MemberLookup<'a> = &'a dyn Fn(&Ascii<String>, &Ascii<String>) -> Option<VariableValue>;

/// What a constant expression is worth, or `None` if it takes anything the compiler
/// cannot know. Names are asked for, so one constant may be written in terms of an
/// earlier one.
pub fn const_value(expr: &Expression, lookup: ConstantLookup<'_>) -> Option<VariableValue> {
    const_value_with_members(expr, lookup, &|_, _| None)
}

/// As `const_value`, but `Enum.Member` is worth what the member stands for.
pub fn const_value_with_members(expr: &Expression, lookup: ConstantLookup<'_>, member: MemberLookup<'_>) -> Option<VariableValue> {
    expr.visit(&mut ConstEvaluator { lookup, member })
}

/// The literal a value is written as, in the type its constant was declared with.
pub fn const_expression(value: &VariableValue, variable_type: VariableType) -> Option<Expression> {
    let value = if matches!(variable_type, VariableType::String | VariableType::BigStr) {
        value.clone()
    } else {
        value.clone().convert_to(variable_type)
    };
    let constant = match variable_type {
        VariableType::Boolean => Constant::Boolean(value.as_bool()),
        VariableType::String | VariableType::BigStr => Constant::String(value.as_string()),
        VariableType::Double | VariableType::Float => Constant::Double(value.as_double()),
        VariableType::Money => Constant::Money(value.as_int()),
        VariableType::Unsigned | VariableType::Byte | VariableType::Word | VariableType::DDate => {
            Constant::Unsigned(value.as_unsigned(), NumberFormat::Default)
        }
        VariableType::Integer | VariableType::SByte | VariableType::SWord | VariableType::Date | VariableType::EDate | VariableType::Time => {
            Constant::Integer(value.as_int(), NumberFormat::Default)
        }
        _ => return None,
    };
    Some(ConstantExpression::create_empty_expression(constant))
}

struct ConstEvaluator<'a> {
    lookup: ConstantLookup<'a>,
    member: MemberLookup<'a>,
}

impl AstVisitor<Option<VariableValue>> for ConstEvaluator<'_> {
    fn visit_identifier_expression(&mut self, identifier: &crate::ast::IdentifierExpression) -> Option<VariableValue> {
        (self.lookup)(identifier.get_identifier())
    }

    fn visit_member_reference_expression(&mut self, member: &MemberReferenceExpression) -> Option<VariableValue> {
        let Expression::Identifier(base) = member.get_expression() else {
            return None;
        };
        (self.member)(base.get_identifier(), member.get_identifier())
    }

    fn visit_constant_expression(&mut self, constant: &ConstantExpression) -> Option<VariableValue> {
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

    fn visit_function_call_expression(&mut self, call: &FunctionCallExpression) -> Option<VariableValue> {
        let Expression::Identifier(identifier) = call.get_expression() else {
            return None;
        };
        let arguments = call.get_arguments().iter().map(|argument| argument.visit(self)).collect::<Option<Vec<_>>>()?;
        let alpha = match identifier.get_identifier().as_ref().to_ascii_uppercase().as_str() {
            "RGB" if arguments.len() == 3 => 255,
            "RGB" if arguments.len() == 4 => arguments[3].as_int(),
            _ => return None,
        };
        Some(VariableValue::new_unsigned(u64::from(crate::icy_board::state::ppl_graphics::rgba_value(
            arguments[0].as_int(),
            arguments[1].as_int(),
            arguments[2].as_int(),
            alpha,
        ))))
    }
}
