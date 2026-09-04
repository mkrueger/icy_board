use crate::Res;
use crate::ast::constant::STACK_LIMIT;
use crate::executable::{PPEExpr, VariableValue};
use crate::icy_board::state::ppl_error::{ERR_KIND_STACK, ERR_STACK, PplError};

use super::{ErrorHandler, ReturnAddress, VMError, VirtualMachine};

impl VirtualMachine<'_> {
    #[allow(clippy::needless_range_loop)]
    pub(super) async fn prepare_call(&mut self, locals: usize, parameters: usize, first: usize, arguments: &[PPEExpr], pass_flags: u16) -> Res<()> {
        let mut values = Vec::with_capacity(parameters);
        for argument in arguments.iter().take(parameters) {
            values.push(self.eval_expr(argument).await?);
        }
        self.save_call_frame(locals, parameters, first);
        for (i, value) in values.into_iter().enumerate() {
            let id = first + i;
            self.variable_table.set_value(id, value);

            if 1u16.checked_shl(i as u32).is_some_and(|mask| mask & pass_flags != 0) {
                self.write_back_stack.push(arguments[i].clone());
            }
        }
        Ok(())
    }

    /// The same, for a call the VM makes itself and so has the arguments of already.
    pub(super) fn prepare_call_with_values(&mut self, locals: usize, parameters: usize, first: usize, arguments: Vec<VariableValue>) {
        self.save_call_frame(locals, parameters, first);
        for (i, value) in arguments.into_iter().take(parameters).enumerate() {
            self.variable_table.set_value(first + i, value);
        }
    }

    fn save_call_frame(&mut self, locals: usize, parameters: usize, first: usize) {
        for i in 0..(locals + parameters) {
            let id = first + i;
            if self.variable_table.get_var_entry(id).header.flags & 0x1 == 0x0 {
                let empty = self.variable_table.get_value(id).emptied();
                let value = std::mem::replace(self.variable_table.get_value_mut(id), empty);
                self.call_local_value_stack.push(value);
            }
        }
    }

    pub(super) fn goto(&mut self, label: usize) -> Result<(), VMError> {
        if let Some(statement) = self.label_table.get(&label) {
            self.cur_ptr = *statement;
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
