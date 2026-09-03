use crate::Res;
use crate::executable::VariableType;
use crate::icy_board::state::ppl_error::{
    ERR_FORMAT, ERR_INVALID, ERR_IO, ERR_KIND_DBASE, ERR_KIND_FILE, ERR_KIND_GFX, ERR_LIMIT, ERR_UNAVAILABLE, ERR_UNSUPPORTED, PplError,
};

use super::{ErrorHandler, ReturnAddress, VMError, VirtualMachine};

/// What the graphics codes mean, so `Error.Last().Message` reads like the other subsystems'.
fn gfx_error_message(code: i32) -> &'static str {
    match code {
        ERR_UNAVAILABLE => "graphics are not initialized",
        ERR_INVALID => "invalid surface",
        ERR_IO => "graphics I/O failed",
        ERR_FORMAT => "the image could not be decoded",
        ERR_LIMIT => "a graphics limit was reached",
        ERR_UNSUPPORTED => "the terminal does not support this",
        _ => "graphics operation failed",
    }
}

impl VirtualMachine<'_> {
    /// Records what an operation failed with. `ON ERROR` acts on it once the statement is over,
    /// so the operation itself always runs to its end first.
    pub fn set_error(&mut self, error: PplError) {
        if self.error_pending {
            return;
        }
        self.error_pending = !error.is_ok();
        self.last_error = error;
    }

    /// A success clears an older statement's error, but never a failure from this statement.
    pub fn operation_succeeded(&mut self) {
        if !self.error_pending {
            self.last_error = PplError::default();
        }
    }

    /// What an operation that worked reports, so a later `Error.Last()` does not answer for an older one.
    pub fn clear_error(&mut self) {
        self.error_pending = false;
        self.last_error = PplError::default();
    }

    /// Takes what the graphics code left behind. It writes a plain code rather than
    /// reaching for the VM, so this is where that becomes an error like any other.
    fn publish_gfx_error(&mut self) {
        match std::mem::replace(&mut self.icy_board_state.gfx_error, -1) {
            -1 => {}
            0 => self.operation_succeeded(),
            code => self.set_error(PplError::new(ERR_KIND_GFX, code, gfx_error_message(code))),
        }
    }

    /// The same for the file and dBase channels, which keep their own `FERR`/`DERR` flags.
    fn publish_io_error(&mut self) {
        if let Some(result) = self.io.take_operation_result() {
            match result {
                Ok(()) => self.operation_succeeded(),
                Err((channel, message)) => self.set_error(PplError::new(ERR_KIND_FILE, ERR_IO, message).on_channel(channel)),
            }
        }
        if let Some(result) = self.dbase.take_operation_result() {
            match result {
                Ok(()) => self.operation_succeeded(),
                Err((channel, message)) => self.set_error(PplError::new(ERR_KIND_DBASE, ERR_IO, message).on_channel(channel)),
            }
        }
    }

    /// Makes subsystem inboxes visible to `Error.Last()` and to statement-end trapping.
    pub fn publish_operation_result(&mut self) {
        self.publish_gfx_error();
        self.publish_io_error();
    }

    /// Hands the program to its `ON ERROR` handler, if it has one and is not already in it.
    pub(super) fn check_error_trap(&mut self) -> Res<()> {
        self.publish_operation_result();
        if self.in_handler
            && let Some(depth) = self.handler_depth
            && self.return_addresses.len() <= depth
        {
            self.in_handler = false;
            self.handler_depth = None;
        }

        if !self.error_pending {
            return Ok(());
        }
        self.error_pending = false;
        if self.in_handler {
            return Ok(());
        }

        match self.error_handler {
            ErrorHandler::Off => {}
            ErrorHandler::Goto(label) => {
                self.error_handler = ErrorHandler::Off;
                self.goto(label)?;
            }
            ErrorHandler::Gosub(label) => {
                let depth = self.return_addresses.len();
                self.in_handler = true;
                self.handler_depth = Some(depth);
                if self.push_return_address(ReturnAddress::gosub(self.cur_ptr))? {
                    self.goto(label)?;
                } else {
                    self.in_handler = false;
                    self.handler_depth = None;
                }
            }
            ErrorHandler::Procedure(proc_id) => {
                self.call_error_handler(proc_id)?;
            }
        }
        Ok(())
    }

    /// Calls the `ON ERROR` procedure, handing it the error when it takes one.
    fn call_error_handler(&mut self, proc_id: usize) -> Res<()> {
        let proc_offset;
        let locals;
        let parameters;
        let first;
        let pass_flags;

        unsafe {
            let proc = &self.variable_table.get_var_entry(proc_id);
            proc_offset = proc.value.data.procedure_value.start_offset as usize;
            first = (proc.value.data.procedure_value.first_var_id + 1) as usize;
            locals = proc.value.data.procedure_value.local_variables as usize;
            parameters = proc.value.data.procedure_value.parameters as usize;
            pass_flags = proc.value.data.procedure_value.pass_flags;
        }
        let valid_parameter = parameters == 0
            || (parameters == 1
                && pass_flags == 0
                && self.variable_table.get_var_entry(first).header.variable_type == VariableType::UserData(crate::parser::ERROR_ID as u8));
        if !valid_parameter {
            return Err(VMError::ErrorInFunctionCall("ON ERROR".to_string(), "invalid handler signature".to_string()).into());
        }

        // The handler takes the error itself or none at all, so there is never a value to write back.
        let arguments = if parameters == 0 { Vec::new() } else { vec![self.last_error.clone().value()] };
        self.prepare_call_with_values(locals, parameters, first, arguments);

        let depth = self.return_addresses.len();
        self.in_handler = true;
        self.handler_depth = Some(depth);
        if self.push_return_address(ReturnAddress::func_call(self.cur_ptr, proc_id))? {
            self.goto(proc_offset)?;
        } else {
            self.in_handler = false;
            self.handler_depth = None;
        }
        Ok(())
    }
}
