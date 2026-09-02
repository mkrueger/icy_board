use crate::{
    ast::{AstVisitor, Constant, Expression, constant::NumberFormat},
    executable::{FuncOpCode, VariableType},
    hir::{CallId, HirExpr, MemberId, UserTypeId},
    semantic::SemanticInfo,
};

use super::PPECompiler;

pub struct HirExpressionResolver<'a> {
    pub compiler: &'a mut PPECompiler,
}

impl AstVisitor<HirExpr> for HirExpressionResolver<'_> {
    fn visit_record_literal_expression(&mut self, record: &crate::ast::RecordLiteralExpression) -> HirExpr {
        let VariableType::UserData(type_id) = record.get_variable_type() else {
            return HirExpr::Invalid;
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
                Some((MemberId(field_id), field.get_value().visit(self)))
            })
            .collect();
        HirExpr::RecordLiteral(UserTypeId(type_id), fields)
    }

    fn visit_identifier_expression(&mut self, identifier: &crate::ast::IdentifierExpression) -> HirExpr {
        if let Some(decl) = self.compiler.lookup_table.lookup_variable(identifier.get_identifier()) {
            if decl.header.variable_type == VariableType::Function
                && self
                    .compiler
                    .semantic_visitor
                    .is_function_return_value(identifier.get_identifier_token().span.start)
            {
                return HirExpr::variable(unsafe { decl.value.data.function_value.return_var as usize });
            }
            if self
                .compiler
                .semantic_visitor
                .is_routine_reference(identifier.get_identifier_token().span.start)
            {
                return HirExpr::routine_reference(decl.header.id);
            }
            return HirExpr::variable(decl.header.id);
        }
        // A type name used as a receiver lowers to the call that hands its instance back.
        if let Some(provider) = self
            .compiler
            .semantic_visitor
            .instance_provider_lookup
            .get(&identifier.get_identifier_token().span.start)
        {
            return HirExpr::predefined(*provider, Vec::new());
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
            return HirExpr::predefined(FuncOpCode::StaticReceiver, vec![HirExpr::constant(type_id)]);
        }
        log::error!("Variable not found: {}", identifier.get_identifier());
        HirExpr::Invalid
    }

    fn visit_member_reference_expression(&mut self, member_reference_expression: &crate::ast::MemberReferenceExpression) -> HirExpr {
        let base = member_reference_expression.get_expression().visit(self);
        // Semantic analysis has already reported why the member is unknown, so codegen
        // only has to avoid running into it.
        let Some(type_id) = self
            .compiler
            .semantic_visitor
            .user_type_lookup
            .get(&member_reference_expression.get_identifier_token().span.start)
        else {
            return HirExpr::Invalid;
        };
        if let Some(member_id) = self
            .compiler
            .semantic_visitor
            .type_registry
            .record_field_index(*type_id, member_reference_expression.get_identifier())
        {
            return HirExpr::member(base, member_id);
        }
        let Some(typ) = self.compiler.semantic_visitor.type_registry.get_type_from_id(*type_id) else {
            return HirExpr::Invalid;
        };
        let Some(member_id) = typ.member_id_lookup.get(member_reference_expression.get_identifier()) else {
            return HirExpr::Invalid;
        };

        HirExpr::member(base, *member_id)
    }

    fn visit_constant_expression(&mut self, constant: &crate::ast::ConstantExpression) -> HirExpr {
        let table_id = self.compiler.lookup_table.lookup_constant(constant.get_constant_value());
        HirExpr::constant(table_id)
    }

    fn visit_binary_expression(&mut self, bin_expr: &crate::ast::BinaryExpression) -> HirExpr {
        let left = bin_expr.get_left_expression().visit(self);
        let right = bin_expr.get_right_expression().visit(self);
        HirExpr::Binary(bin_expr.get_op(), Box::new(left), Box::new(right))
    }

    fn visit_unary_expression(&mut self, unary: &crate::ast::UnaryExpression) -> HirExpr {
        let expression = unary.get_expression().visit(self);
        HirExpr::Unary(unary.get_op(), Box::new(expression))
    }

    fn visit_function_call_expression(&mut self, call: &crate::ast::FunctionCallExpression) -> HirExpr {
        let arguments = call.get_arguments().iter().map(|e| e.visit(self)).collect();
        let Some(function_type) = self.compiler.semantic_visitor.function_type_lookup.get(&CallId(call.id)).cloned() else {
            log::error!("function not found at: {} ({})", call.get_expression().get_span().start, call.get_expression());
            return HirExpr::Invalid;
        };

        match function_type {
            SemanticInfo::PredefinedFunc(op_code) | SemanticInfo::ScalarStaticFunc(op_code) => HirExpr::predefined(op_code, arguments),
            SemanticInfo::MemberFunctionCall(idx) => HirExpr::member_call(call.get_expression().visit(self), arguments, idx),
            SemanticInfo::MemberSetterCall(idx) => {
                let Expression::MemberReference(member) = call.get_expression() else {
                    log::error!("member setter without a receiver at: {}", call.get_expression().get_span().start);
                    return HirExpr::Invalid;
                };
                let receiver = member.get_expression().visit(self);
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
                            _ => return HirExpr::Invalid,
                        };
                        vec![HirExpr::Binary(
                            op,
                            Box::new(HirExpr::member(receiver.clone(), idx)),
                            Box::new(arguments.into_iter().next().unwrap_or(HirExpr::Invalid)),
                        )]
                    }
                };
                HirExpr::member_call(receiver, arguments, idx)
            }
            SemanticInfo::IndexedRecordField(idx) => {
                let Expression::MemberReference(member) = call.get_expression() else {
                    return HirExpr::Invalid;
                };
                HirExpr::indexed_member(member.get_expression().visit(self), idx, arguments)
            }
            SemanticInfo::ArrayMemberFunc(op_code, defaults) => {
                // `a.Len(0)` is `Len(a, 0)`: the receiver leads, then what was written,
                // then whatever the member fills in for a left out argument.
                let Expression::MemberReference(member) = call.get_expression() else {
                    log::error!("array member call without a receiver at: {}", call.get_expression().get_span().start);
                    return HirExpr::Invalid;
                };
                let mut call_arguments = vec![member.get_expression().visit(self)];
                call_arguments.extend(arguments);
                call_arguments.extend(
                    defaults
                        .iter()
                        .map(|value| HirExpr::constant(self.compiler.lookup_table.lookup_constant(&Constant::Integer(*value, NumberFormat::Default)))),
                );
                HirExpr::predefined(op_code, call_arguments)
            }
            SemanticInfo::ScalarMemberFunc(op_code, defaults) => {
                let Expression::MemberReference(member) = call.get_expression() else {
                    return HirExpr::Invalid;
                };
                let mut call_arguments = vec![member.get_expression().visit(self)];
                call_arguments.extend(arguments);
                call_arguments.extend(
                    defaults
                        .iter()
                        .map(|value| HirExpr::constant(self.compiler.lookup_table.lookup_constant(&Constant::Integer(*value, NumberFormat::Default)))),
                );
                HirExpr::predefined(op_code, call_arguments)
            }
            SemanticInfo::ArrayValueAt => {
                let Expression::MemberReference(member) = call.get_expression() else {
                    return HirExpr::Invalid;
                };
                HirExpr::predefined(
                    FuncOpCode::ArrayValueAt,
                    vec![member.get_expression().visit(self), arguments.into_iter().next().unwrap()],
                )
            }
            SemanticInfo::FunctionReference(idx) => {
                let reference_index = self.compiler.semantic_visitor.function_containers[idx].id;
                let table_index = self.compiler.semantic_visitor.references[reference_index].1.variable_table_index;
                HirExpr::function(table_index, arguments)
            }
            SemanticInfo::VariableReference(reference_index) => {
                let r = &self.compiler.semantic_visitor.references[reference_index];
                let table_index = r.1.variable_table_index;
                HirExpr::dim(table_index, arguments)
            }
            SemanticInfo::PredefFunctionGroup(_) => {
                log::error!("Invalid function call: {function_type:?}");
                HirExpr::Invalid
            }
            SemanticInfo::ArrayMemberProc(_) => {
                log::error!("Array statement used where a value is expected: {function_type:?}");
                HirExpr::Invalid
            }
        }
    }

    fn visit_indexer_expression(&mut self, indexer: &crate::ast::IndexerExpression) -> HirExpr {
        let arguments = indexer.get_arguments().iter().map(|e| e.visit(self)).collect();

        if self.compiler.lookup_table.has_variable(indexer.get_identifier()) {
            let Some(table_idx) = self.compiler.lookup_variable_index(indexer.get_identifier()) else {
                log::error!("function not found: {}", indexer.get_identifier());
                return HirExpr::Invalid;
            };

            let var = self.compiler.lookup_table.variable_table.get_var_entry(table_idx);
            if var.value.get_type() == VariableType::Function {
                return HirExpr::function(var.header.id, arguments);
            }
            if var.header.dim == 0
                && matches!(
                    var.header.variable_type,
                    VariableType::String | VariableType::BigStr | VariableType::UnboundedString
                )
                && arguments.len() == 1
            {
                return HirExpr::predefined(
                    FuncOpCode::StringCharAt,
                    vec![HirExpr::variable(var.header.id), arguments.into_iter().next().unwrap()],
                );
            }
            if var.header.dim == 1
                && let VariableType::UserData(type_id) = var.header.variable_type
                && self.compiler.semantic_visitor.type_registry.get_type_from_id(type_id).is_some()
                && arguments.len() == 1
            {
                return HirExpr::predefined(
                    FuncOpCode::ArrayValueAt,
                    vec![HirExpr::variable(var.header.id), arguments.into_iter().next().unwrap()],
                );
            }
            if var.header.dim as usize != arguments.len() {
                log::error!("Invalid dimensions for function call: {}", indexer.get_identifier());
                return HirExpr::Invalid;
            }
            return HirExpr::dim(var.header.id, arguments);
        }
        log::error!("Invalid indexer call: {}", indexer.get_identifier());
        HirExpr::Invalid
    }

    fn visit_parens_expression(&mut self, parens: &crate::ast::ParensExpression) -> HirExpr {
        parens.get_expression().visit(self)
    }
}
