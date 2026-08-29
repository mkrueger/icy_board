use crate::{
    ast::{AstVisitor, Constant, Expression, constant::NumberFormat},
    executable::{FuncOpCode, PPEExpr, VariableType},
    semantic::SemanticInfo,
};

use super::PPECompiler;

pub struct ExpressionCompiler<'a> {
    pub compiler: &'a mut PPECompiler,
}

impl AstVisitor<PPEExpr> for ExpressionCompiler<'_> {
    fn visit_record_literal_expression(&mut self, record: &crate::ast::RecordLiteralExpression) -> PPEExpr {
        let VariableType::UserData(type_id) = record.get_variable_type() else {
            return PPEExpr::Value(0);
        };
        let fields = record
            .get_fields()
            .iter()
            .filter_map(|field| {
                let field_id = self
                    .compiler
                    .semantic_visitor
                    .type_registry
                    .record_field_index(type_id, field.get_identifier())?;
                Some((field_id, field.get_value().visit(self)))
            })
            .collect();
        PPEExpr::RecordLiteral(type_id, fields)
    }

    fn visit_identifier_expression(&mut self, identifier: &crate::ast::IdentifierExpression) -> PPEExpr {
        if let Some(decl) = self.compiler.lookup_table.lookup_variable(identifier.get_identifier()) {
            if decl.header.variable_type == VariableType::Function
                && self
                    .compiler
                    .semantic_visitor
                    .is_function_return_value(identifier.get_identifier_token().span.start)
            {
                return PPEExpr::Value(unsafe { decl.value.data.function_value.return_var as usize });
            }
            if self
                .compiler
                .semantic_visitor
                .is_routine_reference(identifier.get_identifier_token().span.start)
            {
                return PPEExpr::RoutineReference(decl.header.id);
            }
            return PPEExpr::Value(decl.header.id);
        }
        // A type name used as a receiver lowers to the call that hands its instance back.
        if let Some(provider) = self
            .compiler
            .semantic_visitor
            .instance_provider_lookup
            .get(&identifier.get_identifier_token().span.start)
        {
            return PPEExpr::PredefinedFunctionCall(provider.get_definition(), Vec::new());
        }
        // A type a static member was called on lowers to the receiver that member dispatches from.
        if let Some(type_id) = self
            .compiler
            .semantic_visitor
            .static_receiver_lookup
            .get(&identifier.get_identifier_token().span.start)
        {
            let type_id = self
                .compiler
                .lookup_table
                .lookup_constant(&crate::ast::Constant::Integer(i32::from(*type_id), crate::ast::constant::NumberFormat::Default));
            return PPEExpr::PredefinedFunctionCall(FuncOpCode::StaticReceiver.get_definition(), vec![PPEExpr::Value(type_id)]);
        }
        log::error!("Variable not found: {}", identifier.get_identifier());
        PPEExpr::Value(0)
    }

    fn visit_member_reference_expression(&mut self, member_reference_expression: &crate::ast::MemberReferenceExpression) -> PPEExpr {
        let base = member_reference_expression.get_expression().visit(self);
        // Semantic analysis has already reported why the member is unknown, so codegen
        // only has to avoid running into it.
        let Some(type_id) = self
            .compiler
            .semantic_visitor
            .user_type_lookup
            .get(&member_reference_expression.get_identifier_token().span.start)
        else {
            return PPEExpr::Value(0);
        };
        if let Some(member_id) = self
            .compiler
            .semantic_visitor
            .type_registry
            .record_field_index(*type_id, member_reference_expression.get_identifier())
        {
            return PPEExpr::Member(Box::new(base), member_id);
        }
        let Some(typ) = self.compiler.semantic_visitor.type_registry.get_type_from_id(*type_id) else {
            return PPEExpr::Value(0);
        };
        let Some(member_id) = typ.member_id_lookup.get(member_reference_expression.get_identifier()) else {
            return PPEExpr::Value(0);
        };

        PPEExpr::Member(Box::new(base), *member_id)
    }

    fn visit_constant_expression(&mut self, constant: &crate::ast::ConstantExpression) -> PPEExpr {
        let table_id = self.compiler.lookup_table.lookup_constant(constant.get_constant_value());
        PPEExpr::Value(table_id)
    }

    fn visit_binary_expression(&mut self, bin_expr: &crate::ast::BinaryExpression) -> PPEExpr {
        let left = bin_expr.get_left_expression().visit(self);
        let right = bin_expr.get_right_expression().visit(self);
        PPEExpr::BinaryExpression(bin_expr.get_op(), Box::new(left), Box::new(right))
    }

    fn visit_unary_expression(&mut self, unary: &crate::ast::UnaryExpression) -> PPEExpr {
        let expression = unary.get_expression().visit(self);
        PPEExpr::UnaryExpression(unary.get_op(), Box::new(expression))
    }

    fn visit_function_call_expression(&mut self, call: &crate::ast::FunctionCallExpression) -> PPEExpr {
        let arguments = call.get_arguments().iter().map(|e| e.visit(self)).collect();
        let Some(function_type) = self.compiler.semantic_visitor.function_type_lookup.get(&call.id).cloned() else {
            log::error!("function not found at: {} ({})", call.get_expression().get_span().start, call.get_expression());
            return PPEExpr::Value(0);
        };

        match function_type {
            SemanticInfo::PredefinedFunc(op_code) => {
                PPEExpr::PredefinedFunctionCall(
                    op_code.get_definition(), // to de-alias aliases
                    call.get_arguments().iter().map(|e| e.visit(self)).collect(),
                )
            }
            SemanticInfo::MemberFunctionCall(idx) => {
                let expr = call.get_expression().visit(self);
                PPEExpr::MemberFunctionCall(Box::new(expr), arguments, idx)
            }
            SemanticInfo::MemberSetterCall(idx) => {
                let Expression::MemberReference(member) = call.get_expression() else {
                    log::error!("member setter without a receiver at: {}", call.get_expression().get_span().start);
                    return PPEExpr::Value(0);
                };
                let arguments = match call.get_lpar_token().token {
                    crate::parser::lexer::Token::Eq => arguments,
                    ref token => {
                        let op = match token {
                            crate::parser::lexer::Token::AddAssign => crate::ast::BinOp::Add,
                            crate::parser::lexer::Token::SubAssign => crate::ast::BinOp::Sub,
                            crate::parser::lexer::Token::MulAssign => crate::ast::BinOp::Mul,
                            crate::parser::lexer::Token::DivAssign => crate::ast::BinOp::Div,
                            crate::parser::lexer::Token::ModAssign => crate::ast::BinOp::Mod,
                            crate::parser::lexer::Token::AndAssign => crate::ast::BinOp::And,
                            crate::parser::lexer::Token::OrAssign => crate::ast::BinOp::Or,
                            _ => return PPEExpr::Value(0),
                        };
                        vec![PPEExpr::BinaryExpression(
                            op,
                            Box::new(PPEExpr::Member(Box::new(member.get_expression().visit(self)), idx)),
                            Box::new(arguments.into_iter().next().unwrap_or(PPEExpr::Value(0))),
                        )]
                    }
                };
                PPEExpr::MemberFunctionCall(Box::new(member.get_expression().visit(self)), arguments, idx)
            }
            SemanticInfo::IndexedRecordField(idx) => {
                let Expression::MemberReference(member) = call.get_expression() else {
                    return PPEExpr::Value(0);
                };
                PPEExpr::IndexedMember(Box::new(member.get_expression().visit(self)), idx, arguments)
            }
            SemanticInfo::ArrayMemberFunc(op_code, defaults) => {
                // `a.Len(0)` is `Len(a, 0)`: the receiver leads, then what was written,
                // then whatever the member fills in for a left out argument.
                let Expression::MemberReference(member) = call.get_expression() else {
                    log::error!("array member call without a receiver at: {}", call.get_expression().get_span().start);
                    return PPEExpr::Value(0);
                };
                let mut call_arguments = vec![member.get_expression().visit(self)];
                call_arguments.extend(arguments);
                call_arguments.extend(
                    defaults
                        .iter()
                        .map(|value| PPEExpr::Value(self.compiler.lookup_table.lookup_constant(&Constant::Integer(*value, NumberFormat::Default)))),
                );
                PPEExpr::PredefinedFunctionCall(op_code.get_definition(), call_arguments)
            }
            SemanticInfo::StringMemberFunc(op_code, defaults) => {
                let Expression::MemberReference(member) = call.get_expression() else {
                    return PPEExpr::Value(0);
                };
                let mut call_arguments = vec![member.get_expression().visit(self)];
                call_arguments.extend(arguments);
                call_arguments.extend(
                    defaults
                        .iter()
                        .map(|value| PPEExpr::Value(self.compiler.lookup_table.lookup_constant(&Constant::Integer(*value, NumberFormat::Default)))),
                );
                PPEExpr::PredefinedFunctionCall(op_code.get_definition(), call_arguments)
            }
            SemanticInfo::StringStaticFunc(op_code) => PPEExpr::PredefinedFunctionCall(op_code.get_definition(), arguments),
            SemanticInfo::ArrayValueAt => {
                let Expression::MemberReference(member) = call.get_expression() else {
                    return PPEExpr::Value(0);
                };
                PPEExpr::PredefinedFunctionCall(
                    FuncOpCode::ArrayValueAt.get_definition(),
                    vec![member.get_expression().visit(self), arguments.into_iter().next().unwrap()],
                )
            }
            SemanticInfo::FunctionReference(idx) => {
                let reference_index = self.compiler.semantic_visitor.function_containers[idx].id;
                let table_index = self.compiler.semantic_visitor.references[reference_index].1.variable_table_index;
                PPEExpr::FunctionCall(table_index, arguments)
            }
            SemanticInfo::VariableReference(reference_index) => {
                let r = &self.compiler.semantic_visitor.references[reference_index];
                let table_index = r.1.variable_table_index;
                PPEExpr::Dim(table_index, arguments)
            }
            SemanticInfo::PredefFunctionGroup(_) => {
                log::error!("Invalid function call: {function_type:?}");
                PPEExpr::Value(0)
            }
            SemanticInfo::ArrayMemberProc(_) | SemanticInfo::RegexSplitProc { .. } => {
                log::error!("Array statement used where a value is expected: {function_type:?}");
                PPEExpr::Value(0)
            }
        }
    }

    fn visit_indexer_expression(&mut self, indexer: &crate::ast::IndexerExpression) -> PPEExpr {
        let arguments = indexer.get_arguments().iter().map(|e| e.visit(self)).collect();

        if self.compiler.lookup_table.has_variable(indexer.get_identifier()) {
            let Some(table_idx) = self.compiler.lookup_variable_index(indexer.get_identifier()) else {
                log::error!("function not found: {}", indexer.get_identifier());
                return PPEExpr::Value(0);
            };

            let var = self.compiler.lookup_table.variable_table.get_var_entry(table_idx);
            if var.value.get_type() == VariableType::Function {
                return PPEExpr::FunctionCall(var.header.id, arguments);
            }
            if var.header.dim == 0 && matches!(var.header.variable_type, VariableType::String | VariableType::BigStr) && arguments.len() == 1 {
                return PPEExpr::PredefinedFunctionCall(
                    FuncOpCode::StringCharAt.get_definition(),
                    vec![PPEExpr::Value(var.header.id), arguments.into_iter().next().unwrap()],
                );
            }
            if var.header.dim as usize != arguments.len() {
                log::error!("Invalid dimensions for function call: {}", indexer.get_identifier());
                return PPEExpr::Value(0);
            }
            return PPEExpr::Dim(var.header.id, arguments);
        }
        log::error!("Invalid indexer call: {}", indexer.get_identifier());
        PPEExpr::Value(0)
    }

    fn visit_parens_expression(&mut self, parens: &crate::ast::ParensExpression) -> PPEExpr {
        parens.get_expression().visit(self)
    }
}
