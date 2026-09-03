use crate::{ast::Expression, compiler::CompilationErrorType, executable::VariableType, hir::CallId};

use super::{FunctionDeclaration, SemanticInfo, SemanticVisitor};

#[derive(Clone, Debug, PartialEq, Eq)]
pub(super) struct ArrayShape {
    pub(super) element_type: VariableType,
    pub(super) rank: u8,
    pub(super) bounds: [usize; 3],
    pub(super) resizable: bool,
    pub(super) field_name: Option<String>,
}

impl ArrayShape {
    fn source_name(&self) -> String {
        let bounds = self.bounds[..self.rank as usize].iter().map(usize::to_string).collect::<Vec<_>>().join(", ");
        format!("{}({bounds})", self.element_type)
    }

    fn same_layout(&self, other: &Self) -> bool {
        let compatible_elements = self.element_type == other.element_type
            || (matches!(self.element_type, VariableType::String | VariableType::BigStr | VariableType::UnboundedString)
                && matches!(other.element_type, VariableType::String | VariableType::BigStr | VariableType::UnboundedString));
        compatible_elements && self.rank == other.rank && (self.resizable || self.bounds == other.bounds)
    }
}

impl SemanticVisitor {
    pub(super) fn array_shape(&mut self, expression: &Expression) -> Option<ArrayShape> {
        match expression {
            Expression::Identifier(identifier) => {
                let index = self.lookup_variable(identifier.get_identifier())?;
                let reference = &self.references[index].1;
                // A routine reference keeps its parameter count in `dim`, which is not a bound.
                if matches!(reference.variable_type, VariableType::Function | VariableType::Procedure) {
                    return None;
                }
                let header = reference.header.as_ref()?;
                (header.dim > 0).then(|| ArrayShape {
                    element_type: reference.variable_type,
                    rank: header.dim,
                    bounds: [header.vector_size, header.matrix_size, header.cube_size],
                    resizable: true,
                    field_name: None,
                })
            }
            Expression::Parens(parens) => self.array_shape(parens.get_expression()),
            Expression::FunctionCall(call) => {
                if let Some((element_type, rank)) = self.member_array_returns.get(&CallId(call.id)).copied() {
                    return Some(ArrayShape {
                        element_type,
                        rank,
                        bounds: [0; 3],
                        resizable: true,
                        field_name: None,
                    });
                }
                let SemanticInfo::FunctionReference(index) = self.function_type_lookup.get(&CallId(call.id))? else {
                    return None;
                };
                let FunctionDeclaration::Function(function) = &self.function_containers[*index].functions else {
                    return None;
                };
                (function.get_return_rank() > 0).then(|| ArrayShape {
                    element_type: function.get_return_type(),
                    rank: function.get_return_rank(),
                    bounds: [0; 3],
                    resizable: true,
                    field_name: None,
                })
            }
            Expression::MemberReference(member) => {
                let type_id = self.user_type_lookup.get(&member.get_identifier_token().span.start).copied()?;
                if let Some(definition) = self.type_registry.get_record_type_from_id(type_id) {
                    let field = definition.field(definition.field_index(member.get_identifier())?)?;
                    return (field.dim > 0).then(|| ArrayShape {
                        element_type: field.variable_type,
                        rank: field.dim,
                        bounds: [field.vector_size as usize, field.matrix_size as usize, field.cube_size as usize],
                        resizable: false,
                        field_name: Some(member.get_identifier().to_string()),
                    });
                }
                let registry = self.type_registry.get_type_from_id(type_id)?;
                let rank = registry.field_ranks.get(member.get_identifier()).copied()?;
                Some(ArrayShape {
                    element_type: registry.fields.get(member.get_identifier()).copied()?,
                    rank,
                    bounds: [0; 3],
                    resizable: true,
                    field_name: Some(member.get_identifier().to_string()),
                })
            }
            _ => None,
        }
    }

    pub(super) fn is_whole_custom_type_array(&mut self, expression: &Expression) -> bool {
        self.array_shape(expression)
            .is_some_and(|shape| matches!(shape.element_type, VariableType::UserData(_)))
    }

    /// A bare array is not a value: `PCBoard` wanted one subscript per dimension
    /// everywhere a variable was read (`wrVIDSUB`), and only the statements that take a
    /// whole array saw one.
    pub(super) fn reject_bare_array_value(&mut self, expression: &Expression) {
        let Some(shape) = self.array_shape(expression) else {
            return;
        };
        if shape.field_name.is_some() {
            self.errors
                .lock()
                .unwrap()
                .report_error(expression.get_span(), CompilationErrorType::WholeArrayUsedAsScalar);
            return;
        }
        let mut expression = expression;
        while let Expression::Parens(parens) = expression {
            expression = parens.get_expression();
        }
        if let Expression::Identifier(identifier) = expression {
            self.check_arg_count(shape.rank as usize, 0, identifier.get_identifier_token());
        }
    }

    /// Reports what stops `value` from being stored in an array shaped target.
    pub(super) fn check_array_target_assignment(&mut self, target_shape: &ArrayShape, value: &Expression, span: &core::ops::Range<usize>) {
        let field_name = target_shape.field_name.clone().unwrap_or_default();
        match self.array_shape(value) {
            Some(value_shape) if target_shape.same_layout(&value_shape) => {}
            Some(value_shape) => {
                let expected = target_shape.source_name();
                let actual = value_shape.source_name();
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(span.clone(), CompilationErrorType::RecordArrayShapeMismatch(field_name, expected, actual));
            }
            None => {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(span.clone(), CompilationErrorType::RecordArrayValueExpected(field_name));
            }
        }
    }

    pub(super) fn is_assignable_explicit_target(&mut self, expression: &Expression) -> bool {
        match expression {
            Expression::Identifier(_) | Expression::Indexer(_) => true,
            Expression::MemberReference(member) => self.is_assignable_explicit_target(member.get_expression()),
            Expression::FunctionCall(call) => {
                if matches!(
                    self.function_type_lookup.get(&CallId(call.id)),
                    Some(SemanticInfo::IndexedRecordField(_) | SemanticInfo::VariableReference(_))
                ) {
                    return true;
                }
                match call.get_expression() {
                    Expression::Identifier(identifier) => {
                        let Some(index) = self.lookup_variable(identifier.get_identifier()) else {
                            return false;
                        };
                        self.references[index].1.header.as_ref().is_some_and(|header| header.dim > 0)
                    }
                    Expression::MemberReference(member) => {
                        let Some(type_id) = self.user_type_lookup.get(&member.get_identifier_token().span.start).copied() else {
                            return false;
                        };
                        self.type_registry
                            .get_record_type_from_id(type_id)
                            .and_then(|definition| definition.field_index(member.get_identifier()).and_then(|field_id| definition.field(field_id)))
                            .is_some_and(|field| field.dim > 0)
                    }
                    _ => false,
                }
            }
            _ => false,
        }
    }
}
