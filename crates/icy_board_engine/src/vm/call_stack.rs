use crate::Res;
use crate::ast::constant::STACK_LIMIT;
use crate::executable::{GenericVariableData, PPEExpr, VariableValue, variable_table::VARIABLE_FLAG_ARRAY_PARAMETER};
use crate::icy_board::state::ppl_error::{ERR_KIND_STACK, ERR_STACK, PplError};

use super::{ErrorHandler, LValuePathStep, ResolvedLValuePathStep, ReturnAddress, VMError, VirtualMachine, WriteBackTarget};

impl VirtualMachine<'_> {
    pub(super) fn is_legacy_array_parameter(&self, parameter: usize) -> bool {
        let header = &self.variable_table.get_var_entry(parameter).header;
        header.dim > 0
            && !matches!(
                header.variable_type,
                crate::executable::VariableType::Function | crate::executable::VariableType::Procedure
            )
            && !(self.variable_table.get_version() >= 400 && header.flags & VARIABLE_FLAG_ARRAY_PARAMETER != 0)
    }

    pub(super) fn call_parameter_value(&self, parameter: usize) -> VariableValue {
        let value = self.variable_table.get_value(parameter);
        if self.is_legacy_array_parameter(parameter) {
            value.get_array_value(0, 0, 0)
        } else {
            value.clone()
        }
    }

    pub(super) fn set_call_parameter(&mut self, parameter: usize, value: VariableValue) -> Res<()> {
        let expected = self.variable_table.get_var_entry(parameter).header.variable_type;
        let value = self.variable_table.checked_enum_value(expected, value)?;
        if self.is_legacy_array_parameter(parameter) {
            // SCREXEC assigns through *varLst[id]->data: only element zero,
            // never the parameter array's storage or its persistent tail.
            if let Some(target) = self.variable_table.get_value_mut(parameter).get_array_value_mut(0, 0, 0) {
                *target = super::decay_array(value).convert_to(target.vtype);
            }
        } else {
            self.variable_table.set_value(parameter, value);
        }
        Ok(())
    }

    /// The destination parameter header determines whether an argument is an
    /// array value. Routine-reference headers also use dim, but for arity.
    fn is_whole_array_parameter(&self, parameter: usize) -> bool {
        let header = &self.variable_table.get_var_entry(parameter).header;
        self.variable_table.get_version() >= 400
            && header.dim > 0
            && header.flags & VARIABLE_FLAG_ARRAY_PARAMETER != 0
            && !matches!(
                header.variable_type,
                crate::executable::VariableType::Function | crate::executable::VariableType::Procedure
            )
    }

    async fn eval_call_argument(&mut self, parameter: usize, argument: &PPEExpr) -> Res<VariableValue> {
        if self.is_whole_array_parameter(parameter) {
            self.eval_array_operand(argument).await
        } else {
            self.eval_expr(argument).await
        }
    }

    #[allow(clippy::needless_range_loop)]
    pub(super) async fn prepare_call(&mut self, locals: usize, parameters: usize, first: usize, arguments: &[PPEExpr], pass_modes: &[bool]) -> Res<()> {
        let mut values = Vec::with_capacity(parameters);
        let mut targets = Vec::new();
        for (i, argument) in arguments.iter().take(parameters).enumerate() {
            let value = if pass_modes.get(i).copied().unwrap_or(false) {
                let (target, mut value) = self.resolve_write_back_target(argument).await?;
                if !self.is_whole_array_parameter(first + i) && value.get_dimensions() > 0 {
                    value = self.read_array_element(&value, 0, 0, 0)?;
                }
                targets.push(target);
                value
            } else {
                self.eval_call_argument(first + i, argument).await?
            };
            values.push(value);
        }
        self.save_call_frame(locals, parameters, first)?;
        for (i, value) in values.into_iter().enumerate() {
            let id = first + i;
            self.set_call_parameter(id, value)?;
        }
        self.write_back_stack.extend(targets);
        Ok(())
    }

    async fn resolve_write_back_target(&mut self, argument: &PPEExpr) -> Res<(WriteBackTarget, VariableValue)> {
        let mut root = argument;
        let mut path = Vec::new();
        loop {
            match root {
                PPEExpr::Member(base, member) => {
                    path.push(LValuePathStep::Member(*member));
                    root = base;
                }
                PPEExpr::IndexedMember(base, member, indices) => {
                    path.push(LValuePathStep::IndexedMember(*member, indices));
                    root = base;
                }
                _ => break,
            }
        }
        path.reverse();
        let (root_id, root_indices) = match root {
            PPEExpr::Value(id) => (*id, None),
            PPEExpr::Dim(id, indices) => (*id, Some(self.eval_array_indices(indices).await?)),
            _ => return Err(VMError::InternalVMError.into()),
        };
        let mut value = self.variable_table.get_value(root_id).clone();
        if let Some((first, second, third)) = root_indices {
            value = self.read_array_element(&value, first, second, third)?;
        }
        let mut resolved = Vec::with_capacity(path.len());
        for step in path {
            let GenericVariableData::Record(fields) = &value.generic_data else {
                return Err(VMError::NoUserTypeBase.into());
            };
            match step {
                LValuePathStep::Member(member) => {
                    value = fields.get(member).ok_or(VMError::InternalVMError)?.clone();
                    resolved.push(ResolvedLValuePathStep::Member(member));
                }
                LValuePathStep::IndexedMember(member, indices) => {
                    let field = fields.get(member).ok_or(VMError::InternalVMError)?.clone();
                    let (first, second, third) = self.eval_array_indices(indices).await?;
                    value = self.read_array_element(&field, first, second, third)?;
                    resolved.push(ResolvedLValuePathStep::IndexedMember(member, first, second, third));
                }
            }
        }
        Ok((
            WriteBackTarget {
                root_id,
                root_indices,
                path: resolved,
            },
            value,
        ))
    }

    pub(super) async fn set_write_back_target(&mut self, target: &WriteBackTarget, value: VariableValue) -> Res<()> {
        if !target.path.is_empty() {
            return self.set_record_target(target, value);
        }
        if let Some((first, second, third)) = target.root_indices {
            let element_type = self.variable_table.get_var_entry(target.root_id).header.variable_type;
            let value = self.variable_table.checked_enum_value(element_type, value)?;
            self.check_nominal_assignment(element_type, value.vtype)?;
            self.check_record_assignment(element_type, 0, &value)?;
            self.variable_table.get_value_mut(target.root_id).set_array_value(first, second, third, value)?;
            Ok(())
        } else {
            self.set_variable(&PPEExpr::Value(target.root_id), value).await
        }
    }

    /// The same, for a call the VM makes itself and so has the arguments of already.
    pub(super) fn prepare_call_with_values(&mut self, locals: usize, parameters: usize, first: usize, arguments: Vec<VariableValue>) -> Res<()> {
        self.save_call_frame(locals, parameters, first)?;
        for (i, value) in arguments.into_iter().take(parameters).enumerate() {
            self.set_call_parameter(first + i, value)?;
        }
        Ok(())
    }

    fn save_call_frame(&mut self, locals: usize, parameters: usize, first: usize) -> Res<()> {
        // Resolve fallible defaults before changing any of the caller's slots.
        let defaults = (0..locals + parameters)
            .map(|i| {
                let id = first + i;
                let entry = self.variable_table.get_var_entry(id);
                if (i < parameters && self.is_legacy_array_parameter(id)) || entry.header.flags & crate::executable::variable_table::VARIABLE_FLAG_STATIC != 0 {
                    Ok(None)
                } else {
                    self.local_default(entry).map(Some)
                }
            })
            .collect::<Res<Vec<_>>>()?;
        for (i, empty) in defaults.into_iter().enumerate() {
            let id = first + i;
            if i < parameters && self.is_legacy_array_parameter(id) {
                // stkinit/stkclean save and restore only parameter element zero;
                // initLocals starts after the parameters, so tails are not reset.
                self.call_local_value_stack.push(self.call_parameter_value(id));
                continue;
            }
            if let Some(empty) = empty {
                let value = std::mem::replace(self.variable_table.get_value_mut(id), empty);
                self.call_local_value_stack.push(value);
            }
        }
        Ok(())
    }

    pub(super) fn goto(&mut self, label: usize) -> Result<(), VMError> {
        if let Some((cached_label, statement)) = self.last_jump
            && cached_label == label
        {
            self.cur_ptr = statement;
            return Ok(());
        }
        if let Some(statement) = self.label_table.get(&label) {
            self.cur_ptr = *statement;
            self.last_jump = Some((label, *statement));
            Ok(())
        } else {
            Err(VMError::LabelNotFound(label))
        }
    }

    /// Whether another call fits on the stack.
    ///
    /// Once it is exhausted the PPE either ends here or, if it turned
    /// `STACKABORT` off, skips the call and carries on with the next statement.
    /// That is as far as "continue after a stack error" can sensibly go.
    pub(super) fn has_stack_room(&mut self) -> Res<bool> {
        if (self.return_addresses.len() as i32) < STACK_LIMIT {
            return Ok(true);
        }
        // A handler is given the chance to clean up, which aborting would take away.
        if self.error_handler != ErrorHandler::Off && !self.in_handler {
            self.set_error(PplError::new(ERR_KIND_STACK, ERR_STACK, "PPE call stack exhausted"));
            return Ok(false);
        }
        if self.abort_on_stack_error {
            return Err(Box::new(VMError::StackOverflow));
        }
        log::warn!("PPE stack exhausted, skipping the call because STACKABORT is off");
        Ok(false)
    }

    /// Takes a call, and reports whether there was room for it.
    pub(super) fn push_return_address(&mut self, address: ReturnAddress) -> Res<bool> {
        if !self.has_stack_room()? {
            return Ok(false);
        }
        self.return_addresses.push(address);
        Ok(true)
    }
}
