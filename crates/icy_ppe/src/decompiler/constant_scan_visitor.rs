use crate::{
    ast::{
        constant::{ConstantType, BUILTIN_CONSTS},
        AstVisitorMut, BinOp, BinaryExpression, Constant, ConstantExpression, Expression,
        PredefinedCallStatement, PredefinedFunctionCallExpression, Statement, UnaryExpression,
        UnaryOp,
    },
    executable::{FuncOpCode, OpCode},
};
struct ConstantReplaceVisitor {
    replace_category: ConstantType,
}
impl ConstantReplaceVisitor {
    pub fn new(replace_category: ConstantType) -> Self {
        Self { replace_category }
    }
}
impl AstVisitorMut for ConstantReplaceVisitor {
    fn visit_constant_expression(&mut self, constant: &ConstantExpression) -> Expression {
        if let Constant::Integer(i) = constant.get_constant_value() {
            let mut expression = None;
            //check non 0 value built-in constants combinations
            for c in &BUILTIN_CONSTS {
                if c.value != 0
                    && (c.value & *i) == c.value
                    && c.used_by.contains(&self.replace_category)
                {
                    if expression.is_some() {
                        expression = Some(BinaryExpression::create_empty_expression(
                            BinOp::Add,
                            expression.unwrap(),
                            ConstantExpression::create_empty_expression(Constant::Builtin(c)),
                        ));
                    } else {
                        expression = Some(ConstantExpression::create_empty_expression(
                            Constant::Builtin(c),
                        ));
                    }
                }
            }

            // search for 0 value built-in constants
            if expression.is_none() {
                for c in &BUILTIN_CONSTS {
                    if c.value == 0 && c.used_by.contains(&self.replace_category) {
                        expression = Some(ConstantExpression::create_empty_expression(
                            Constant::Builtin(c),
                        ));
                        break;
                    }
                }
            }

            if let Some(expr) = expression {
                return expr;
            }
        }
        Expression::Const(constant.clone())
    }
}

/// Used to scan for constants and replace them with their respective built-in constants in decompiled output.
#[derive(Default)]
pub struct ConstantScanVisitor {}
impl ConstantScanVisitor {
    fn strip_parens(get_expression: &Expression) -> &Expression {
        let mut condition = get_expression;
        while let Expression::Parens(expr) = condition {
            condition = expr.get_expression();
        }
        condition
    }
}

impl AstVisitorMut for ConstantScanVisitor {
    fn visit_predefined_call_statement(&mut self, call: &PredefinedCallStatement) -> Statement {
        let mut args: Vec<Expression> = Vec::new();

        let op = call.get_func().opcode;

        let is_conf_flag_call = [OpCode::CONFFLAG, OpCode::CONFUNFLAG].contains(&op);
        let is_fopen_call = [OpCode::FCREATE, OpCode::FOPEN, OpCode::FAPPEND].contains(&op);

        for (i, arg) in call.get_arguments().iter().enumerate() {
            let new_arg = if is_conf_flag_call && i == 1 {
                arg.visit_mut(&mut ConstantReplaceVisitor::new(ConstantType::ConfFlag))
            } else if op == OpCode::DISPFILE && i == 1 {
                arg.visit_mut(&mut ConstantReplaceVisitor::new(ConstantType::DispFile))
            } else if is_fopen_call && i == 3 {
                arg.visit_mut(&mut ConstantReplaceVisitor::new(ConstantType::FileSec))
            } else if is_fopen_call && i == 2 {
                arg.visit_mut(&mut ConstantReplaceVisitor::new(ConstantType::FileAccess))
            } else if op == OpCode::STARTDISP && i == 0 {
                arg.visit_mut(&mut ConstantReplaceVisitor::new(ConstantType::StartDisp))
            } else if op == OpCode::INPUTSTR && i == 5 || op == OpCode::PROMPTSTR && i == 4 {
                arg.visit_mut(&mut ConstantReplaceVisitor::new(ConstantType::InputStr))
            } else if op == OpCode::DISPTEXT && i == 1 {
                arg.visit_mut(&mut ConstantReplaceVisitor::new(ConstantType::DispText))
            } else if (op == OpCode::ACCOUNT || op == OpCode::RECORDUSAGE) && i == 0 {
                arg.visit_mut(&mut ConstantReplaceVisitor::new(ConstantType::Account))
            } else if op == OpCode::FSEEK && i == 0 {
                arg.visit_mut(&mut ConstantReplaceVisitor::new(ConstantType::FileSeek))
            } else {
                arg.visit_mut(self)
            };
            args.push(new_arg);
        }
        Statement::PredifinedCall(PredefinedCallStatement::empty(call.get_func(), args))
    }

    fn visit_condition(&mut self, condition: &Expression) -> Expression {
        if let Expression::Parens(expr) = condition {
            return expr.get_expression().visit_mut(self);
        }
        condition.visit_mut(self)
    }

    fn visit_unary_expression(&mut self, unary: &crate::ast::UnaryExpression) -> Expression {
        if unary.get_op() == UnaryOp::Not {
            if let Expression::Unary(u) = Self::strip_parens(unary.get_expression()) {
                if u.get_op() == UnaryOp::Not {
                    return Self::strip_parens(u.get_expression()).visit_mut(self);
                }
            } else if let Expression::Const(c) = unary.get_expression() {
                if let Constant::Boolean(b) = c.get_constant_value() {
                    return ConstantExpression::create_empty_expression(Constant::Boolean(!b));
                }
            }
        }

        let expr = unary.get_expression().visit_mut(self);
        UnaryExpression::create_empty_expression(unary.get_op(), expr)
    }

    fn visit_predefined_function_call_expression(
        &mut self,
        call: &crate::ast::PredefinedFunctionCallExpression,
    ) -> Expression {
        let mut args: Vec<Expression> = Vec::new();

        let op = call.get_func().opcode;

        for (i, arg) in call.get_arguments().iter().enumerate() {
            let new_arg = if op == FuncOpCode::CRC32 && i == 0 {
                arg.visit_mut(&mut ConstantReplaceVisitor::new(ConstantType::Crc))
            } else if op == FuncOpCode::PCBACCOUNT && i == 0 {
                arg.visit_mut(&mut ConstantReplaceVisitor::new(ConstantType::PcbAccount))
            } else if op == FuncOpCode::SCANMSGHDR && i == 2 {
                arg.visit_mut(&mut ConstantReplaceVisitor::new(ConstantType::ScanMsgHdr))
            } else {
                arg.visit_mut(self)
            };
            args.push(new_arg);
        }

        Expression::PredefinedFunctionCall(PredefinedFunctionCallExpression::empty(
            call.get_func(),
            args,
        ))
    }
}
