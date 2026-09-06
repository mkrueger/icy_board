use crate::Res;
use crate::ast::constant::STACK_LIMIT;
use crate::executable::{PPEExpr, VariableValue, variable_table::VARIABLE_FLAG_ARRAY_PARAMETER};
use crate::icy_board::state::ppl_error::{ERR_KIND_STACK, ERR_STACK, PplError};

use super::{ErrorHandler, ReturnAddress, VMError, VirtualMachine};

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
    async fn eval_call_argument(&mut self, parameter: usize, argument: &PPEExpr) -> Res<VariableValue> {
        let header = &self.variable_table.get_var_entry(parameter).header;
        if self.variable_table.get_version() >= 400
            && header.dim > 0
            && header.flags & VARIABLE_FLAG_ARRAY_PARAMETER != 0
            && !matches!(
                header.variable_type,
                crate::executable::VariableType::Function | crate::executable::VariableType::Procedure
            )
        {
            self.eval_array_operand(argument).await
        } else {
            self.eval_expr(argument).await
        }
    }

    #[allow(clippy::needless_range_loop)]
    pub(super) async fn prepare_call(&mut self, locals: usize, parameters: usize, first: usize, arguments: &[PPEExpr], pass_flags: u16) -> Res<()> {
        if parameters <= 1 {
            let value = match (parameters, arguments.first()) {
                (1, Some(argument)) => Some(self.eval_call_argument(first, argument).await?),
                _ => None,
            };
            self.save_call_frame(locals, parameters, first);
            if let Some(value) = value {
                self.set_call_parameter(first, value)?;
                if pass_flags & 1 != 0 {
                    self.write_back_stack.push(arguments[0].clone());
                }
            }
            return Ok(());
        }

        let mut values = Vec::with_capacity(parameters);
        for (i, argument) in arguments.iter().take(parameters).enumerate() {
            values.push(self.eval_call_argument(first + i, argument).await?);
        }
        self.save_call_frame(locals, parameters, first);
        for (i, value) in values.into_iter().enumerate() {
            let id = first + i;
            self.set_call_parameter(id, value)?;

            if 1u16.checked_shl(i as u32).is_some_and(|mask| mask & pass_flags != 0) {
                self.write_back_stack.push(arguments[i].clone());
            }
        }
        Ok(())
    }

    /// The same, for a call the VM makes itself and so has the arguments of already.
    pub(super) fn prepare_call_with_values(&mut self, locals: usize, parameters: usize, first: usize, arguments: Vec<VariableValue>) -> Res<()> {
        self.save_call_frame(locals, parameters, first);
        for (i, value) in arguments.into_iter().take(parameters).enumerate() {
            self.set_call_parameter(first + i, value)?;
        }
        Ok(())
    }

    fn save_call_frame(&mut self, locals: usize, parameters: usize, first: usize) {
        for i in 0..(locals + parameters) {
            let id = first + i;
            if i < parameters && self.is_legacy_array_parameter(id) {
                // stkinit/stkclean save and restore only parameter element zero;
                // initLocals starts after the parameters, so tails are not reset.
                self.call_local_value_stack.push(self.call_parameter_value(id));
                continue;
            }
            let entry = self.variable_table.get_var_entry(id);
            if entry.header.flags & crate::executable::variable_table::VARIABLE_FLAG_STATIC == 0 {
                let empty =
                    if self.variable_table.get_version() >= 400 && entry.header.flags & crate::executable::variable_table::VARIABLE_FLAG_DYNAMIC_ARRAY != 0 {
                        VariableValue {
                            vtype: entry.value.vtype,
                            data: entry.value.data,
                            generic_data: entry.header.create_generic_data().unwrap_or_default(),
                        }
                    } else {
                        entry.value.emptied()
                    };
                let value = std::mem::replace(self.variable_table.get_value_mut(id), empty);
                self.call_local_value_stack.push(value);
            }
        }
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
