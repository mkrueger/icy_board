use std::collections::HashSet;

use crate::{
    ast::{
        AstVisitor, CommentAstNode, ConstDeclarationStatement, Constant, ConstantExpression, EnumDeclarationAstNode, Expression, FunctionCallExpression,
        FunctionDeclarationAstNode, FunctionImplementation, GosubStatement, GotoStatement, IdentifierExpression, LabelStatement, LetStatement,
        MemberCallStatement, OnErrorMode, OnErrorStatement, ParameterSpecifier, PredefinedCallStatement, ProcedureCallStatement, ProcedureDeclarationAstNode,
        ProcedureImplementation, TypeDeclarationAstNode, VariableDeclarationStatement, VariableParameterSpecifier, const_value_with_members,
        walk_indexer_expression, walk_predefined_call_statement, walk_procedure_call_statement,
    },
    compiler::{CompilationErrorType, CompilationWarningType, user_data::UserDataMemberRegistry},
    executable::{
        FIRST_RECORD_LITERAL_RUNTIME, FIRST_ROUTINE_REFERENCE_RUNTIME, FIRST_TYPE_TABLE_RUNTIME, FUNCTION_DEFINITIONS, FuncOpCode, FunctionDefinition, OpCode,
        VariableType, VariableValue,
    },
    hir::{CallId, SymbolId},
    parser::{
        ParserErrorType,
        lexer::{Spanned, Token},
    },
};

use super::{
    ArrayShape, FunctionContainer, FunctionDeclaration, ReferenceType, References, SemanticInfo, SemanticVisitor, StaticReceiver, VariableLookups,
    array_member, array_procedure, bytes_member, bytes_member_type, string_member, string_member_type, string_type_name, takes_whole_array,
};

impl AstVisitor<VariableType> for SemanticVisitor {
    fn visit_member_call_statement(&mut self, call: &MemberCallStatement) -> VariableType {
        let previous = self.statement_member_call;
        self.statement_member_call = match call.get_expression() {
            Expression::FunctionCall(call) => Some(CallId(call.id)),
            _ => None,
        };
        let result = call.get_expression().visit(self);
        self.statement_member_call = previous;
        result
    }

    fn visit_main(&mut self, main: &crate::ast::BlockStatement) -> VariableType {
        self.visit_statement_sequence(main.get_statements());
        VariableType::None
    }

    fn visit_unary_expression(&mut self, unary: &crate::ast::UnaryExpression) -> VariableType {
        let result = unary.get_expression().visit(self);
        self.reject_bare_array_value(unary.get_expression());
        result
    }

    fn visit_if_statement(&mut self, if_stmt: &crate::ast::IfStatement) -> VariableType {
        crate::ast::walk_if_stmt(self, if_stmt);
        self.reject_bare_array_value(if_stmt.get_condition());
        VariableType::None
    }

    fn visit_if_then_statement(&mut self, if_then: &crate::ast::IfThenStatement) -> VariableType {
        crate::ast::walk_if_then_stmt(self, if_then);
        self.reject_bare_array_value(if_then.get_condition());
        VariableType::None
    }

    fn visit_while_statement(&mut self, while_stmt: &crate::ast::WhileStatement) -> VariableType {
        crate::ast::walk_while_stmt(self, while_stmt);
        self.reject_bare_array_value(while_stmt.get_condition());
        VariableType::None
    }

    fn visit_while_do_statement(&mut self, while_do: &crate::ast::WhileDoStatement) -> VariableType {
        crate::ast::walk_while_do_stmt(self, while_do);
        self.reject_bare_array_value(while_do.get_condition());
        VariableType::None
    }

    fn visit_repeat_until_statement(&mut self, repeat_until: &crate::ast::RepeatUntilStatement) -> VariableType {
        crate::ast::walk_repeat_until_stmt(self, repeat_until);
        self.reject_bare_array_value(repeat_until.get_condition());
        VariableType::None
    }

    fn visit_select_statement(&mut self, select_stmt: &crate::ast::SelectStatement) -> VariableType {
        crate::ast::walk_select_stmt(self, select_stmt);
        self.reject_bare_array_value(select_stmt.get_expression());
        VariableType::None
    }

    fn visit_return_statement(&mut self, return_stmt: &crate::ast::ReturnStatement) -> VariableType {
        crate::ast::walk_return_stmt(self, return_stmt);
        if let Some(expression) = return_stmt.get_expression() {
            self.reject_bare_array_value(expression);
        }
        VariableType::None
    }

    fn visit_record_literal_expression(&mut self, record: &crate::ast::RecordLiteralExpression) -> VariableType {
        if self.runtime < FIRST_RECORD_LITERAL_RUNTIME {
            self.errors.lock().unwrap().report_error(
                record.get_type_token().span.clone(),
                CompilationErrorType::RecordLiteralNeedsRuntime(FIRST_RECORD_LITERAL_RUNTIME),
            );
        }
        let VariableType::UserData(type_id) = record.get_variable_type() else {
            return VariableType::None;
        };
        let Some(definition) = self.type_registry.get_user_type_from_id(type_id) else {
            return VariableType::None;
        };
        let mut seen = HashSet::new();
        for field in record.get_fields() {
            let name = field.get_identifier();
            if !seen.insert(name.clone()) {
                self.errors.lock().unwrap().report_error(
                    field.get_identifier_token().span.clone(),
                    CompilationErrorType::DuplicateRecordLiteralField(name.to_string()),
                );
                continue;
            }
            let Some(index) = definition.field_index(name) else {
                self.errors.lock().unwrap().report_error(
                    field.get_identifier_token().span.clone(),
                    CompilationErrorType::UnknownRecordLiteralField(record.get_variable_type(), name.to_string()),
                );
                field.get_value().visit(self);
                continue;
            };
            let expected_field = definition.field(index);
            let expected = expected_field.map_or(VariableType::None, |field| field.variable_type);
            let actual = field.get_value().visit(self);
            let value_shape = self.array_shape(field.get_value());
            if let Some(expected_field) = expected_field
                && expected_field.dim > 0
            {
                let target_shape = ArrayShape {
                    element_type: expected_field.variable_type,
                    rank: expected_field.dim,
                    bounds: [
                        expected_field.vector_size as usize,
                        expected_field.matrix_size as usize,
                        expected_field.cube_size as usize,
                    ],
                    resizable: false,
                    field_name: Some(name.to_string()),
                };
                self.check_array_target_assignment(&target_shape, field.get_value(), &field.get_value().get_span());
                continue;
            }
            if value_shape.is_some() {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(field.get_value().get_span(), CompilationErrorType::WholeArrayUsedAsScalar);
                continue;
            }
            if expected != actual && (matches!(expected, VariableType::UserData(_)) || matches!(actual, VariableType::UserData(_))) {
                self.errors.lock().unwrap().report_error(
                    field.get_value().get_span(),
                    CompilationErrorType::RecordLiteralFieldTypeMismatch(name.to_string(), self.source_type_name(expected), self.source_type_name(actual)),
                );
            }
        }
        record.get_variable_type()
    }

    fn visit_binary_expression(&mut self, binary: &crate::ast::BinaryExpression) -> VariableType {
        let left = binary.get_left_expression().visit(self);
        let right = binary.get_right_expression().visit(self);
        let left_array = self.array_shape(binary.get_left_expression());
        let right_array = self.array_shape(binary.get_right_expression());
        if left_array.is_some() || right_array.is_some() {
            let compares_record_arrays = matches!(binary.get_op(), crate::ast::BinOp::Eq | crate::ast::BinOp::NotEq)
                && left_array
                    .iter()
                    .chain(right_array.iter())
                    .any(|shape| matches!(shape.element_type, VariableType::UserData(_)));
            if compares_record_arrays {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(binary.get_op_token().span.clone(), CompilationErrorType::CustomTypeArrayComparisonNotSupported);
                return VariableType::None;
            }
            self.reject_bare_array_value(binary.get_left_expression());
            self.reject_bare_array_value(binary.get_right_expression());
            return VariableType::None;
        }
        let has_enum = self.type_registry.is_enum_type(left) || self.type_registry.is_enum_type(right);
        if left == VariableType::UserData(crate::parser::REGEX_OPTIONS_ENUM_ID)
            && right == left
            && matches!(binary.get_op(), crate::ast::BinOp::And | crate::ast::BinOp::Or)
        {
            return left;
        }
        if has_enum && self.counts_a_loop(binary.get_left_expression()) {
            // A FOR writes its own comparison and step, so it may count over an enum.
            return match binary.get_op() {
                crate::ast::BinOp::Lower | crate::ast::BinOp::LowerEq | crate::ast::BinOp::Greater | crate::ast::BinOp::GreaterEq => {
                    if left != right {
                        self.errors.lock().unwrap().report_error(
                            binary.get_right_expression().get_span(),
                            CompilationErrorType::EnumComparisonTypeMismatch(self.source_type_name(left), self.source_type_name(right)),
                        );
                    }
                    VariableType::Boolean
                }
                _ => left,
            };
        }
        if has_enum && !matches!(binary.get_op(), crate::ast::BinOp::Eq | crate::ast::BinOp::NotEq) {
            self.errors.lock().unwrap().report_error(
                binary.get_op_token().span.clone(),
                CompilationErrorType::CustomTypeOperatorNotSupported(binary.get_op()),
            );
            return VariableType::None;
        }
        if has_enum && left != right {
            self.errors.lock().unwrap().report_error(
                binary.get_op_token().span.clone(),
                CompilationErrorType::EnumComparisonTypeMismatch(self.source_type_name(left), self.source_type_name(right)),
            );
            return VariableType::Boolean;
        }
        let has_custom_type = matches!(left, VariableType::UserData(_)) || matches!(right, VariableType::UserData(_));
        if has_custom_type && !matches!(binary.get_op(), crate::ast::BinOp::Eq | crate::ast::BinOp::NotEq) {
            self.errors.lock().unwrap().report_error(
                binary.get_op_token().span.clone(),
                CompilationErrorType::CustomTypeOperatorNotSupported(binary.get_op()),
            );
        }
        if has_custom_type && matches!(binary.get_op(), crate::ast::BinOp::Eq | crate::ast::BinOp::NotEq) {
            if self.is_whole_custom_type_array(binary.get_left_expression()) || self.is_whole_custom_type_array(binary.get_right_expression()) {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(binary.get_op_token().span.clone(), CompilationErrorType::CustomTypeArrayComparisonNotSupported);
            } else if left != right {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(binary.get_op_token().span.clone(), CompilationErrorType::ComparisonTypeMismatch(left, right));
            }
            VariableType::Boolean
        } else {
            VariableType::None
        }
    }

    fn visit_identifier_expression(&mut self, identifier: &IdentifierExpression) -> VariableType {
        if let Some((variable_type, reference_index)) = self
            .lookup_constant(identifier.get_identifier())
            .map(|(variable_type, _, reference_index)| (*variable_type, *reference_index))
        {
            self.add_reference_to(identifier.get_identifier_token(), reference_index);
            return variable_type;
        }
        let predef = FunctionDefinition::get_function_definitions(identifier.get_identifier());
        if !predef.is_empty() && (self.cur_func_call > 0 || self.lookup_variable(identifier.get_identifier()).is_none()) {
            let def = predef
                .iter()
                .map(|index| &FUNCTION_DEFINITIONS[*index])
                .filter(|definition| definition.version <= self.lang_version)
                .max_by_key(|definition| definition.version)
                .unwrap_or(&FUNCTION_DEFINITIONS[predef[0]]);
            if self.cur_func_call > 0 {
                self.function_type_lookup
                    .insert(CallId(self.cur_func_call), SemanticInfo::PredefFunctionGroup(predef));
            } else {
                self.errors.lock().unwrap().report_error(
                    identifier.get_identifier_token().span.clone(),
                    CompilationErrorType::FunctionUsedAsVariable(identifier.get_identifier().to_string()),
                );
            }
            def.return_type
        } else if let Some(idx) = self.lookup_variable(identifier.get_identifier()) {
            if self.cur_func_call == 0
                && self.cur_func_impl == Some(idx)
                && let ReferenceType::Function(container_idx) = self.references[idx].0
                && let FunctionDeclaration::Function(function) = &self.function_containers[container_idx].functions
            {
                self.function_return_value_spans.insert(identifier.get_identifier_token().span.start);
                return function.get_return_type();
            }
            if self.references_are_reachable {
                if matches!(self.references[idx].0, ReferenceType::Function(_) | ReferenceType::Procedure(_)) {
                    self.call_graph.add_call(self.cur_func_impl.map(SymbolId), SymbolId(idx));
                }
                self.reference_owners.entry(idx).or_default().insert(self.cur_func_impl);
            }
            let (rt, r) = &mut self.references[idx];
            let identifier = identifier.get_identifier_token();
            if self.cur_func_call > 0 {
                if let ReferenceType::Function(func_idx) = rt {
                    self.function_type_lookup
                        .insert(CallId(self.cur_func_call), SemanticInfo::FunctionReference(*func_idx));
                } else if let ReferenceType::Variable(func_idx) = rt {
                    self.function_type_lookup
                        .insert(CallId(self.cur_func_call), SemanticInfo::VariableReference(*func_idx));
                }
            } else {
                match rt {
                    ReferenceType::Function(_) | ReferenceType::Procedure(_)
                        if !self.allow_routine_reference && !self.allowed_routine_reference_spans.contains(&identifier.span.start) =>
                    {
                        self.errors.lock().unwrap().report_error(
                            identifier.span.clone(),
                            CompilationErrorType::FunctionUsedAsVariable(identifier.token.to_string()),
                        );
                        return VariableType::None;
                    }
                    ReferenceType::Function(_) | ReferenceType::Procedure(_) if self.allow_routine_reference => {
                        if self.runtime < FIRST_ROUTINE_REFERENCE_RUNTIME {
                            self.errors.lock().unwrap().report_error(
                                identifier.span.clone(),
                                CompilationErrorType::RoutineReferenceNeedsRuntime(FIRST_ROUTINE_REFERENCE_RUNTIME),
                            );
                        }
                        self.allowed_routine_reference_spans.insert(identifier.span.start);
                    }
                    _ => {}
                }
            }
            r.usages
                .push((self.current_file.clone(), Spanned::new(identifier.token.to_string(), identifier.span.clone())));
            r.variable_type
        } else {
            if self.lang_version < 350 || self.cur_func_call == 0 {
                self.errors.lock().unwrap().report_error(
                    identifier.get_identifier_token().span.clone(),
                    CompilationErrorType::VariableNotFound(identifier.get_identifier().to_string()),
                );
            }
            VariableType::None
        }
    }

    fn visit_member_reference_expression(&mut self, member_reference_expression: &crate::ast::MemberReferenceExpression) -> VariableType {
        // Taken rather than read, so the receiver walked below sees it cleared.
        let is_called = self.callee_member.take() == Some(member_reference_expression.get_identifier_token().span.start);
        if let Expression::Identifier(base) = member_reference_expression.get_expression()
            && let Some(definition) = self.type_registry.get_enum(base.get_identifier())
        {
            if let Some(value) = definition.value(member_reference_expression.get_identifier()) {
                self.add_constant(&Constant::Integer(value, crate::ast::constant::NumberFormat::Default));
                self.user_type_lookup
                    .insert(member_reference_expression.get_identifier_token().span.start, definition.id);
                return VariableType::UserData(definition.id);
            }
            self.errors.lock().unwrap().report_error(
                member_reference_expression.get_identifier_token().span.clone(),
                CompilationErrorType::EnumMemberNotFound(definition.name.to_string(), member_reference_expression.get_identifier().to_string()),
            );
            return VariableType::None;
        }
        if let Expression::Identifier(base) = member_reference_expression.get_expression()
            && self.lookup_variable(base.get_identifier()).is_none()
            && matches!(
                crate::parser::built_in_type(base.get_identifier(), self.lang_version),
                Some(VariableType::String | VariableType::BigStr | VariableType::UnboundedString)
            )
        {
            self.member_receiver_type_lookup
                .insert(member_reference_expression.get_identifier_token().span.start, VariableType::String);
            if self.lang_version < 400 {
                self.errors.lock().unwrap().report_error(
                    member_reference_expression.get_identifier_token().span.clone(),
                    CompilationErrorType::BuiltinNeedsRuntime(format!("STRING.{}", member_reference_expression.get_identifier()), 400),
                );
                return VariableType::None;
            }
            if !is_called
                && matches!(
                    member_reference_expression.get_identifier().as_ref().to_ascii_lowercase().as_str(),
                    "join" | "repeat" | "split"
                )
            {
                self.errors.lock().unwrap().report_error(
                    member_reference_expression.get_identifier_token().span.clone(),
                    CompilationErrorType::FunctionUsedAsVariable(member_reference_expression.get_identifier().to_string()),
                );
                return VariableType::None;
            }
            return match member_reference_expression.get_identifier().as_ref().to_ascii_lowercase().as_str() {
                "join" | "repeat" => VariableType::UnboundedString,
                "split" => VariableType::None,
                _ => {
                    self.errors.lock().unwrap().report_error(
                        member_reference_expression.get_identifier_token().span.clone(),
                        CompilationErrorType::InvalidMemberReferenceExpression,
                    );
                    VariableType::None
                }
            };
        }
        if let Expression::Identifier(base) = member_reference_expression.get_expression()
            && self.lookup_variable(base.get_identifier()).is_none()
            && crate::parser::built_in_type(base.get_identifier(), self.lang_version) == Some(VariableType::Bytes)
        {
            self.member_receiver_type_lookup
                .insert(member_reference_expression.get_identifier_token().span.start, VariableType::Bytes);
            return if member_reference_expression.get_identifier().as_ref().eq_ignore_ascii_case("FromBase64") {
                if !is_called {
                    self.errors.lock().unwrap().report_error(
                        member_reference_expression.get_identifier_token().span.clone(),
                        CompilationErrorType::FunctionUsedAsVariable(member_reference_expression.get_identifier().to_string()),
                    );
                    return VariableType::None;
                }
                VariableType::Bytes
            } else {
                self.errors.lock().unwrap().report_error(
                    member_reference_expression.get_identifier_token().span.clone(),
                    CompilationErrorType::InvalidMemberReferenceExpression,
                );
                VariableType::None
            };
        }
        let receiver = self.static_receiver(member_reference_expression.get_expression(), member_reference_expression.get_identifier());
        let called_on_the_type = matches!(receiver, StaticReceiver::StaticMember(_));

        // An array carries the built-in array functions as members. Its type is the
        // element's, so the declaration is what says it has them.
        if matches!(receiver, StaticReceiver::NotAType)
            && self.array_shape(member_reference_expression.get_expression()).is_some()
            && (array_member(member_reference_expression.get_identifier()).is_some() || array_procedure(member_reference_expression.get_identifier()).is_some())
        {
            self.visit_receiver(member_reference_expression.get_expression(), member_reference_expression.get_identifier_token());
            if !is_called {
                self.errors.lock().unwrap().report_error(
                    member_reference_expression.get_identifier_token().span.clone(),
                    CompilationErrorType::FunctionUsedAsVariable(member_reference_expression.get_identifier().to_string()),
                );
                return VariableType::None;
            }
            return array_member(member_reference_expression.get_identifier()).map_or(VariableType::None, |member| member.return_type);
        }

        let t = match receiver {
            StaticReceiver::Instance(type_id) | StaticReceiver::StaticMember(type_id) => VariableType::UserData(type_id),
            StaticReceiver::NotAType => self.visit_receiver(member_reference_expression.get_expression(), member_reference_expression.get_identifier_token()),
            StaticReceiver::Rejected => return VariableType::None,
        };
        if matches!(t, VariableType::String | VariableType::BigStr | VariableType::UnboundedString)
            && let Some(return_type) = string_member_type(member_reference_expression.get_identifier())
        {
            self.member_receiver_type_lookup
                .insert(member_reference_expression.get_identifier_token().span.start, t);
            if self.lang_version < 400 {
                self.errors.lock().unwrap().report_error(
                    member_reference_expression.get_identifier_token().span.clone(),
                    CompilationErrorType::BuiltinNeedsRuntime(format!("STRING.{}", member_reference_expression.get_identifier()), 400),
                );
                return VariableType::None;
            }
            // Every string member is a function, so a bare one leaves codegen nothing to lower.
            if !is_called {
                self.errors.lock().unwrap().report_error(
                    member_reference_expression.get_identifier_token().span.clone(),
                    CompilationErrorType::FunctionUsedAsVariable(member_reference_expression.get_identifier().to_string()),
                );
                return VariableType::None;
            }
            return return_type;
        }
        if t == VariableType::Bytes
            && let Some(return_type) = bytes_member_type(member_reference_expression.get_identifier())
        {
            self.member_receiver_type_lookup
                .insert(member_reference_expression.get_identifier_token().span.start, t);
            if !is_called {
                self.errors.lock().unwrap().report_error(
                    member_reference_expression.get_identifier_token().span.clone(),
                    CompilationErrorType::FunctionUsedAsVariable(member_reference_expression.get_identifier().to_string()),
                );
                return VariableType::None;
            }
            return return_type;
        }
        if let VariableType::UserData(d) = t {
            if self.type_registry.is_record_type(d) {
                return self.resolve_record_field(
                    d,
                    member_reference_expression.get_identifier(),
                    &member_reference_expression.get_identifier_token().span,
                );
            }
            if let Some(t) = self.type_registry.get_type_from_id(d) {
                for (name, t) in &t.fields {
                    if name == member_reference_expression.get_identifier() {
                        self.user_type_lookup.insert(member_reference_expression.get_identifier_token().span.start, d);
                        return *t;
                    }
                }
                for (name, function) in &t.functions {
                    if name == member_reference_expression.get_identifier() {
                        if t.statics.contains(name) && !called_on_the_type {
                            self.errors.lock().unwrap().report_error(
                                member_reference_expression.get_identifier_token().span.clone(),
                                CompilationErrorType::StaticMemberOnValue(name.to_string()),
                            );
                            return VariableType::None;
                        }
                        if !is_called {
                            self.errors.lock().unwrap().report_error(
                                member_reference_expression.get_identifier_token().span.clone(),
                                CompilationErrorType::FunctionUsedAsVariable(name.to_string()),
                            );
                            return VariableType::None;
                        }
                        self.user_type_lookup.insert(member_reference_expression.get_identifier_token().span.start, d);
                        return function.return_type;
                    }
                }
                for name in t.procedures.keys() {
                    if name == member_reference_expression.get_identifier() {
                        if !is_called {
                            self.errors.lock().unwrap().report_error(
                                member_reference_expression.get_identifier_token().span.clone(),
                                CompilationErrorType::FunctionUsedAsVariable(name.to_string()),
                            );
                            return VariableType::None;
                        }
                        self.user_type_lookup.insert(member_reference_expression.get_identifier_token().span.start, d);
                        return VariableType::None;
                    }
                }
                self.errors.lock().unwrap().report_error(
                    member_reference_expression.get_identifier_token().span.clone(),
                    CompilationErrorType::InvalidMemberReferenceExpression,
                );
            } else {
                self.errors.lock().unwrap().report_error(
                    member_reference_expression.get_expression().get_span().clone(),
                    CompilationErrorType::TypeNotFound,
                );
            }
        } else {
            self.errors.lock().unwrap().report_error(
                member_reference_expression.get_identifier_token().span.clone(),
                CompilationErrorType::InvalidMemberReferenceExpression,
            );
        }
        VariableType::None
    }

    fn visit_constant_expression(&mut self, constant: &ConstantExpression) -> VariableType {
        self.add_constant(constant.get_constant_value());
        match constant.get_constant_value() {
            Constant::String(_) => VariableType::String,
            Constant::Boolean(_) => VariableType::Boolean,
            Constant::Money(_) => VariableType::Money,
            Constant::Unsigned(_, _) => VariableType::Unsigned,
            Constant::Double(_) => VariableType::Double,
            Constant::Integer(_, _) | Constant::Builtin(_) => VariableType::Integer,
        }
    }

    fn visit_comment(&mut self, _comment: &CommentAstNode) -> VariableType {
        // nothing yet
        VariableType::None
    }

    fn visit_enum_declaration(&mut self, _enum_decl: &EnumDeclarationAstNode) -> VariableType {
        VariableType::None
    }

    fn visit_predefined_call_statement(&mut self, call_stmt: &PredefinedCallStatement) -> VariableType {
        let def = call_stmt.get_func();
        if def.opcode == OpCode::REDIM && !call_stmt.get_arguments().is_empty() {
            call_stmt.get_arguments()[0].visit(self);
        }
        if def.opcode == OpCode::REDIM
            && let Some(shape) = call_stmt.get_arguments().first().and_then(|argument| self.array_shape(argument))
            && !shape.resizable
        {
            for argument in call_stmt.get_arguments().iter().skip(1) {
                argument.visit(self);
            }
            self.errors.lock().unwrap().report_error(
                call_stmt.get_arguments()[0].get_span(),
                CompilationErrorType::FixedRecordArrayCannotBeRedimmed(shape.field_name.unwrap_or_default()),
            );
            self.add_reference(
                ReferenceType::PredefinedProc(def.opcode),
                VariableType::Procedure,
                call_stmt.get_identifier_token(),
            );
            return VariableType::None;
        }
        if def.opcode != OpCode::REDIM {
            walk_predefined_call_statement(self, call_stmt);
        } else {
            for argument in call_stmt.get_arguments().iter().skip(1) {
                argument.visit(self);
            }
        }
        for (index, argument) in call_stmt.get_arguments().iter().enumerate() {
            if !takes_whole_array(def.opcode, def.sig, index) {
                self.reject_bare_array_value(argument);
            }
        }

        let minimum_runtime = def.opcode.minimum_runtime();
        if self.runtime < minimum_runtime {
            self.errors.lock().unwrap().report_error(
                call_stmt.get_identifier_token().span.clone(),
                CompilationErrorType::BuiltinNeedsRuntime(def.name.to_string(), minimum_runtime),
            );
        }

        match def.sig {
            crate::executable::StatementSignature::Invalid => panic!("Invalid signature"),
            crate::executable::StatementSignature::ArgumentsWithVariable(v, arg_count) => {
                self.check_arg_count(arg_count, call_stmt.get_arguments().len(), call_stmt.get_identifier_token());
                if v > 0
                    && let Some(arg) = call_stmt.get_arguments().get(v - 1)
                {
                    self.check_argument_is_variable(v - 1, arg);
                }
            }
            crate::executable::StatementSignature::VariableArguments(_, min, max) => {
                if call_stmt.get_arguments().len() < min {
                    self.errors.lock().unwrap().report_error(
                        call_stmt.get_identifier_token().span.clone(),
                        CompilationErrorType::TooFewArguments(call_stmt.get_identifier().to_string(), min),
                    );
                }
                if max > 0 && call_stmt.get_arguments().len() > max {
                    self.errors.lock().unwrap().report_error(
                        call_stmt.get_identifier_token().span.clone(),
                        CompilationErrorType::TooManyArguments(call_stmt.get_identifier().to_string(), max),
                    );
                }
            }
            crate::executable::StatementSignature::SpecialCaseDlockg => {
                self.check_arg_count(3, call_stmt.get_arguments().len(), call_stmt.get_identifier_token());
                if call_stmt.get_arguments().len() >= 3 {
                    self.check_argument_is_variable(2, &call_stmt.get_arguments()[2]);
                }
            }
            crate::executable::StatementSignature::SpecialCaseDcreate => {
                self.check_arg_count(4, call_stmt.get_arguments().len(), call_stmt.get_identifier_token());
                if call_stmt.get_arguments().len() >= 4 {
                    self.check_argument_is_variable(3, &call_stmt.get_arguments()[3]);
                }
            }
            crate::executable::StatementSignature::SpecialCaseSort => {
                self.check_arg_count(2, call_stmt.get_arguments().len(), call_stmt.get_identifier_token());

                for i in 0..=1 {
                    if call_stmt.get_arguments().len() <= i {
                        break;
                    }
                    if let Expression::Identifier(a) = &call_stmt.get_arguments()[i] {
                        if let Some(idx) = self.lookup_variable(a.get_identifier()) {
                            let (_rt, r) = &mut self.references[idx];
                            if let Some(header) = &r.header
                                && header.dim != 1
                            {
                                self.errors.lock().unwrap().report_error(
                                    a.get_identifier_token().span.clone(),
                                    CompilationErrorType::SortArgumentDimensionError(header.dim),
                                );
                            }
                        } else {
                            self.errors
                                .lock()
                                .unwrap()
                                .report_error(call_stmt.get_arguments()[i].get_span().clone(), CompilationErrorType::VariableExpected(i + 1));
                        }
                    } else {
                        self.errors
                            .lock()
                            .unwrap()
                            .report_error(call_stmt.get_arguments()[i].get_span().clone(), CompilationErrorType::VariableExpected(i + 1));
                    }
                }
            }
            crate::executable::StatementSignature::SpecialCaseVarSeg => {
                self.check_arg_count(2, call_stmt.get_arguments().len(), call_stmt.get_identifier_token());

                for (v, arg) in call_stmt.get_arguments().iter().enumerate() {
                    self.check_argument_is_variable(v, arg);
                }
            }
            crate::executable::StatementSignature::SpecialCasePop => {
                for (v, arg) in call_stmt.get_arguments().iter().enumerate() {
                    self.check_argument_is_variable(v, arg);
                }
            }
        }

        if matches!(def.opcode, OpCode::FGetRec | OpCode::FPutRec | OpCode::FReadRec | OpCode::FWriteRec)
            && let Some(record) = call_stmt.get_arguments().get(1)
        {
            let actual = self.resolved_record_io_type(record);
            let VariableType::UserData(type_id) = actual else {
                self.errors.lock().unwrap().report_error(
                    record.get_span(),
                    CompilationErrorType::ArgumentTypeMismatch(2, "user-defined record".to_string(), self.source_type_name(actual)),
                );
                return VariableType::None;
            };
            if !crate::parser::is_user_declared_type(type_id) {
                self.errors.lock().unwrap().report_error(
                    record.get_span(),
                    CompilationErrorType::ArgumentTypeMismatch(2, "user-defined record".to_string(), self.source_type_name(actual)),
                );
            } else if let Some((path, field_type)) = self.first_unserializable_record_field(type_id, "") {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(record.get_span(), CompilationErrorType::RecordIoFieldNotSerializable(path, field_type));
            }
        }

        self.add_reference(
            ReferenceType::PredefinedProc(call_stmt.get_func().opcode),
            VariableType::Procedure,
            call_stmt.get_identifier_token(),
        );
        VariableType::None
    }

    fn visit_function_call_expression(&mut self, call: &FunctionCallExpression) -> VariableType {
        let mut res = VariableType::None;
        let is_ident = matches!(call.get_expression(), Expression::Identifier(_));
        if let Expression::MemberReference(member) = call.get_expression()
            && let Some((_, opcode, arguments)) = array_procedure(member.get_identifier())
        {
            self.visit_receiver(member.get_expression(), member.get_identifier_token());
            if let Some(shape) = self.array_shape(member.get_expression()) {
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                if self.statement_member_call != Some(CallId(call.id)) {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(member.get_identifier_token().span.clone(), CompilationErrorType::ProcedureUsedAsFunction);
                    return VariableType::None;
                }
                if !shape.resizable {
                    self.errors.lock().unwrap().report_error(
                        member.get_expression().get_span(),
                        CompilationErrorType::FixedRecordArrayCannotBeRedimmed(shape.field_name.unwrap_or_default()),
                    );
                    return VariableType::None;
                }
                let given = call.get_arguments().len();
                if !arguments.contains(&given) {
                    self.check_expr_arg_range(*arguments.start(), *arguments.end(), given, call.get_expression());
                    return VariableType::None;
                }
                self.function_type_lookup.insert(CallId(call.id), SemanticInfo::ArrayMemberProc(*opcode));
                return VariableType::None;
            }
        }
        if let Expression::MemberReference(member) = call.get_expression()
            && let Some(array_member) = array_member(member.get_identifier())
        {
            self.visit_receiver(member.get_expression(), member.get_identifier_token());
            if self.array_shape(member.get_expression()).is_some() {
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                let given = call.get_arguments().len();
                if !array_member.arguments.contains(&given) {
                    self.check_expr_arg_range(*array_member.arguments.start(), *array_member.arguments.end(), given, call.get_expression());
                    return VariableType::None;
                }
                let filled_in = array_member.defaults[given.min(array_member.defaults.len())..].to_vec();
                for value in &filled_in {
                    self.add_constant(&Constant::Integer(*value, crate::ast::constant::NumberFormat::Default));
                }
                self.function_type_lookup
                    .insert(CallId(call.id), SemanticInfo::ArrayMemberFunc(array_member.opcode, filled_in));
                return array_member.return_type;
            }
        }
        if self.lang_version >= 400
            && let Expression::MemberReference(member) = call.get_expression()
            && member.get_identifier().as_ref() == "<get>"
            && call.get_arguments().len() == 1
        {
            let receiver_type = self.visit_receiver(member.get_expression(), member.get_identifier_token());
            if let Some(shape) = self.array_shape(member.get_expression())
                && shape.rank == 1
            {
                call.get_arguments()[0].visit(self);
                self.function_type_lookup.insert(CallId(call.id), SemanticInfo::ArrayValueAt);
                return shape.element_type;
            }
            if matches!(receiver_type, VariableType::String | VariableType::BigStr | VariableType::UnboundedString) {
                call.get_arguments()[0].visit(self);
                self.function_type_lookup
                    .insert(CallId(call.id), SemanticInfo::ScalarMemberFunc(FuncOpCode::StringCharAt, &[]));
                return VariableType::UnboundedString;
            }
        }
        let outer_func_call = self.cur_func_call;
        self.cur_func_call = call.id;
        if let Expression::MemberReference(member) = call.get_expression() {
            self.callee_member = Some(member.get_identifier_token().span.start);
        }
        call.get_expression().visit(self);
        self.callee_member = None;
        self.cur_func_call = outer_func_call;

        // A member call is decided by the receiver's type, whatever expression produced it.
        if let Expression::MemberReference(member) = call.get_expression() {
            if string_type_name(member.get_expression(), self.lang_version)
                && let Expression::Identifier(base) = member.get_expression()
                && self.lookup_variable(base.get_identifier()).is_none()
            {
                match member.get_identifier().as_ref().to_ascii_lowercase().as_str() {
                    "join" if call.get_arguments().len() == 2 => {
                        for argument in call.get_arguments() {
                            argument.visit(self);
                        }
                        let valid_array = call
                            .get_arguments()
                            .first()
                            .and_then(|argument| self.array_shape(argument))
                            .is_some_and(|shape| {
                                shape.rank == 1 && matches!(shape.element_type, VariableType::String | VariableType::BigStr | VariableType::UnboundedString)
                            });
                        if !valid_array {
                            self.errors.lock().unwrap().report_error(
                                call.get_arguments()[0].get_span(),
                                CompilationErrorType::ArgumentTypeMismatch(1, "one-dimensional string array".to_string(), "value".to_string()),
                            );
                        }
                        self.function_type_lookup
                            .insert(CallId(call.id), SemanticInfo::ScalarStaticFunc(FuncOpCode::StringJoin));
                        return VariableType::UnboundedString;
                    }
                    "repeat" if call.get_arguments().len() == 2 => {
                        for argument in call.get_arguments() {
                            argument.visit(self);
                        }
                        self.function_type_lookup
                            .insert(CallId(call.id), SemanticInfo::ScalarStaticFunc(FuncOpCode::StringRepeat));
                        return VariableType::UnboundedString;
                    }
                    "split" if (2..=3).contains(&call.get_arguments().len()) => {
                        let argument_types: Vec<_> = call.get_arguments().iter().map(|argument| argument.visit(self)).collect();
                        let opcode = if call.get_arguments().len() == 2 {
                            FuncOpCode::StringSplit
                        } else {
                            FuncOpCode::StringSplitLimit
                        };
                        if opcode == FuncOpCode::StringSplitLimit && argument_types.last() != Some(&VariableType::Integer) {
                            let actual = argument_types.last().copied().unwrap_or(VariableType::None);
                            self.errors.lock().unwrap().report_error(
                                call.get_arguments().last().unwrap().get_span(),
                                CompilationErrorType::ArgumentTypeMismatch(3, "INTEGER".to_string(), self.source_type_name(actual)),
                            );
                        }
                        self.function_type_lookup.insert(CallId(call.id), SemanticInfo::ScalarStaticFunc(opcode));
                        self.member_array_returns.insert(CallId(call.id), (VariableType::UnboundedString, 1));
                        return VariableType::UnboundedString;
                    }
                    _ => {}
                }
            }

            if matches!(
                member.get_expression(),
                Expression::Identifier(identifier)
                    if crate::parser::built_in_type(identifier.get_identifier(), self.lang_version) == Some(VariableType::Bytes)
                        && self.lookup_variable(identifier.get_identifier()).is_none()
            ) && member.get_identifier().as_ref().eq_ignore_ascii_case("FromBase64")
                && call.get_arguments().len() == 1
            {
                call.get_arguments()[0].visit(self);
                self.function_type_lookup
                    .insert(CallId(call.id), SemanticInfo::ScalarStaticFunc(FuncOpCode::BASE64DEC));
                return VariableType::Bytes;
            }

            let registered_type_receiver = matches!(
                member.get_expression(),
                Expression::Identifier(identifier) if self.type_registry.get_board_object(identifier.get_identifier()).is_some()
            );
            let receiver_type = if registered_type_receiver {
                VariableType::None
            } else {
                self.visit_receiver(member.get_expression(), member.get_identifier_token())
            };
            if matches!(receiver_type, VariableType::String | VariableType::BigStr | VariableType::UnboundedString)
                && let Some((opcode, return_type, defaults)) = string_member(member.get_identifier(), call.get_arguments().len())
            {
                let argument_types: Vec<_> = call.get_arguments().iter().map(|argument| argument.visit(self)).collect();
                if opcode == FuncOpCode::StringSplitLimit && argument_types.last() != Some(&VariableType::Integer) {
                    let actual = argument_types.last().copied().unwrap_or(VariableType::None);
                    self.errors.lock().unwrap().report_error(
                        call.get_arguments().last().unwrap().get_span(),
                        CompilationErrorType::ArgumentTypeMismatch(call.get_arguments().len(), "INTEGER".to_string(), self.source_type_name(actual)),
                    );
                }
                if matches!(
                    opcode,
                    FuncOpCode::StringFindComparison
                        | FuncOpCode::StringFindLastComparison
                        | FuncOpCode::StringContainsComparison
                        | FuncOpCode::StringStartsWithComparison
                        | FuncOpCode::StringEndsWithComparison
                        | FuncOpCode::StringCountComparison
                        | FuncOpCode::StringEqualsComparison
                ) && argument_types.last() != Some(&VariableType::UserData(crate::parser::STRING_COMPARISON_ENUM_ID))
                {
                    let actual = argument_types.last().copied().unwrap_or(VariableType::None);
                    self.errors.lock().unwrap().report_error(
                        call.get_arguments().last().unwrap().get_span(),
                        CompilationErrorType::ArgumentTypeMismatch(call.get_arguments().len(), "StringComparison".to_string(), self.source_type_name(actual)),
                    );
                }
                for value in defaults {
                    self.add_constant(&Constant::Integer(*value, crate::ast::constant::NumberFormat::Default));
                }
                if self.runtime < opcode.minimum_runtime() {
                    self.errors.lock().unwrap().report_error(
                        member.get_identifier_token().span.clone(),
                        CompilationErrorType::BuiltinNeedsRuntime(format!("STRING.{}", member.get_identifier()), opcode.minimum_runtime()),
                    );
                }
                self.function_type_lookup
                    .insert(CallId(call.id), SemanticInfo::ScalarMemberFunc(opcode, defaults));
                if matches!(opcode, FuncOpCode::StringSplit | FuncOpCode::StringSplitLimit) {
                    self.member_array_returns.insert(CallId(call.id), (VariableType::UnboundedString, 1));
                }
                return return_type;
            }
            if receiver_type == VariableType::Bytes
                && let Some((opcode, return_type)) = bytes_member(member.get_identifier(), call.get_arguments().len())
            {
                let argument_types: Vec<_> = call.get_arguments().iter().map(|argument| argument.visit(self)).collect();
                if opcode == FuncOpCode::BytesGetChecksum && argument_types.first() != Some(&VariableType::UserData(crate::parser::CHECKSUM_ENUM_ID)) {
                    let actual = argument_types.first().copied().unwrap_or(VariableType::None);
                    self.errors.lock().unwrap().report_error(
                        call.get_arguments()[0].get_span(),
                        CompilationErrorType::ArgumentTypeMismatch(1, "Checksum".to_string(), self.source_type_name(actual)),
                    );
                }
                self.function_type_lookup.insert(CallId(call.id), SemanticInfo::ScalarMemberFunc(opcode, &[]));
                return return_type;
            }

            // An array's members are the built-in array functions written the other way round.
            if self.array_shape(member.get_expression()).is_some()
                && let Some(array_member) = array_member(member.get_identifier())
            {
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                let given = call.get_arguments().len();
                if !array_member.arguments.contains(&given) {
                    self.check_expr_arg_range(*array_member.arguments.start(), *array_member.arguments.end(), given, call.get_expression());
                    return VariableType::None;
                }
                let filled_in = array_member.defaults[given.min(array_member.defaults.len())..].to_vec();
                for value in &filled_in {
                    self.add_constant(&Constant::Integer(*value, crate::ast::constant::NumberFormat::Default));
                }
                self.function_type_lookup
                    .insert(CallId(call.id), SemanticInfo::ArrayMemberFunc(array_member.opcode, filled_in));
                return array_member.return_type;
            }

            if self.array_shape(member.get_expression()).is_some()
                && let Some((_, opcode, arguments)) = array_procedure(member.get_identifier())
            {
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                let given = call.get_arguments().len();
                if !arguments.contains(&given) {
                    self.check_expr_arg_range(*arguments.start(), *arguments.end(), given, call.get_expression());
                    return VariableType::None;
                }
                self.function_type_lookup.insert(CallId(call.id), SemanticInfo::ArrayMemberProc(*opcode));
                return VariableType::None;
            }

            if let Some(type_id) = self.user_type_lookup.get(&member.get_identifier_token().span.start).copied()
                && self.type_registry.is_record_type(type_id)
                && let Some(member_id) = self.type_registry.record_field_index(type_id, member.get_identifier())
                && let Some(field) = self
                    .type_registry
                    .get_record_type_from_id(type_id)
                    .and_then(|definition| definition.field(member_id))
                && field.dim > 0
            {
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                if field.dim as usize != call.get_arguments().len() {
                    self.errors.lock().unwrap().report_error(
                        call.get_expression().get_span(),
                        CompilationErrorType::RecordArrayIndexCount(
                            member.get_identifier().to_string(),
                            field.dim,
                            call.get_arguments().len(),
                            if call.get_arguments().len() == 1 { "index was" } else { "indices were" },
                        ),
                    );
                }
                self.function_type_lookup.insert(CallId(call.id), SemanticInfo::IndexedRecordField(member_id));
                return field.variable_type;
            }

            let Some(user_type) = self.user_type_lookup.get(&member.get_identifier_token().span.start).copied() else {
                // Visiting the member reference already reported why it could not be resolved.
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                return VariableType::None;
            };
            if !member.get_identifier().starts_with('<')
                && matches!(
                    call.get_lpar_token().token,
                    Token::Eq
                        | Token::AddAssign
                        | Token::SubAssign
                        | Token::MulAssign
                        | Token::DivAssign
                        | Token::ModAssign
                        | Token::AndAssign
                        | Token::OrAssign
                )
            {
                let Some(registry) = self.type_registry.get_type_from_id(user_type) else {
                    for argument in call.get_arguments() {
                        argument.visit(self);
                    }
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(member.get_identifier_token().span.clone(), CompilationErrorType::InvalidLetVariable);
                    return VariableType::None;
                };
                let Some(member_id) = registry.get_member_id(member.get_identifier()) else {
                    return VariableType::None;
                };
                if !matches!(registry.id_table.get(member_id), Some(crate::compiler::user_data::UserDataEntry::Field(_))) {
                    for argument in call.get_arguments() {
                        argument.visit(self);
                    }
                    self.errors.lock().unwrap().report_error(
                        member.get_identifier_token().span.clone(),
                        CompilationErrorType::MemberIsReadOnly(member.get_identifier().to_string()),
                    );
                    return VariableType::None;
                }
                let expected = registry.fields.get(member.get_identifier()).copied().unwrap_or(VariableType::None);
                self.check_member_arg_types(&[expected], call.get_arguments());
                self.function_type_lookup.insert(CallId(call.id), SemanticInfo::MemberSetterCall(member_id));
                return VariableType::None;
            }
            // A record field indexed like `rec.field(1)` is not a member call; the variable path below takes it.
            if self.type_registry.get_type_from_id(user_type).is_some() {
                if let Some((member_id, required, parameters, return_type, return_rank)) = self.member_function_signature(user_type, member.get_identifier()) {
                    self.check_expr_arg_range(required, parameters.len(), call.get_arguments().len(), call.get_expression());
                    self.check_member_arg_types(&parameters, call.get_arguments());
                    self.function_type_lookup.insert(CallId(call.id), SemanticInfo::MemberFunctionCall(member_id));
                    if return_rank > 0 {
                        self.member_array_returns.insert(CallId(call.id), (return_type, return_rank));
                    }
                    return return_type;
                }
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                self.errors.lock().unwrap().report_error(
                    member.get_identifier_token().span.clone(),
                    CompilationErrorType::FunctionNotFound(member.get_identifier().to_string()),
                );
                return VariableType::None;
            }
        }

        match self.function_type_lookup.get(&CallId(call.id)).cloned() {
            Some(SemanticInfo::FunctionReference(idx)) => {
                let declaration = self.function_containers[idx].functions.clone();
                let arg_count = if let FunctionDeclaration::Function(f) = &declaration {
                    res = f.get_return_type();
                    self.check_arg_types(f.get_parameters(), call.get_arguments());
                    f.get_parameters().len()
                } else {
                    self.errors.lock().unwrap().report_error(
                        call.get_expression().get_span(),
                        CompilationErrorType::FunctionNotFound(call.get_expression().to_string()),
                    );
                    0
                };
                self.check_expr_arg_count(arg_count, call.get_arguments().len(), call.get_expression());
            }
            Some(SemanticInfo::VariableReference(idx)) => {
                for argument in call.get_arguments() {
                    argument.visit(self);
                    self.reject_bare_array_value(argument);
                }

                let (rt, r) = &mut self.references[idx];

                if self.lang_version >= 400 && r.header.as_ref().is_some_and(|header| header.dim > 0) && call.get_lpar_token().token == Token::LPar {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_warning(call.get_lpar_token().span.clone(), CompilationWarningType::ArrayBracketsRequired);
                }

                let arg_count = if let ReferenceType::Variable(_func) = rt {
                    r.header.as_ref().unwrap().dim as usize
                } else {
                    0
                };
                res = r.variable_type;
                self.check_expr_arg_count(arg_count, call.get_arguments().len(), call.get_expression());
            }
            Some(SemanticInfo::PredefFunctionGroup(funcs)) => {
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                let mut funcs = funcs;
                funcs.sort_by_key(|func| std::cmp::Reverse(FUNCTION_DEFINITIONS[*func].version));
                for func in &funcs {
                    let def = &FUNCTION_DEFINITIONS[*func];
                    if def.parameter_count() == call.get_arguments().len() && def.version <= self.lang_version {
                        let minimum_runtime = def.opcode.minimum_runtime();
                        if self.runtime < minimum_runtime {
                            self.errors.lock().unwrap().report_error(
                                call.get_expression().get_span(),
                                CompilationErrorType::BuiltinNeedsRuntime(def.name.to_string(), minimum_runtime),
                            );
                            return res;
                        }
                        self.function_type_lookup.insert(CallId(call.id), SemanticInfo::PredefinedFunc(def.opcode));
                        if def.opcode != FuncOpCode::Len_Dim {
                            for argument in call.get_arguments() {
                                self.reject_bare_array_value(argument);
                            }
                        }
                        if let Expression::Identifier(id) = call.get_expression() {
                            self.add_reference(ReferenceType::PredefinedFunc(def.opcode), VariableType::Function, id.get_identifier_token());
                        }
                        return def.return_type;
                    }
                }
                if let Some(def) = funcs
                    .iter()
                    .map(|func| &FUNCTION_DEFINITIONS[*func])
                    .filter(|def| def.parameter_count() == call.get_arguments().len())
                    .min_by_key(|def| def.version)
                {
                    self.errors.lock().unwrap().report_error(
                        call.get_expression().get_span(),
                        ParserErrorType::FunctionVersionNotSupported(def.opcode, def.version, self.lang_version),
                    );
                    return res;
                }
                // report wrong argument count
                self.check_expr_arg_count(
                    FUNCTION_DEFINITIONS[funcs[0]].parameter_count(),
                    call.get_arguments().len(),
                    call.get_expression(),
                );
            }

            _ => {
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                if self.lang_version < 350 || !is_ident {
                    self.errors.lock().unwrap().report_error(
                        call.get_expression().get_span(),
                        CompilationErrorType::FunctionNotFound(call.get_expression().to_string()),
                    );
                } else if let Expression::Identifier(ident) = call.get_expression() {
                    let id = self.add_declaration(VariableType::Function, ident.get_identifier_token());
                    self.global_lookup.variable_lookup.insert(ident.get_identifier().clone(), id);
                    self.function_containers.push(FunctionContainer {
                        name: ident.get_identifier().clone(),
                        parameter_index: None,
                        id,
                        functions: FunctionDeclaration::Function(FunctionDeclarationAstNode::empty(
                            ident.get_identifier().clone(),
                            call.get_arguments()
                                .iter()
                                .map(|_a| ParameterSpecifier::Variable(VariableParameterSpecifier::empty(false, VariableType::None, None)))
                                .collect(),
                            VariableType::None,
                        )),
                        lookup: VariableLookups::default(),
                        parameters: 0..0,
                        local_variables: 0..0,
                    });
                } else {
                    panic!("Invalid function call expression");
                }
            }
        }
        res
    }

    fn visit_indexer_expression(&mut self, indexer: &crate::ast::IndexerExpression) -> VariableType {
        let mut found = false;
        let mut res = VariableType::None;
        let mut string_index = false;
        let arg_count = if let Some(idx) = self.lookup_variable(indexer.get_identifier()) {
            if self.references_are_reachable {
                self.reference_owners.entry(idx).or_default().insert(self.cur_func_impl);
            }
            let (rt, r) = &mut self.references[idx];
            if matches!(rt, ReferenceType::Function(_)) || r.header.is_none() {
                // A routine or a label carries no variable header, so an indexer has nothing to address.
                self.errors.lock().unwrap().report_error(
                    indexer.get_identifier_token().span.clone(),
                    CompilationErrorType::IndexerCalledOnFunction(indexer.get_identifier().to_string()),
                );
                return VariableType::None;
            }
            found = true;
            res = r.variable_type;
            string_index = self.lang_version >= 400
                && matches!(r.variable_type, VariableType::String | VariableType::BigStr | VariableType::UnboundedString)
                && r.header.as_ref().unwrap().dim == 0
                && indexer.get_arguments().len() == 1;
            r.usages.push((
                self.current_file.clone(),
                Spanned::new(indexer.get_identifier().to_string(), indexer.get_identifier_token().span.clone()),
            ));
            r.header.as_ref().unwrap().dim as usize
        } else {
            0
        };

        if found {
            if string_index {
                res = VariableType::String;
            } else {
                self.check_arg_count(arg_count, indexer.get_arguments().len(), indexer.get_identifier_token());
            }
        } else {
            self.errors.lock().unwrap().report_error(
                indexer.get_identifier_token().span.clone(),
                CompilationErrorType::FunctionNotFound(indexer.get_identifier().to_string()),
            );
        }
        walk_indexer_expression(self, indexer);
        for argument in indexer.get_arguments() {
            self.reject_bare_array_value(argument);
        }
        res
    }

    fn visit_goto_statement(&mut self, goto: &GotoStatement) -> VariableType {
        self.add_label_usage(goto.get_label_token());
        VariableType::None
    }

    fn visit_gosub_statement(&mut self, gosub: &GosubStatement) -> VariableType {
        self.add_label_usage(gosub.get_label_token());
        VariableType::None
    }

    fn visit_on_error_statement(&mut self, on_error: &OnErrorStatement) -> VariableType {
        match on_error.get_mode() {
            OnErrorMode::Off => {}
            OnErrorMode::Goto | OnErrorMode::Gosub => self.add_label_usage(on_error.get_target_token()),
            OnErrorMode::Procedure => {
                let Some(name) = on_error.get_target() else {
                    return VariableType::None;
                };
                let Some(idx) = self.lookup_variable(name) else {
                    self.errors.lock().unwrap().report_error(
                        on_error.get_target_token().span.clone(),
                        CompilationErrorType::ProcedureNotFound(name.to_string()),
                    );
                    return VariableType::None;
                };
                if !matches!(self.references[idx].0, ReferenceType::Procedure(_)) {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(on_error.get_target_token().span.clone(), CompilationErrorType::ProcedureExpected);
                    return VariableType::None;
                }
                if let Some(container) = self.function_containers.iter().find(|p| p.name == *name)
                    && let FunctionDeclaration::Procedure(procedure) = &container.functions.clone()
                {
                    let parameters = procedure.get_parameters();
                    // The handler is called from wherever the failure happened, so there is no
                    // argument expression a VAR parameter could be written back to.
                    let takes_the_error = match parameters.len() {
                        0 => true,
                        1 => match &parameters[0] {
                            ParameterSpecifier::Variable(var) => {
                                !var.is_var() && var.get_variable_type() == VariableType::UserData(crate::parser::ERROR_ID as u8)
                            }
                            ParameterSpecifier::Function(_) | ParameterSpecifier::Procedure(_) => false,
                        },
                        _ => false,
                    };
                    if !takes_the_error {
                        self.errors.lock().unwrap().report_error(
                            on_error.get_target_token().span.clone(),
                            CompilationErrorType::InvalidErrorHandler(name.to_string()),
                        );
                    }
                }
                self.add_reference_to(on_error.get_target_token(), idx);
            }
        }
        VariableType::None
    }

    fn visit_label_statement(&mut self, label: &LabelStatement) -> VariableType {
        self.set_label_declaration(label.get_label_token());
        VariableType::None
    }

    fn visit_let_statement(&mut self, let_stmt: &LetStatement) -> VariableType {
        if let Some(target) = let_stmt.get_target_expression() {
            let target_type = target.visit(self);
            if !self.is_assignable_explicit_target(target) {
                if let Expression::MemberReference(member) = target
                    && let Some(type_id) = self.user_type_lookup.get(&member.get_identifier_token().span.start).copied()
                {
                    if self.type_registry.is_record_type(type_id) {
                        self.errors
                            .lock()
                            .unwrap()
                            .report_error(member.get_identifier_token().span.clone(), CompilationErrorType::InvalidLetVariable);
                    } else if let Some(registry) = self.type_registry.get_type_from_id(type_id)
                        && let Some(member_id) = registry.get_member_id(member.get_identifier())
                        && !matches!(registry.id_table.get(member_id), Some(crate::compiler::user_data::UserDataEntry::Field(_)))
                    {
                        self.errors.lock().unwrap().report_error(
                            member.get_identifier_token().span.clone(),
                            CompilationErrorType::MemberIsReadOnly(member.get_identifier().to_string()),
                        );
                    }
                }
                return VariableType::None;
            }
            let value_type = let_stmt.get_value_expression().visit(self);
            if let Some(target_shape) = self.array_shape(target) {
                self.check_array_target_assignment(&target_shape, let_stmt.get_value_expression(), &let_stmt.get_eq_token().span);
                return VariableType::None;
            }
            if self.array_shape(let_stmt.get_value_expression()).is_some() {
                self.reject_bare_array_value(let_stmt.get_value_expression());
                return VariableType::None;
            }
            if target_type != value_type && !matches!(target_type, VariableType::None) {
                let_stmt.get_value_expression().visit(self);
            }
            return VariableType::None;
        }
        let mut target_type = VariableType::None;
        let mut target_array_shape = None;
        if self.lookup_constant(let_stmt.get_identifier()).is_some() {
            self.errors.lock().unwrap().report_error(
                let_stmt.get_identifier_token().span.clone(),
                CompilationErrorType::CannotAssignToConstant(let_stmt.get_identifier().to_string()),
            );
            return VariableType::None;
        }
        if let Some(idx) = self.lookup_variable(let_stmt.get_identifier()) {
            if self.references[idx].1.variable_type == VariableType::Procedure {
                self.errors
                    .lock()
                    .unwrap()
                    .report_warning(let_stmt.get_identifier_token().span.clone(), CompilationWarningType::CannotAssignToProcedure);
            } else if self.references[idx].1.variable_type == VariableType::Function {
                self.references[idx].1.return_types.push((
                    self.current_file.clone(),
                    Spanned::new(let_stmt.get_identifier().to_string(), let_stmt.get_identifier_token().span.clone()),
                ));
                if let Some(container) = self.function_containers.iter().find(|container| container.id == idx)
                    && let FunctionDeclaration::Function(function) = &container.functions
                {
                    target_type = function.get_return_type();
                    if function.get_return_rank() > 0 {
                        target_array_shape = Some(ArrayShape {
                            element_type: target_type,
                            rank: function.get_return_rank(),
                            bounds: [0; 3],
                            resizable: true,
                            field_name: None,
                        });
                    }
                }
            } else {
                target_type = self.references[idx].1.variable_type;
                if let Some(header) = &self.references[idx].1.header {
                    if self.lang_version >= 400 && header.dim > 0 && let_stmt.get_arguments().is_empty() {
                        target_array_shape = Some(ArrayShape {
                            element_type: target_type,
                            rank: header.dim,
                            bounds: [header.vector_size, header.matrix_size, header.cube_size],
                            resizable: true,
                            field_name: None,
                        });
                    } else {
                        self.check_arg_count(header.dim as usize, let_stmt.get_arguments().len(), let_stmt.get_identifier_token());
                    }
                } else {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(let_stmt.get_identifier_token().span.clone(), CompilationErrorType::InvalidLetVariable);
                }

                self.add_reference_to(let_stmt.get_identifier_token(), idx);

                let mut variable_type = target_type;
                for (position, member_token) in let_stmt.get_members().iter().enumerate() {
                    match variable_type {
                        VariableType::UserData(type_id) if self.type_registry.is_record_type(type_id) => {
                            let Token::Identifier(member) = &member_token.token else {
                                break;
                            };
                            variable_type = self.resolve_record_field(type_id, member, &member_token.span);
                            if position + 1 == let_stmt.get_members().len()
                                && let Some(definition) = self.type_registry.get_record_type_from_id(type_id)
                                && let Some(field_id) = definition.field_index(member)
                                && let Some(field) = definition.field(field_id)
                                && field.dim > 0
                            {
                                target_array_shape = Some(ArrayShape {
                                    element_type: field.variable_type,
                                    rank: field.dim,
                                    bounds: [field.vector_size as usize, field.matrix_size as usize, field.cube_size as usize],
                                    resizable: false,
                                    field_name: Some(member.to_string()),
                                });
                            }
                        }
                        VariableType::UserData(type_id) => {
                            let Token::Identifier(member) = &member_token.token else {
                                break;
                            };
                            let Some(registry) = self.type_registry.get_type_from_id(type_id) else {
                                self.errors
                                    .lock()
                                    .unwrap()
                                    .report_error(member_token.span.clone(), CompilationErrorType::InvalidLetVariable);
                                break;
                            };
                            let Some(member_id) = registry.get_member_id(member) else {
                                self.errors
                                    .lock()
                                    .unwrap()
                                    .report_error(member_token.span.clone(), CompilationErrorType::InvalidLetVariable);
                                break;
                            };
                            let is_last = position + 1 == let_stmt.get_members().len();
                            if is_last && !matches!(registry.id_table.get(member_id), Some(crate::compiler::user_data::UserDataEntry::Field(_))) {
                                self.errors
                                    .lock()
                                    .unwrap()
                                    .report_error(member_token.span.clone(), CompilationErrorType::MemberIsReadOnly(member.to_string()));
                                break;
                            }
                            self.user_type_lookup.insert(member_token.span.start, type_id);
                            variable_type = registry.fields.get(member).copied().unwrap_or(VariableType::None);
                        }
                        _ => {
                            self.errors
                                .lock()
                                .unwrap()
                                .report_error(member_token.span.clone(), CompilationErrorType::InvalidLetVariable);
                            break;
                        }
                    }
                }
                target_type = variable_type;
            }
        } else {
            let root = let_stmt.get_identifier();
            let Some(VariableType::UserData(mut type_id)) = self.type_registry.get_board_object(root) else {
                self.errors.lock().unwrap().report_error(
                    let_stmt.get_identifier_token().span.clone(),
                    CompilationErrorType::VariableNotFound(root.to_string()),
                );
                return VariableType::None;
            };
            let Some(provider) = self.type_registry.get_type_from_id(type_id).and_then(|registry| registry.instance_provider) else {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(let_stmt.get_identifier_token().span.clone(), CompilationErrorType::InvalidLetVariable);
                return VariableType::None;
            };
            self.instance_provider_lookup.insert(let_stmt.get_identifier_token().span.start, provider);
            for (position, member_token) in let_stmt.get_members().iter().enumerate() {
                let Token::Identifier(member) = &member_token.token else {
                    return VariableType::None;
                };
                let Some(registry) = self.type_registry.get_type_from_id(type_id) else {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(member_token.span.clone(), CompilationErrorType::InvalidLetVariable);
                    return VariableType::None;
                };
                let Some(member_id) = registry.get_member_id(member) else {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(member_token.span.clone(), CompilationErrorType::InvalidLetVariable);
                    return VariableType::None;
                };
                let is_last = position + 1 == let_stmt.get_members().len();
                if is_last && !matches!(registry.id_table.get(member_id), Some(crate::compiler::user_data::UserDataEntry::Field(_))) {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(member_token.span.clone(), CompilationErrorType::MemberIsReadOnly(member.to_string()));
                    return VariableType::None;
                }
                self.user_type_lookup.insert(member_token.span.start, type_id);
                target_type = registry.fields.get(member).copied().unwrap_or(VariableType::None);
                if let VariableType::UserData(next) = target_type {
                    type_id = next;
                }
            }
        }
        for arg in let_stmt.get_arguments() {
            arg.visit(self);
        }
        let value_type = let_stmt.get_value_expression().visit(self);
        if let Some(target_shape) = target_array_shape {
            self.check_array_target_assignment(&target_shape, let_stmt.get_value_expression(), &let_stmt.get_eq_token().span);
            return VariableType::None;
        }
        if self.array_shape(let_stmt.get_value_expression()).is_some() && !let_stmt.get_members().is_empty() {
            self.errors
                .lock()
                .unwrap()
                .report_error(let_stmt.get_eq_token().span.clone(), CompilationErrorType::WholeArrayUsedAsScalar);
            return VariableType::None;
        }
        if self.array_shape(let_stmt.get_value_expression()).is_some() {
            self.reject_bare_array_value(let_stmt.get_value_expression());
            return VariableType::None;
        }
        if (self.type_registry.is_enum_type(target_type) || self.type_registry.is_enum_type(value_type)) && target_type != value_type {
            self.errors.lock().unwrap().report_error(
                let_stmt.get_eq_token().span.clone(),
                CompilationErrorType::EnumAssignmentTypeMismatch(self.source_type_name(target_type), self.source_type_name(value_type)),
            );
            return VariableType::None;
        }
        // A multitype value carries its type at run time, so there is nothing here to
        // disagree with; this is what lets FOREACH hand an element to a typed variable.
        if target_type != value_type
            && value_type != VariableType::None
            && (matches!(target_type, VariableType::UserData(_)) || matches!(value_type, VariableType::UserData(_)))
        {
            self.errors.lock().unwrap().report_error(
                let_stmt.get_eq_token().span.clone(),
                CompilationErrorType::AssignmentTypeMismatch(target_type, value_type),
            );
        }
        VariableType::None
    }

    fn visit_for_statement(&mut self, for_stmt: &crate::ast::ForStatement) -> VariableType {
        if let Some(idx) = self.lookup_variable(for_stmt.get_identifier()) {
            if self.references_are_reachable {
                self.reference_owners.entry(idx).or_default().insert(self.cur_func_impl);
            }
            let (_rt, r) = &mut self.references[idx];
            let identifier = for_stmt.get_identifier_token();
            r.usages
                .push((self.current_file.clone(), Spanned::new(identifier.token.to_string(), identifier.span.clone())));
        } else {
            self.errors.lock().unwrap().report_error(
                for_stmt.get_identifier_token().span.clone(),
                CompilationErrorType::VariableNotFound(for_stmt.get_identifier().to_string()),
            );
        }
        crate::ast::walk_for_stmt(self, for_stmt);
        self.reject_bare_array_value(for_stmt.get_start_expr());
        self.reject_bare_array_value(for_stmt.get_end_expr());
        if let Some(step) = for_stmt.get_step_expr() {
            self.reject_bare_array_value(step);
        }
        VariableType::None
    }

    fn visit_foreach_statement(&mut self, foreach_stmt: &crate::ast::ForEachStatement) -> VariableType {
        if let Some(index) = self.lookup_variable(foreach_stmt.get_identifier()) {
            self.add_reference_to(foreach_stmt.get_identifier_token(), index);
        } else {
            self.errors.lock().unwrap().report_error(
                foreach_stmt.get_identifier_token().span.clone(),
                CompilationErrorType::VariableNotFound(foreach_stmt.get_identifier().to_string()),
            );
        }
        crate::ast::walk_foreach_stmt(self, foreach_stmt);
        VariableType::None
    }

    fn visit_case_specifier(&mut self, case_specifier: &crate::ast::CaseSpecifier) -> VariableType {
        match case_specifier {
            crate::ast::CaseSpecifier::Expression(expression) => {
                expression.visit(self);
                self.reject_bare_array_value(expression);
            }
            crate::ast::CaseSpecifier::FromTo(from, to) => {
                from.visit(self);
                to.visit(self);
                self.reject_bare_array_value(from);
                self.reject_bare_array_value(to);
            }
        }
        VariableType::None
    }

    fn visit_const_declaration_statement(&mut self, const_decl: &ConstDeclarationStatement) -> VariableType {
        // The value is never read at runtime, so walking it with the semantic visitor
        // would put literals nobody uses into the variable table. Collect only names
        // here so references to earlier constants still support navigation and hover.
        #[derive(Default)]
        struct ConstantReferences(Vec<Spanned<Token>>);

        impl AstVisitor<()> for ConstantReferences {
            fn visit_identifier_expression(&mut self, identifier: &IdentifierExpression) {
                self.0.push(identifier.get_identifier_token().clone());
            }
        }

        if self.has_variable_defined(const_decl.get_identifier()) {
            self.errors.lock().unwrap().report_error(
                const_decl.get_identifier_token().span.clone(),
                CompilationErrorType::VariableAlreadyDefined(const_decl.get_identifier().to_string()),
            );
            return VariableType::None;
        }

        let value = const_value_with_members(
            const_decl.get_value(),
            &|id| self.lookup_constant(id).map(|(_, value, _)| value.clone()),
            &|type_name, member| {
                self.type_registry
                    .get_enum(type_name)
                    .and_then(|definition| definition.value(member))
                    .map(VariableValue::new_int)
            },
        );
        let Some(value) = value else {
            self.errors
                .lock()
                .unwrap()
                .report_error(const_decl.get_value().get_span(), CompilationErrorType::ConstantValueExpected);
            return VariableType::None;
        };

        let mut constant_references = ConstantReferences::default();
        const_decl.get_value().visit(&mut constant_references);
        for identifier in constant_references.0 {
            if let Token::Identifier(name) = &identifier.token
                && let Some(reference_index) = self.lookup_constant(name).map(|(_, _, reference_index)| *reference_index)
            {
                self.add_reference_to(&identifier, reference_index);
            }
        }

        let declared_type = const_decl.get_variable_type();
        if self.type_registry.is_enum_type(declared_type) {
            let actual = self.declared_constant_type(const_decl.get_value()).unwrap_or_else(|| value.get_type());
            if actual != declared_type {
                self.errors.lock().unwrap().report_error(
                    const_decl.get_value().get_span(),
                    CompilationErrorType::EnumAssignmentTypeMismatch(self.source_type_name(declared_type), self.source_type_name(actual)),
                );
                return VariableType::None;
            }
        }

        let name = const_decl.get_identifier().clone();
        // An enum keeps the value its member stands for; converting to the type itself would mean nothing.
        let entry = if self.type_registry.is_enum_type(declared_type) {
            (declared_type, value)
        } else if self.lang_version >= 400 && declared_type == VariableType::String {
            (declared_type, value)
        } else {
            (declared_type, value.convert_to(declared_type))
        };
        let reference_index = self.references.len();
        self.references.push((
            ReferenceType::Constant(reference_index),
            References {
                variable_type: entry.0,
                variable_table_index: 0,
                header: None,
                declaration: Some((
                    self.current_file.clone(),
                    Spanned::new(
                        const_decl.get_identifier_token().token.to_string(),
                        const_decl.get_identifier_token().span.clone(),
                    ),
                )),
                implementation: None,
                return_types: vec![],
                usages: vec![],
            },
        ));
        let entry = (entry.0, entry.1, reference_index);
        if let Some(local) = &mut self.local_constants {
            local.insert(name, entry);
        } else {
            self.global_constants.insert(name, entry);
        }
        VariableType::None
    }

    fn visit_variable_declaration_statement(&mut self, var_decl: &VariableDeclarationStatement) -> VariableType {
        for v in var_decl.get_variables() {
            if self.has_variable_defined(v.get_identifier()) {
                self.errors.lock().unwrap().report_error(
                    v.get_identifier_token().span.clone(),
                    CompilationErrorType::VariableAlreadyDefined(v.get_identifier().to_string()),
                );
                continue;
            }
            let (dims, vs) = if let Some(Expression::ArrayInitializer(arr_expr)) = v.get_initalizer() {
                for expr in arr_expr.get_expressions() {
                    expr.visit(self);
                }
                (1, arr_expr.get_expressions().len().saturating_sub(1))
            } else {
                if let Some(initializer) = v.get_initalizer() {
                    initializer.visit(self);
                }
                (v.get_dimensions().len() as u8, v.get_vector_size())
            };
            self.add_variable(
                var_decl.get_variable_type(),
                v.get_identifier_token(),
                dims,
                vs,
                v.get_matrix_size(),
                v.get_cube_size(),
            );
        }
        VariableType::None
    }

    fn visit_procedure_call_statement(&mut self, call: &ProcedureCallStatement) -> VariableType {
        let mut found = false;
        if let Some(idx) = self.lookup_variable(call.get_identifier()) {
            if matches!(self.references[idx].0, ReferenceType::Variable(_)) {
                self.add_reference_to(call.get_identifier_token(), idx);
                found = true;
            }

            if matches!(self.references[idx].0, ReferenceType::Function(_)) {
                let f = self.function_containers.iter().find(|p| p.name == call.get_identifier()).unwrap();
                if let FunctionDeclaration::Function(f) = &f.functions.clone() {
                    let param_count = f.get_parameters().len();
                    let arg_count = call.get_arguments().len();
                    let identifier_token = call.get_identifier_token();
                    self.check_arg_count(param_count, arg_count, identifier_token);
                    self.check_arg_types(f.get_parameters(), call.get_arguments());
                }

                self.add_reference_to(call.get_identifier_token(), idx);
                found = true;
            }

            if matches!(self.references[idx].0, ReferenceType::Procedure(_)) {
                let func_container = self.function_containers.iter().find(|p| p.name == call.get_identifier()).unwrap();

                if let FunctionDeclaration::Procedure(f) = &func_container.functions.clone() {
                    let arg_count = call.get_arguments().len();
                    let par_len = f.get_parameters().len();

                    self.check_arg_count(par_len, arg_count, call.get_identifier_token());
                    let arg_count = arg_count.min(par_len);
                    let pass_flags = f.get_pass_flags();
                    self.check_arg_types(f.get_parameters(), call.get_arguments());

                    for i in 0..arg_count.min(u16::BITS as usize) {
                        if 1u16.checked_shl(i as u32).is_some_and(|mask| pass_flags & mask != 0) {
                            self.check_argument_is_variable(i, &call.get_arguments()[i]);
                        }
                    }
                }

                self.add_reference_to(call.get_identifier_token(), idx);
                found = true;
            }
        }

        if !found {
            if self.lang_version < 350 {
                self.errors.lock().unwrap().report_error(
                    call.get_identifier_token().span.clone(),
                    CompilationErrorType::ProcedureNotFound(call.get_identifier().to_string()),
                );
            } else {
                let id = self.add_declaration(VariableType::Procedure, call.get_identifier_token());
                self.global_lookup.variable_lookup.insert(call.get_identifier().clone(), id);
                self.function_containers.push(FunctionContainer {
                    name: call.get_identifier().clone(),
                    parameter_index: None,
                    id,
                    functions: FunctionDeclaration::Procedure(ProcedureDeclarationAstNode::empty(
                        call.get_identifier().clone(),
                        call.get_arguments()
                            .iter()
                            .map(|_a| ParameterSpecifier::Variable(VariableParameterSpecifier::empty(false, VariableType::None, None)))
                            .collect(),
                    )),
                    lookup: VariableLookups::default(),
                    parameters: 0..0,
                    local_variables: 0..0,
                });
                return self.visit_procedure_call_statement(call);
            }
        }

        walk_procedure_call_statement(self, call);
        VariableType::None
    }

    fn visit_function_declaration(&mut self, func_decl: &FunctionDeclarationAstNode) -> VariableType {
        if self.has_variable_defined(func_decl.get_identifier()) {
            self.errors.lock().unwrap().report_error(
                func_decl.get_identifier_token().span.clone(),
                CompilationErrorType::VariableAlreadyDefined(func_decl.get_identifier().to_string()),
            );
            return VariableType::None;
        }
        let id = self.add_declaration(VariableType::Function, func_decl.get_identifier_token());
        self.global_lookup.variable_lookup.insert(func_decl.get_identifier().clone(), id);
        self.function_containers.push(FunctionContainer {
            name: func_decl.get_identifier().clone(),
            parameter_index: None,
            id,
            functions: FunctionDeclaration::Function(func_decl.clone()),
            lookup: VariableLookups::default(),
            parameters: 0..0,
            local_variables: 0..0,
        });
        VariableType::None
    }

    fn visit_function_implementation(&mut self, function: &FunctionImplementation) -> VariableType {
        if let Some(idx) = self.lookup_variable(function.get_identifier()) {
            // Procedure call may've added a function wrongly as a procedure, fix that here.
            {
                let (ref_kind, refs) = &mut self.references[idx];
                match ref_kind.clone() {
                    ReferenceType::Procedure(container_idx) => {
                        // Switch the reference kind.
                        *ref_kind = ReferenceType::Function(container_idx);
                        // Update semantic type.
                        refs.variable_type = VariableType::Function;
                        if let Some(h) = refs.header.as_mut() {
                            h.variable_type = VariableType::Function;
                        }
                    }
                    ReferenceType::Function(_) => {
                        // All good.
                    }
                    _ => {
                        self.errors.lock().unwrap().report_error(
                            function.get_identifier_token().span.clone(),
                            CompilationErrorType::InternalError(format!(
                                "Internal error: Found function implementation for non-procedure: {}",
                                function.get_identifier()
                            )),
                        );
                    }
                }
            }

            let identifier = function.get_identifier_token();
            self.cur_func_impl = Some(idx);
            self.references[idx].1.implementation = Some((self.current_file.clone(), Spanned::new(identifier.token.to_string(), identifier.span.clone())));
            for cont in &mut self.function_containers {
                if cont.id == idx {
                    let documentation = match &cont.functions {
                        FunctionDeclaration::Function(declaration) => declaration.get_documentation(),
                        FunctionDeclaration::Procedure(declaration) => declaration.get_documentation(),
                    }
                    .or_else(|| function.get_documentation())
                    .map(str::to_owned);
                    if let FunctionDeclaration::Function(func) = &cont.functions {
                        if func.get_parameters().len() != function.get_parameters().len() {
                            self.errors.lock().unwrap().report_error(
                                function.get_identifier_token().span.clone(),
                                CompilationErrorType::ParameterMismatch(function.get_identifier().to_string()),
                            );
                        }
                        if func.get_return_type() != function.get_return_type() || func.get_return_rank() != function.get_return_rank() {
                            self.errors.lock().unwrap().report_error(
                                function.get_return_type_token().span.clone(),
                                CompilationErrorType::ReturnTypeMismatch(function.get_identifier().to_string()),
                            );
                        } // may've been wrongly added as procedure before - get's corrected.
                    } else if let FunctionDeclaration::Procedure(func) = &cont.functions
                        && func.get_parameters().len() != function.get_parameters().len()
                    {
                        self.errors.lock().unwrap().report_error(
                            function.get_identifier_token().span.clone(),
                            CompilationErrorType::ParameterMismatch(function.get_identifier().to_string()),
                        );
                    }
                    cont.functions = FunctionDeclaration::Function(
                        FunctionDeclarationAstNode::empty(function.get_identifier().clone(), function.get_parameters().clone(), function.get_return_type())
                            .with_return_rank(function.get_return_rank())
                            .with_documentation(documentation.as_deref()),
                    );
                    break;
                }
            }
        } else if self.lang_version < 350 {
            self.errors.lock().unwrap().report_error(
                function.get_identifier_token().span.clone(),
                CompilationErrorType::FunctionNotFound(function.get_identifier().to_string()),
            );
        } else {
            let id = self.add_declaration(VariableType::Function, function.get_identifier_token());
            self.cur_func_impl = Some(id);
            self.global_lookup.variable_lookup.insert(function.get_identifier().clone(), id);

            self.function_containers.push(FunctionContainer {
                name: function.get_identifier().clone(),
                parameter_index: None,
                id,
                functions: FunctionDeclaration::Function(
                    FunctionDeclarationAstNode::empty(function.get_identifier().clone(), function.get_parameters().clone(), function.get_return_type())
                        .with_documentation(function.get_documentation()),
                ),
                lookup: VariableLookups::default(),
                parameters: 0..0,
                local_variables: 0..0,
            });
        }

        self.start_parse_function_body();
        let start_parameter = self.references.len();
        self.add_parameters(function.get_parameters());
        let end_parameter = self.references.len();

        let start_locals = self.references.len();
        self.visit_statement_sequence(function.get_statements());
        let end_locals = self.references.len();
        let lookup = self.end_parse_function_body().unwrap();
        self.cur_func_impl = None;

        for f in &mut self.function_containers {
            if f.name == function.get_identifier() {
                f.lookup = lookup;
                f.parameters = start_parameter..end_parameter;
                f.local_variables = start_locals..end_locals;
                break;
            }
        }
        VariableType::None
    }

    fn visit_procedure_declaration(&mut self, proc_decl: &ProcedureDeclarationAstNode) -> VariableType {
        if self.has_variable_defined(proc_decl.get_identifier()) {
            self.errors.lock().unwrap().report_error(
                proc_decl.get_identifier_token().span.clone(),
                CompilationErrorType::VariableAlreadyDefined(proc_decl.get_identifier().to_string()),
            );
            return VariableType::None;
        }

        let id = self.add_declaration(VariableType::Procedure, proc_decl.get_identifier_token());
        self.global_lookup.variable_lookup.insert(proc_decl.get_identifier().clone(), id);

        self.function_containers.push(FunctionContainer {
            name: proc_decl.get_identifier().clone(),
            parameter_index: None,
            id,
            functions: FunctionDeclaration::Procedure(proc_decl.clone()),
            lookup: VariableLookups::default(),
            parameters: 0..0,
            local_variables: 0..0,
        });
        VariableType::None
    }

    fn visit_procedure_implementation(&mut self, procedure: &ProcedureImplementation) -> VariableType {
        if let Some(idx) = self.lookup_variable(procedure.get_identifier()) {
            // Procedure call may've added a function wrongly as a procedure, fix that here.
            {
                let (ref_kind, _refs) = &mut self.references[idx];
                match ref_kind.clone() {
                    ReferenceType::Procedure(_container_idx) => {
                        // All good.
                    }
                    ReferenceType::Function(_) => {
                        self.errors
                            .lock()
                            .unwrap()
                            .report_error(procedure.get_identifier_token().span.clone(), CompilationErrorType::ProcedureUsedAsFunction);
                    }
                    _ => {
                        self.errors.lock().unwrap().report_error(
                            procedure.get_identifier_token().span.clone(),
                            CompilationErrorType::InternalError(format!(
                                "Internal error: Found function implementation for non-procedure: {}",
                                procedure.get_identifier()
                            )),
                        );
                    }
                }
            }

            let identifier = procedure.get_identifier_token();
            self.references[idx].1.implementation = Some((self.current_file.clone(), Spanned::new(identifier.token.to_string(), identifier.span.clone())));
            for cont in &mut self.function_containers {
                if cont.id == idx {
                    let documentation = match &cont.functions {
                        FunctionDeclaration::Function(declaration) => declaration.get_documentation(),
                        FunctionDeclaration::Procedure(declaration) => declaration.get_documentation(),
                    }
                    .or_else(|| procedure.get_documentation())
                    .map(str::to_owned);
                    if let FunctionDeclaration::Procedure(func) = &cont.functions
                        && func.get_parameters().len() != procedure.get_parameters().len()
                    {
                        self.errors.lock().unwrap().report_error(
                            procedure.get_identifier_token().span.clone(),
                            CompilationErrorType::ParameterMismatch(procedure.get_identifier().to_string()),
                        );
                    }
                    cont.functions = FunctionDeclaration::Procedure(
                        ProcedureDeclarationAstNode::empty(procedure.get_identifier().clone(), procedure.get_parameters().clone())
                            .with_documentation(documentation.as_deref()),
                    );
                    break;
                }
            }
        } else if self.lang_version < 350 {
            self.errors.lock().unwrap().report_error(
                procedure.get_identifier_token().span.clone(),
                CompilationErrorType::ProcedureNotFound(procedure.get_identifier().to_string()),
            );
        } else {
            let id = self.add_declaration(VariableType::Procedure, procedure.get_identifier_token());
            self.global_lookup.variable_lookup.insert(procedure.get_identifier().clone(), id);
            self.references[id].1.implementation = Some((
                self.current_file.clone(),
                Spanned::new(
                    procedure.get_identifier_token().token.to_string(),
                    procedure.get_identifier_token().span.clone(),
                ),
            ));
            self.function_containers.push(FunctionContainer {
                name: procedure.get_identifier().clone(),
                parameter_index: None,
                id,
                functions: FunctionDeclaration::Procedure(
                    ProcedureDeclarationAstNode::empty(procedure.get_identifier().clone(), procedure.get_parameters().clone())
                        .with_documentation(procedure.get_documentation()),
                ),
                lookup: VariableLookups::default(),
                parameters: 0..0,
                local_variables: 0..0,
            });
        }

        let procedure_id = self.lookup_variable(procedure.get_identifier());
        self.cur_func_impl = procedure_id;
        self.start_parse_function_body();
        let start_parameter = self.references.len();
        self.add_parameters(procedure.get_parameters());
        let end_parameter = self.references.len();

        let start_locals = self.references.len();
        self.visit_statement_sequence(procedure.get_statements());
        let end_locals = self.references.len();
        let lookup = self.end_parse_function_body().unwrap();
        self.cur_func_impl = None;

        for f in &mut self.function_containers {
            if f.name == procedure.get_identifier() {
                if let FunctionDeclaration::Procedure(decl) = &f.functions
                    && decl.get_parameters().len() != procedure.get_parameters().len()
                {
                    self.errors.lock().unwrap().report_error(
                        procedure.get_identifier_token().span.clone(),
                        CompilationErrorType::ParameterMismatch(procedure.get_identifier().to_string()),
                    );
                }
                f.lookup = lookup;

                f.parameters = start_parameter..end_parameter;
                f.local_variables = start_locals..end_locals;
                break;
            }
        }
        VariableType::None
    }

    /// The layout only reaches the PPE from `FIRST_TYPE_TABLE_RUNTIME` on, so an older
    /// target would drop it and leave every field access reading nothing.
    fn visit_type_declaration(&mut self, type_decl: &TypeDeclarationAstNode) -> VariableType {
        if self.runtime < FIRST_TYPE_TABLE_RUNTIME {
            self.errors.lock().unwrap().report_error(
                type_decl.get_identifier_token().span.clone(),
                ParserErrorType::TypeNeedsNewerRuntime(FIRST_TYPE_TABLE_RUNTIME),
            );
        }
        VariableType::None
    }

    fn visit_ast(&mut self, program: &crate::ast::Ast) -> VariableType {
        // Each file says which language it was read as, so the checks follow it.
        self.lang_version = program.language_version;
        // A routine may be called before the file gets to it, so every signature is
        // registered first - the same thing an explicit DECLARE does. A routine that
        // has one is left to it, so its own checks still run.
        let declared: Vec<unicase::Ascii<String>> = program
            .nodes
            .iter()
            .filter_map(|node| match node {
                crate::ast::AstNode::FunctionDeclaration(declaration) => Some(declaration.get_identifier().clone()),
                crate::ast::AstNode::ProcedureDeclaration(declaration) => Some(declaration.get_identifier().clone()),
                _ => None,
            })
            .collect();
        for node in &program.nodes {
            match node {
                crate::ast::AstNode::Function(function) if !declared.contains(function.get_identifier()) => self.predeclare_function(function),
                crate::ast::AstNode::Procedure(procedure) if !declared.contains(procedure.get_identifier()) => self.predeclare_procedure(procedure),
                _ => {}
            }
        }
        for node in &program.nodes {
            match node {
                crate::ast::AstNode::Function(_) | crate::ast::AstNode::Procedure(_) => {}
                _ => {
                    node.visit(self);
                }
            }
        }
        for node in &program.nodes {
            match node {
                crate::ast::AstNode::Function(_) | crate::ast::AstNode::Procedure(_) => {
                    node.visit(self);
                }
                _ => {}
            }
        }

        VariableType::None
    }
}
