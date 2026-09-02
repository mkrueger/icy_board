use crate::{
    executable::{ExpressionNegator, OnErrorTarget, PPECommand, PPEExpr},
    hir::{HirCommand, HirErrorTarget, HirExpr},
};

pub fn lower_expression(expression: &HirExpr) -> PPEExpr {
    match expression {
        HirExpr::Invalid => PPEExpr::Value(0),
        HirExpr::Variable(id) => PPEExpr::Value(id.0),
        HirExpr::Constant(id) => PPEExpr::Value(id.0),
        HirExpr::RoutineReference(id) => PPEExpr::RoutineReference(id.0),
        HirExpr::RecordLiteral(type_id, fields) => {
            PPEExpr::RecordLiteral(type_id.0, fields.iter().map(|(member, value)| (member.0, lower_expression(value))).collect())
        }
        HirExpr::Member(base, member) => PPEExpr::Member(Box::new(lower_expression(base)), member.0),
        HirExpr::IndexedMember(base, member, dimensions) => {
            PPEExpr::IndexedMember(Box::new(lower_expression(base)), member.0, dimensions.iter().map(lower_expression).collect())
        }
        HirExpr::Unary(op, expression) => PPEExpr::UnaryExpression(*op, Box::new(lower_expression(expression))),
        HirExpr::Binary(op, left, right) => PPEExpr::BinaryExpression(*op, Box::new(lower_expression(left)), Box::new(lower_expression(right))),
        HirExpr::Dim(variable, dimensions) => PPEExpr::Dim(variable.0, dimensions.iter().map(lower_expression).collect()),
        HirExpr::PredefinedCall(opcode, arguments) => {
            PPEExpr::PredefinedFunctionCall(opcode.get_definition(), arguments.iter().map(lower_expression).collect())
        }
        HirExpr::FunctionCall(routine, arguments) => PPEExpr::FunctionCall(routine.0, arguments.iter().map(lower_expression).collect()),
        HirExpr::MemberCall(receiver, arguments, member) => {
            PPEExpr::MemberFunctionCall(Box::new(lower_expression(receiver)), arguments.iter().map(lower_expression).collect(), member.0)
        }
    }
}

pub fn lower_command(command: &HirCommand) -> PPECommand {
    match command {
        HirCommand::End => PPECommand::End,
        HirCommand::EndFunction => PPECommand::EndFunc,
        HirCommand::EndProcedure => PPECommand::EndProc,
        HirCommand::Return => PPECommand::Return,
        HirCommand::Goto(label) => PPECommand::Goto(label.0),
        HirCommand::Gosub(label) => PPECommand::Gosub(label.0),
        HirCommand::OnError(target) => PPECommand::OnError(match target {
            HirErrorTarget::Off => OnErrorTarget::Off,
            HirErrorTarget::Goto(label) => OnErrorTarget::Goto(label.0),
            HirErrorTarget::Gosub(label) => OnErrorTarget::Gosub(label.0),
            HirErrorTarget::Procedure(routine) => OnErrorTarget::Procedure(routine.0),
        }),
        HirCommand::ConditionalGoto(condition, label) => {
            let condition = lower_expression(condition).visit_mut(&mut ExpressionNegator::default());
            PPECommand::IfNot(Box::new(condition), label.0)
        }
        HirCommand::Let(target, value) => PPECommand::Let(Box::new(lower_expression(target)), Box::new(lower_expression(value))),
        HirCommand::MemberCall(expression) => PPECommand::MemberCall(Box::new(lower_expression(expression))),
        HirCommand::PredefinedCall(opcode, arguments) => PPECommand::PredefinedCall(opcode.get_definition(), arguments.iter().map(lower_expression).collect()),
        HirCommand::ProcedureCall(routine, arguments) => PPECommand::ProcedureCall(routine.0, arguments.iter().map(lower_expression).collect()),
        HirCommand::ForEach(variable, collection, end) => PPECommand::ForEach(variable.0, Box::new(lower_expression(collection)), end.0),
        HirCommand::NextForEach(start) => PPECommand::NextForEach(start.0),
    }
}

#[cfg(test)]
mod tests {
    use super::lower_expression;
    use crate::{
        ast::BinOp,
        executable::PPEExpr,
        hir::{HirExpr, MemberId, UserTypeId},
    };

    #[test]
    fn typed_ids_lower_only_at_the_executable_boundary() {
        let expression = HirExpr::RecordLiteral(
            UserTypeId(101),
            vec![(
                MemberId(2),
                HirExpr::Binary(BinOp::Add, Box::new(HirExpr::variable(7)), Box::new(HirExpr::constant(8))),
            )],
        );

        assert_eq!(
            PPEExpr::RecordLiteral(
                101,
                vec![(
                    2,
                    PPEExpr::BinaryExpression(BinOp::Add, Box::new(PPEExpr::Value(7)), Box::new(PPEExpr::Value(8)))
                )]
            ),
            lower_expression(&expression)
        );
    }
}
