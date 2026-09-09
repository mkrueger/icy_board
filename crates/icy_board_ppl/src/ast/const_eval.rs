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
    expr.visit(&mut ConstEvaluator { lookup, member, enums: &[] })
}

/// Nominal constant evaluation, including checked-cast operands. Domain errors
/// are diagnosed separately at every source operation, never just at the root.
pub fn const_enum_value(expr: &Expression, lookup: ConstantLookup<'_>, enums: &[crate::parser::EnumDefinition]) -> Option<VariableValue> {
    expr.visit(&mut ConstEvaluator {
        lookup,
        enums,
        member: &|name, member| {
            let definition = enums.iter().find(|definition| definition.name == *name)?;
            Some(VariableValue::new_enum(
                VariableType::UserData(definition.id),
                definition.value(member)?,
                definition.domain[0],
            ))
        },
    })
}

/// The one declaration conversion shared by direct semantic analysis and lowering.
/// Only source 400 CONST declarations check numeric bounds; ordinary assignments
/// and older source versions retain their historical conversions.
pub fn convert_const_declaration(value: VariableValue, declared: VariableType, language: u16) -> Option<VariableValue> {
    if matches!(declared, VariableType::UserData(_)) {
        return (value.vtype == declared).then_some(value);
    }
    if matches!(value.vtype, VariableType::UserData(_)) {
        return None;
    }
    // The parser retains the source STRING name until runtime type lowering.
    // Do not reintroduce the classic 256-character limit for source 400 CONSTs.
    if language >= 400 && declared == VariableType::String {
        return Some(value.convert_to(VariableType::UnboundedString));
    }
    if language >= 400 {
        if matches!(declared, VariableType::Float | VariableType::Double) {
            let number = value.as_double();
            if !number.is_finite() || (declared == VariableType::Float && number.abs() > f32::MAX as f64) {
                return None;
            }
        }
        let bounds = match declared {
            VariableType::Byte => Some((0, u8::MAX as i128)),
            VariableType::SByte => Some((i8::MIN as i128, i8::MAX as i128)),
            VariableType::Word => Some((0, u16::MAX as i128)),
            VariableType::SWord => Some((i16::MIN as i128, i16::MAX as i128)),
            VariableType::Integer => Some((i32::MIN as i128, i32::MAX as i128)),
            VariableType::Unsigned => Some((0, u32::MAX as i128)),
            VariableType::Long => Some((i64::MIN as i128, i64::MAX as i128)),
            VariableType::ULong => Some((0, u64::MAX as i128)),
            // MONEY uses signed 32-bit cents. Text has its own currency parser;
            // do not reinterpret dollar strings as ordinary integer strings.
            VariableType::Money if !matches!(value.vtype, VariableType::String | VariableType::BigStr | VariableType::UnboundedString) => {
                Some((i32::MIN as i128, i32::MAX as i128))
            }
            _ => None,
        };
        if let Some((min, max)) = bounds {
            let number = if matches!(value.vtype, VariableType::Float | VariableType::Double) {
                let number = value.as_double();
                // Test the untruncated sign too: a negative unsigned initializer
                // must not silently become zero. Upper bounds are exclusive so
                // rounded f64 representations of i64/u64 maxima cannot overflow.
                if !number.is_finite() || number < min as f64 || number >= (max + 1) as f64 {
                    return None;
                }
                number.trunc() as i128
            } else {
                integer_constant(&value)?
            };
            if !(min..=max).contains(&number) {
                return None;
            }
            return Some(match declared {
                VariableType::Unsigned => VariableValue::new_unsigned(number as u64),
                VariableType::Long if value.as_str().is_none() => VariableValue::new_long(number as i64),
                VariableType::ULong if value.as_str().is_none() => VariableValue::new_ulong(number as u64),
                // Range validation is not a new rounding/string-parsing policy.
                _ => value.convert_to(declared),
            });
        }
    }
    Some(value.convert_to(declared))
}

fn integer_constant(value: &VariableValue) -> Option<i128> {
    match value.vtype {
        VariableType::Unsigned | VariableType::ULong => Some(value.as_unsigned() as i128),
        VariableType::Long => Some(value.as_long() as i128),
        VariableType::String | VariableType::BigStr | VariableType::UnboundedString => {
            let text = value.as_string();
            let text = text.trim_start();
            let (negative, digits) = if let Some(rest) = text.strip_prefix('-') {
                (true, rest)
            } else {
                (false, text.strip_prefix('+').unwrap_or(text))
            };
            let mut number = 0i128;
            for digit in digits.chars().take_while(char::is_ascii_digit) {
                number = number.checked_mul(10)?.checked_add(digit.to_digit(10)? as i128)?;
            }
            Some(if negative { -number } else { number })
        }
        VariableType::Float | VariableType::Double | VariableType::UserData(_) => None,
        _ => value.try_as_int().map(i128::from),
    }
}

/// The literal a value is written as, in the type its constant was declared with.
pub fn const_expression(value: &VariableValue, variable_type: VariableType) -> Option<Expression> {
    let value = if matches!(variable_type, VariableType::String | VariableType::BigStr | VariableType::UnboundedString) {
        value.clone()
    } else {
        value.clone().convert_to(variable_type)
    };
    // Preserve the declared type, rather than turning SWORD into INTEGER etc.
    // This matters for nominal enum casts and all typed argument checks.
    let conversion = match variable_type {
        VariableType::Byte => Some("ToByte"),
        VariableType::SByte => Some("ToSByte"),
        VariableType::Word => Some("ToWord"),
        VariableType::SWord => Some("ToSWord"),
        VariableType::Float => Some("ToReal"),
        VariableType::Double => Some("ToDReal"),
        VariableType::Long => Some("ToLong"),
        VariableType::ULong => Some("ToULong"),
        VariableType::Date => Some("ToDate"),
        VariableType::EDate => Some("ToEDate"),
        VariableType::DDate => Some("ToDDate"),
        VariableType::Time => Some("ToTime"),
        _ => None,
    };
    if let Some(conversion) = conversion {
        let literal = match variable_type {
            // Decimal literals are REAL (f32) in bytecode. A DOUBLE constant
            // needs a string conversion to preserve its f64 precision/range.
            VariableType::Double => Constant::String(format!("{:e}", value.as_double())),
            VariableType::Long | VariableType::ULong => Constant::String(value.as_string()),
            VariableType::Float => Constant::Double(value.as_double()),
            _ => Constant::Integer(value.as_int(), NumberFormat::Default),
        };
        return Some(FunctionCallExpression::create_empty_expression(
            crate::ast::IdentifierExpression::create_empty_expression(Ascii::new(conversion.to_string())),
            vec![ConstantExpression::create_empty_expression(literal)],
        ));
    }
    let constant = match variable_type {
        VariableType::Boolean => Constant::Boolean(value.as_bool()),
        VariableType::String | VariableType::BigStr | VariableType::UnboundedString => Constant::String(value.as_string()),
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
    enums: &'a [crate::parser::EnumDefinition],
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
            Constant::Money(_) => Some(constant.get_constant_value().get_value()),
            Constant::Unsigned(u, _) => Some(VariableValue::new_unsigned(*u)),
            Constant::Builtin(b) => Some(VariableValue::new_int(b.value)),
        }
    }

    fn visit_unary_expression(&mut self, unary: &crate::ast::UnaryExpression) -> Option<VariableValue> {
        let value = unary.get_expression().visit(self)?;
        if matches!(value.get_type(), VariableType::UserData(_)) {
            return None;
        }
        // Large unsigned literals need a signed representation before checking
        // LONG's lower bound. Do not erase narrow integer/Boolean operand types.
        if unary.get_op() == UnaryOp::Minus
            && matches!(
                value.vtype,
                VariableType::Integer | VariableType::Unsigned | VariableType::Long | VariableType::ULong
            )
            && let Some(number) = integer_constant(&value)
        {
            let number = number.checked_neg()?;
            return if !matches!(value.vtype, VariableType::Long | VariableType::ULong)
                && let Ok(number) = i32::try_from(number)
            {
                Some(VariableValue::new_int(number))
            } else {
                i64::try_from(number).ok().map(VariableValue::new_long)
            };
        }
        Some(match unary.get_op() {
            UnaryOp::Not => value.not(),
            UnaryOp::Minus => -value,
            UnaryOp::Plus => value,
        })
    }

    fn visit_binary_expression(&mut self, binary: &crate::ast::BinaryExpression) -> Option<VariableValue> {
        let left = binary.get_left_expression().visit(self)?;
        if !matches!(left.get_type(), VariableType::UserData(_))
            && let Some(result) = binary.get_op().short_circuit_result(left.as_bool())
        {
            return Some(VariableValue::new_bool(result));
        }
        let right = binary.get_right_expression().visit(self)?;
        if matches!(left.get_type(), VariableType::UserData(_)) || matches!(right.get_type(), VariableType::UserData(_)) {
            if left.get_type() != right.get_type() {
                return None;
            }
            return match binary.get_op() {
                BinOp::And | BinOp::Or => Some(VariableValue::new_enum(
                    left.get_type(),
                    if binary.get_op() == BinOp::And {
                        left.as_int() & right.as_int()
                    } else {
                        left.as_int() | right.as_int()
                    },
                    left.emptied().as_int(),
                )),
                BinOp::Eq => Some(VariableValue::new_bool(left.as_int() == right.as_int())),
                BinOp::NotEq => Some(VariableValue::new_bool(left.as_int() != right.as_int())),
                _ => None,
            };
        }
        Some(match binary.get_op() {
            BinOp::Add => left + right,
            BinOp::Sub => left - right,
            BinOp::Mul => left * right,
            BinOp::Div => left / right,
            BinOp::Mod => left % right,
            BinOp::PoW => left.pow(right),
            BinOp::Eq => VariableValue::new_bool(left == right),
            BinOp::NotEq => VariableValue::new_bool(left != right),
            BinOp::Or | BinOp::ShortOr => VariableValue::new_bool(left.as_bool() || right.as_bool()),
            BinOp::And | BinOp::ShortAnd => VariableValue::new_bool(left.as_bool() && right.as_bool()),
            BinOp::Lower => VariableValue::new_bool(left < right),
            BinOp::LowerEq => VariableValue::new_bool(left <= right),
            BinOp::Greater => VariableValue::new_bool(left > right),
            BinOp::GreaterEq => VariableValue::new_bool(left >= right),
        })
    }

    fn visit_function_call_expression(&mut self, call: &FunctionCallExpression) -> Option<VariableValue> {
        if let Expression::MemberReference(member) = call.get_expression()
            && *member.get_identifier() == "Has"
            && let [mask] = call.get_arguments().as_slice()
        {
            let receiver = member.get_expression().visit(self)?;
            let mask = mask.visit(self)?;
            if self.enums.iter().any(|definition| VariableType::UserData(definition.id) == receiver.get_type()) && receiver.get_type() == mask.get_type() {
                return Some(VariableValue::new_bool((receiver.as_int() & mask.as_int()) == mask.as_int()));
            }
            return None;
        }
        let Expression::Identifier(identifier) = call.get_expression() else {
            return None;
        };
        let arguments = call.get_arguments().iter().map(|argument| argument.visit(self)).collect::<Option<Vec<_>>>()?;
        if let Some(definition) = self.enums.iter().find(|definition| definition.name == *identifier.get_identifier()) {
            return match arguments.as_slice() {
                [value] if value.get_type() == VariableType::Integer => Some(VariableValue::new_enum(
                    VariableType::UserData(definition.id),
                    value.as_int(),
                    definition.domain[0],
                )),
                _ => None,
            };
        }
        let name = identifier.get_identifier().as_ref().to_ascii_uppercase();
        // TOINTEGER is the only ordinary function allowed to erase an enum's
        // nominal type. Folding must not introduce alternate cast/RGB bypasses.
        if name != "TOINTEGER" && arguments.iter().any(|argument| matches!(argument.vtype, VariableType::UserData(_))) {
            return None;
        }
        let alpha = match name.as_str() {
            "TOINTEGER" if arguments.len() == 1 => return Some(arguments[0].clone().convert_to(VariableType::Integer)),
            "TOSWORD" if arguments.len() == 1 => return Some(arguments[0].clone().convert_to(VariableType::SWord)),
            "TOSBYTE" if arguments.len() == 1 => return Some(arguments[0].clone().convert_to(VariableType::SByte)),
            "TOWORD" if arguments.len() == 1 => return Some(arguments[0].clone().convert_to(VariableType::Word)),
            "TOBYTE" if arguments.len() == 1 => return Some(arguments[0].clone().convert_to(VariableType::Byte)),
            "TOREAL" if arguments.len() == 1 => return Some(arguments[0].clone().convert_to(VariableType::Float)),
            "TODREAL" if arguments.len() == 1 => return Some(arguments[0].clone().convert_to(VariableType::Double)),
            "TOLONG" if arguments.len() == 1 => return Some(arguments[0].clone().convert_to(VariableType::Long)),
            "TOULONG" if arguments.len() == 1 => return Some(arguments[0].clone().convert_to(VariableType::ULong)),
            "RGB" if arguments.len() == 3 => 255,
            "RGB" if arguments.len() == 4 => arguments[3].as_int(),
            _ => return None,
        };
        Some(VariableValue::new_unsigned(u64::from(crate::color::rgba_value(
            arguments[0].as_int(),
            arguments[1].as_int(),
            arguments[2].as_int(),
            alpha,
        ))))
    }
}
