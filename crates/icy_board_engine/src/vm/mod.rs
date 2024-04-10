use icy_ppe::ast::BinOp;
use icy_ppe::ast::Statement;
use icy_ppe::ast::UnaryOp;
use icy_ppe::datetime::IcbDate;
use icy_ppe::executable::Executable;
use icy_ppe::executable::PPECommand;
use icy_ppe::executable::PPEExpr;
use icy_ppe::executable::PPEScript;
use icy_ppe::executable::PPEStatement;
use icy_ppe::executable::VariableTable;
use icy_ppe::executable::VariableType;
use icy_ppe::executable::VariableValue;
use icy_ppe::Res;
use std::collections::HashMap;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;

pub mod expressions;

pub mod statements;
use crate::icy_board::pcboard_data::Node;
use crate::icy_board::state::IcyBoardState;
use crate::icy_board::user_base::User;

use self::expressions::FUNCTION_TABLE;
pub use self::statements::*;

pub mod io;
pub use self::io::*;

pub mod errors;
mod tests;

#[derive(Error, Debug, Clone)]
pub enum VMError {
    #[error("Internal VM error")]
    InternalVMError,

    #[error("Label not found ({0})")]
    LabelNotFound(usize),

    #[error("Tried to pop from empty value stack.")]
    PushPopStackEmpty,

    #[error("Can't fread variable ({0}) with size {1} requested size:{2}")]
    FReadError(VariableType, usize, usize),

    #[error("File not found ({0})")]
    FileNotFound(String),

    #[error("Error in function call ({0}): {1}")]
    ErrorInFunctionCall(String, String),

    #[error("Invalid seek position ({0})")]
    InvalidSeekPosition(i32),

    #[error("File channel not open ({0})")]
    FileChannelNotOpen(usize),

    #[error("Invalid user record out of bounds ({0})")]
    UserRecordOutOfBounds(usize),

    #[error("Pass value stack empty")]
    PassValueStackEmpty,

    #[error("Write back stack empty")]
    WriteBackStackEmpty,
}

#[derive(Clone, Copy)]
pub enum TerminalTarget {
    Both,
    User,
    Sysop,
}

#[derive(Clone, Copy, PartialEq)]
pub enum HangupType {
    Hangup,
    Bye,
    Goodbye,
}

pub trait BoardIO: Send {
    fn get_bps(&self) -> i32 {
        115_200
    }

    /// .
    ///
    /// # Errors
    /// Errors if the variable is not found.
    fn write_raw(&mut self, data: &[char]) -> Res<()>;

    /// .
    ///
    /// # Errors
    /// Errors if the variable is not found.
    fn read(&mut self) -> Res<String>;

    /// .
    ///
    /// # Errors
    /// Errors if the variable is not found.
    fn get_char(&mut self) -> Res<Option<(bool, char)>>;

    /// Number of chars in the keyboard buffer
    fn inbytes(&mut self) -> i32;

    /// Put a string into the keyboard buffer
    fn put_keyboard_buffer(&mut self, value: &[char]) -> Res<()>;

    /// .
    /// # Errors
    /// Errors if the variable is not found.
    fn hangup(&mut self) -> Res<()>;
}

pub struct StackFrame {
    pub values: HashMap<unicase::Ascii<String>, VariableValue>,
    pub cur_ptr: usize,
    pub label_table: HashMap<unicase::Ascii<String>, usize>,
}

pub fn calc_stmt_table(blk: &[Statement]) -> HashMap<unicase::Ascii<String>, usize> {
    let mut res = HashMap::new();
    for (i, stmt) in blk.iter().enumerate() {
        if let Statement::Label(label) = stmt {
            res.insert(label.get_label().clone(), i);
        }
    }
    res
}

pub struct ReturnAddress {
    ptr: usize,
    id: usize,
}

impl ReturnAddress {
    pub fn gosub(cur_ptr: usize) -> ReturnAddress {
        ReturnAddress {
            ptr: cur_ptr,
            id: 0,
        }
    }
    fn func_call(cur_ptr: usize, proc_id: usize) -> ReturnAddress {
        ReturnAddress {
            ptr: cur_ptr,
            id: proc_id,
        }
    }

    pub fn get_ptr(&self) -> usize {
        self.ptr
    }

    pub fn get_id(&self) -> usize {
        self.id
    }

    pub fn is_gosub(&self) -> bool {
        self.id == 0
    }
}

pub struct VirtualMachine<'a> {
    io: &'a mut dyn PCBoardIO,
    pub file_name: PathBuf,
    pub variable_table: VariableTable,

    pub script: PPEScript,
    pub cur_ptr: usize,
    pub is_running: bool,
    pub fpclear: bool,

    pub icy_board_state: &'a mut IcyBoardState,

    pub pcb_node: Option<Node>,

    pub return_addresses: Vec<ReturnAddress>,
    call_local_value_stack: Vec<VariableValue>,
    write_back_stack: Vec<PPEExpr>,

    pub label_table: HashMap<usize, usize>,
    pub push_pop_stack: Vec<VariableValue>,
}

impl<'a> VirtualMachine<'a> {
    fn set_user_variables(&mut self, cur_user: &User) {
        self.variable_table.set_value(
            U_EXPERT,
            VariableValue::new_bool(cur_user.flags.expert_mode),
        );
        self.variable_table.set_value(
            U_FSE,
            VariableValue::new_bool(cur_user.flags.use_fsedefault),
        );
        self.variable_table.set_value(
            U_FSEP,
            VariableValue::new_bool(!cur_user.flags.dont_ask_fse),
        );
        self.variable_table
            .set_value(U_CLS, VariableValue::new_bool(cur_user.flags.msg_clear));

        self.variable_table.set_value(
            U_EXPDATE,
            VariableValue::new_date(cur_user.exp_date.to_pcboard_date()),
        );

        self.variable_table.set_value(
            U_SEC,
            VariableValue::new_int(cur_user.security_level as i32),
        );
        self.variable_table
            .set_value(U_PAGELEN, VariableValue::new_int(cur_user.page_len as i32));
        self.variable_table.set_value(
            U_EXPSEC,
            VariableValue::new_int(cur_user.exp_security_level as i32),
        );
        self.variable_table
            .set_value(U_CITY, VariableValue::new_string(cur_user.city.clone()));
        self.variable_table.set_value(
            U_BDPHONE,
            VariableValue::new_string(cur_user.bus_data_phone.clone()),
        );
        self.variable_table.set_value(
            U_HVPHONE,
            VariableValue::new_string(cur_user.home_voice_phone.clone()),
        );

        self.variable_table.set_value(
            U_TRANS,
            VariableValue::new_string(cur_user.protocol.to_string()),
        );
        self.variable_table.set_value(
            U_CMNT1,
            VariableValue::new_string(cur_user.user_comment.clone()),
        );
        self.variable_table.set_value(
            U_CMNT2,
            VariableValue::new_string(cur_user.sysop_comment.clone()),
        );
        self.variable_table.set_value(
            U_PWD,
            VariableValue::new_string(/*cur_user.password.clone()*/ "".to_string()),
        );

        self.variable_table.set_value(
            U_SCROLL,
            VariableValue::new_bool(cur_user.flags.scroll_msg_body),
        );
        self.variable_table.set_value(
            U_LONGHDR,
            VariableValue::new_bool(!cur_user.flags.short_header),
        );

        self.variable_table
            .set_value(U_DEF79, VariableValue::new_bool(cur_user.flags.wide_editor));
        self.variable_table.set_value(
            U_ALIAS,
            VariableValue::new_string(cur_user.alias.to_string()),
        );

        self.variable_table.set_value(
            U_VER,
            VariableValue::new_string(cur_user.verify.to_string()),
        );

        self.variable_table
            .get_var_entry_mut(U_ADDR)
            .value
            .set_array_value(2, 0, 0, VariableValue::new_string(cur_user.city.clone()));

        self.variable_table
            .get_var_entry_mut(U_ADDR)
            .value
            .set_array_value(0, 0, 0, VariableValue::new_string(cur_user.street1.clone()));
        self.variable_table
            .get_var_entry_mut(U_ADDR)
            .value
            .set_array_value(1, 0, 0, VariableValue::new_string(cur_user.street2.clone()));

        self.variable_table
            .get_var_entry_mut(U_ADDR)
            .value
            .set_array_value(3, 0, 0, VariableValue::new_string(cur_user.state.clone()));
        self.variable_table
            .get_var_entry_mut(U_ADDR)
            .value
            .set_array_value(4, 0, 0, VariableValue::new_string(cur_user.zip.clone()));
        self.variable_table
            .get_var_entry_mut(U_ADDR)
            .value
            .set_array_value(5, 0, 0, VariableValue::new_string(cur_user.country.clone()));
        let mut i = 0;
        for l in cur_user.notes.lines() {
            self.variable_table
                .get_var_entry_mut(U_NOTES)
                .value
                .set_array_value(i, 0, 0, VariableValue::new_string(l.to_string()));
            i += 1;
            if i > 4 {
                break;
            }
        }
        while i < 5 {
            self.variable_table
                .get_var_entry_mut(U_NOTES)
                .value
                .set_array_value(i, 0, 0, VariableValue::new_string(String::new()));
            i += 1;
        }

        self.variable_table.set_value(
            U_PWDEXP,
            VariableValue::new_date(cur_user.password.expire_date.to_pcboard_date()),
        );
        if self.variable_table.get_version() >= 300 {
            // PCBoard seems not to set this variable ever.
            // U_ACCOUNT
        }

        if self.variable_table.get_version() >= 340 {
            self.variable_table.set_value(
                U_SHORTDESC,
                VariableValue::new_bool(cur_user.flags.short_header),
            );
            self.variable_table
                .set_value(U_GENDER, VariableValue::new_string(cur_user.gender.clone()));
            self.variable_table.set_value(
                U_BIRTHDATE,
                VariableValue::new_string(cur_user.birth_date.to_string()),
            );
            self.variable_table
                .set_value(U_EMAIL, VariableValue::new_string(cur_user.email.clone()));
            self.variable_table
                .set_value(U_WEB, VariableValue::new_string(cur_user.web.clone()));
        }
    }

    pub fn put_user_variables(&self, cur_user: &mut User) {
        cur_user.flags.expert_mode = self.variable_table.get_value(U_EXPERT).as_bool();
        cur_user.flags.use_fsedefault = self.variable_table.get_value(U_FSE).as_bool();
        cur_user.flags.dont_ask_fse = self.variable_table.get_value(U_FSEP).as_bool();
        cur_user.flags.msg_clear = self.variable_table.get_value(U_CLS).as_bool();

        cur_user.exp_date =
            IcbDate::from_pcboard(self.variable_table.get_value(U_EXPDATE).as_int() as u32);
        cur_user.security_level = self.variable_table.get_value(U_SEC).as_int() as u8;
        cur_user.page_len = self.variable_table.get_value(U_PAGELEN).as_int() as u16;
        cur_user.exp_security_level = self.variable_table.get_value(U_EXPSEC).as_int() as u8;

        cur_user.city = self.variable_table.get_value(U_CITY).as_string();
        cur_user.bus_data_phone = self.variable_table.get_value(U_BDPHONE).as_string();
        cur_user.home_voice_phone = self.variable_table.get_value(U_HVPHONE).as_string();
        cur_user.protocol = self
            .variable_table
            .get_value(U_TRANS)
            .as_string()
            .chars()
            .next()
            .unwrap_or('Z');
        cur_user.user_comment = self.variable_table.get_value(U_CMNT1).as_string();
        cur_user.sysop_comment = self.variable_table.get_value(U_CMNT2).as_string();
        //cur_user.user.password = self.variable_table.get_value(U_PWD).as_string();

        cur_user.flags.scroll_msg_body = self.variable_table.get_value(U_SCROLL).as_bool();
        cur_user.flags.short_header = self.variable_table.get_value(U_LONGHDR).as_bool();
        cur_user.flags.wide_editor = self.variable_table.get_value(U_DEF79).as_bool();
        cur_user.alias = self.variable_table.get_value(U_ALIAS).as_string();
        cur_user.verify = self.variable_table.get_value(U_VER).as_string();
        cur_user.street1 = self
            .variable_table
            .get_value(U_ADDR)
            .get_array_value(0, 0, 0)
            .as_string();
        cur_user.street2 = self
            .variable_table
            .get_value(U_ADDR)
            .get_array_value(1, 0, 0)
            .as_string();
        /* TODO?
        cur_user.city = self
            .variable_table
            .get_value(U_ADDR)
            .get_array_value(2, 0, 0)
            .as_string();
        */
        cur_user.state = self
            .variable_table
            .get_value(U_ADDR)
            .get_array_value(3, 0, 0)
            .as_string();
        cur_user.zip = self
            .variable_table
            .get_value(U_ADDR)
            .get_array_value(4, 0, 0)
            .as_string();
        cur_user.country = self
            .variable_table
            .get_value(U_ADDR)
            .get_array_value(6, 0, 0)
            .as_string();
        cur_user.notes.clear();
        for i in 0..5 {
            let v = self
                .variable_table
                .get_value(U_NOTES)
                .get_array_value(i, 0, 0)
                .as_string();
            cur_user.notes.push_str(&v);
            cur_user.notes.push('\n');
        }
        cur_user.password.expire_date =
            IcbDate::from_pcboard(self.variable_table.get_value(U_PWDEXP).as_int() as u32);

        if self.variable_table.get_version() >= 300 {
            // PCBoard seems not to set this variable ever.
            // U_ACCOUNT
        }

        if self.variable_table.get_version() >= 340 {
            cur_user.flags.short_header = self.variable_table.get_value(U_SHORTDESC).as_bool();

            cur_user.gender = self.variable_table.get_value(U_GENDER).as_string();
            cur_user.birth_date =
                IcbDate::parse(&self.variable_table.get_value(U_BIRTHDATE).as_string());
            cur_user.email = self.variable_table.get_value(U_EMAIL).as_string();
            cur_user.web = self.variable_table.get_value(U_WEB).as_string();
        }
    }

    fn eval_expr(&mut self, expr: &PPEExpr) -> Res<VariableValue> {
        match expr {
            PPEExpr::Invalid => Err(VMError::InternalVMError.into()),
            PPEExpr::Value(id) => Ok(self.variable_table.get_value(*id).clone()),
            PPEExpr::UnaryExpression(op, expr) => {
                let val = self.eval_expr(expr)?;
                match op {
                    UnaryOp::Not => Ok(val.not()),
                    UnaryOp::Minus => Ok(-val),
                    UnaryOp::Plus => Ok(val),
                }
            }
            PPEExpr::BinaryExpression(op, left, right) => {
                let left_value = self.eval_expr(left)?;
                let right_value = self.eval_expr(right)?;

                match op {
                    BinOp::Add => Ok(left_value + right_value),
                    BinOp::Sub => Ok(left_value - right_value),
                    BinOp::Mul => Ok(left_value * right_value),
                    BinOp::Div => Ok(left_value / right_value),
                    BinOp::Mod => Ok(left_value % right_value),
                    BinOp::PoW => Ok(left_value.pow(right_value)),
                    BinOp::Eq => Ok(VariableValue::new_bool(left_value == right_value)),
                    BinOp::NotEq => Ok(VariableValue::new_bool(left_value != right_value)),
                    BinOp::Or => Ok(VariableValue::new_bool(
                        left_value.as_bool() || right_value.as_bool(),
                    )),
                    BinOp::And => Ok(VariableValue::new_bool(
                        left_value.as_bool() && right_value.as_bool(),
                    )),
                    BinOp::Lower => Ok(VariableValue::new_bool(left_value < right_value)),
                    BinOp::LowerEq => Ok(VariableValue::new_bool(left_value <= right_value)),
                    BinOp::Greater => Ok(VariableValue::new_bool(left_value > right_value)),
                    BinOp::GreaterEq => Ok(VariableValue::new_bool(left_value >= right_value)),
                }
            }
            PPEExpr::Dim(id, dims) => {
                let dim_1 = self.eval_expr(&dims[0])?.as_int() as usize;
                let dim_2 = if dims.len() >= 2 {
                    self.eval_expr(&dims[1])?.as_int() as usize
                } else {
                    0
                };
                let dim_3 = if dims.len() >= 3 {
                    self.eval_expr(&dims[2])?.as_int() as usize
                } else {
                    0
                };

                Ok(self
                    .variable_table
                    .get_value(*id)
                    .get_array_value(dim_1, dim_2, dim_3))
            }

            PPEExpr::PredefinedFunctionCall(func, arguments) => {
                match (FUNCTION_TABLE[(func.opcode as i16).unsigned_abs() as usize])(
                    self, arguments,
                ) {
                    Ok(val) => Ok(val),
                    Err(e) => Err(VMError::ErrorInFunctionCall(
                        func.name.to_string(),
                        e.to_string(),
                    )
                    .into()),
                }
            }

            PPEExpr::FunctionCall(func_id, arguments) => {
                let proc_offset;
                let locals;
                let parameters;
                let first;
                let return_var_id;
                unsafe {
                    let proc = &self.variable_table.get_var_entry(*func_id);
                    proc_offset = proc.value.data.function_value.start_offset as usize;
                    first = (proc.value.data.function_value.first_var_id + 1) as usize;
                    locals = proc.value.data.function_value.local_variables as usize;
                    parameters = proc.value.data.function_value.parameters as usize;
                    return_var_id = proc.value.data.function_value.return_var as usize;
                }

                self.prepare_call(locals, parameters, first, arguments, 0)?;

                self.return_addresses
                    .push(ReturnAddress::func_call(self.cur_ptr, *func_id));
                self.goto(proc_offset)?;
                self.run()?;
                self.fpclear = false;
                Ok(self.variable_table.get_value(return_var_id).clone())
            }
        }
    }

    fn run(&mut self) -> Res<()> {
        let max_ptr = self.script.statements.len();
        while !self.fpclear && self.is_running && self.cur_ptr < max_ptr {
            let p = self.cur_ptr;
            self.cur_ptr += 1;
            let c = self.script.statements[p].command.clone();
            self.execute_statement(&c)?;
        }
        Ok(())
    }

    fn set_variable(&mut self, variable: &PPEExpr, value: VariableValue) -> Res<()> {
        match variable {
            PPEExpr::Value(id) => {
                self.variable_table.set_value(*id, value);
            }
            PPEExpr::Dim(id, dims) => {
                let dim_1 = self.eval_expr(&dims[0])?.as_int() as usize;
                let dim_2 = if dims.len() >= 2 {
                    self.eval_expr(&dims[1])?.as_int() as usize
                } else {
                    0
                };
                let dim_3 = if dims.len() >= 3 {
                    self.eval_expr(&dims[2])?.as_int() as usize
                } else {
                    0
                };

                self.variable_table
                    .get_var_entry_mut(*id)
                    .value
                    .set_array_value(dim_1, dim_2, dim_3, value);
            }
            _ => {
                return Err(VMError::InternalVMError.into());
            }
        }
        Ok(())
    }

    fn execute_statement(&mut self, stmt: &PPECommand) -> Res<()> {
        match stmt {
            PPECommand::End | PPECommand::Stop => {
                self.is_running = false;
            }

            PPECommand::EndFunc | PPECommand::EndProc | PPECommand::Return => {
                if let Some(addr) = self.return_addresses.pop() {
                    self.cur_ptr = addr.get_ptr();
                    let proc_id = addr.get_id();
                    if proc_id > 0 {
                        let locals;
                        let first;
                        let parameters;
                        let return_var_id;
                        let pass_flags;
                        let is_func;
                        unsafe {
                            let proc = &self.variable_table.get_var_entry(proc_id);
                            first = (proc.value.data.procedure_value.first_var_id + 1) as usize;
                            locals = proc.value.data.procedure_value.local_variables as usize;
                            parameters = proc.value.data.procedure_value.parameters as usize;
                            if proc.header.variable_type == VariableType::Function {
                                is_func = true;
                                return_var_id = proc.value.data.function_value.return_var as usize;
                                pass_flags = 0;
                            } else {
                                is_func = false;
                                return_var_id = 0;
                                pass_flags = proc.value.data.procedure_value.pass_flags;
                            }
                        }

                        // get write back values
                        let mut pass_values = Vec::new();
                        if pass_flags > 0 {
                            for i in 0..parameters {
                                if (1 << i) & pass_flags != 0 {
                                    let id = first + i;
                                    let val = self.variable_table.get_value(id).clone();
                                    // println!("push pass value {}", val);
                                    pass_values.push(val);
                                }
                            }
                        }

                        // write back locals + parameters
                        for i in (0..(locals + parameters)).rev() {
                            let id = first + i;
                            if self.variable_table.get_var_entry(id).header.flags & 0x1 == 0x0 {
                                let Some(value) = self.call_local_value_stack.pop() else {
                                    return Err(VMError::PushPopStackEmpty.into());
                                };
                                if id != return_var_id {
                                    self.variable_table.set_value(id, value);
                                }
                            }
                        }

                        if pass_flags > 0 {
                            for i in (0..parameters).rev() {
                                if (1 << i) & pass_flags != 0 {
                                    let Some(val) = pass_values.pop() else {
                                        return Err(VMError::PassValueStackEmpty.into());
                                    };
                                    if let Some(argument_expr) = self.write_back_stack.pop() {
                                        //println!("pop pass value {}", val);
                                        self.set_variable(&argument_expr, val)?;
                                    } else {
                                        return Err(VMError::WriteBackStackEmpty.into());
                                    }
                                }
                            }
                        }

                        if is_func {
                            self.fpclear = true;
                        }
                    }
                } else {
                    self.is_running = false;
                }
            }

            PPECommand::IfNot(expr, label) => {
                let value = self.eval_expr(expr)?.as_bool();
                if !value {
                    self.goto(*label)?;
                }
            }

            PPECommand::ProcedureCall(proc_id, arguments) => {
                let proc_offset;
                let locals;
                let parameters;
                let first;
                let pass_flags;

                unsafe {
                    let proc = &self.variable_table.get_var_entry(*proc_id);
                    proc_offset = proc.value.data.procedure_value.start_offset as usize;
                    first = (proc.value.data.procedure_value.first_var_id + 1) as usize;
                    locals = proc.value.data.procedure_value.local_variables as usize;
                    parameters = proc.value.data.procedure_value.parameters as usize;
                    pass_flags = proc.value.data.procedure_value.pass_flags;
                }
                self.prepare_call(locals, parameters, first, arguments, pass_flags)?;

                self.return_addresses
                    .push(ReturnAddress::func_call(self.cur_ptr, *proc_id));
                self.goto(proc_offset)?;
            }

            PPECommand::PredefinedCall(proc, arguments) => {
                (STATEMENT_TABLE[proc.opcode as usize])(self, arguments)?;
            }

            PPECommand::Goto(label) => {
                self.goto(*label)?;
            }
            PPECommand::Gosub(label) => {
                self.return_addresses
                    .push(ReturnAddress::gosub(self.cur_ptr));
                self.goto(*label)?;
            }
            PPECommand::Let(variable, expr) => {
                let val = self.eval_expr(expr)?;
                self.set_variable(variable, val)?;
            }
        }

        Ok(())
    }

    #[allow(clippy::needless_range_loop)]
    fn prepare_call(
        &mut self,
        locals: usize,
        parameters: usize,
        first: usize,
        arguments: &[PPEExpr],
        pass_flags: u16,
    ) -> Res<()> {
        // store locals + parameters
        for i in 0..(locals + parameters) {
            let id = first + i;
            if self.variable_table.get_var_entry(id).header.flags & 0x1 == 0x0 {
                let val = self.variable_table.get_value(id).clone();
                //println!("store {:02X} to {}", id, val);
                self.call_local_value_stack.push(val);
            }
        }
        for i in 0..parameters {
            let id = first + i;
            let value = self.eval_expr(&arguments[i])?;
            //  println!("set parameter {:02X} to {}", id, value);
            self.variable_table.set_value(id, value);

            if (1 << i) & pass_flags != 0 {
                self.write_back_stack.push(arguments[i].clone());
            }
        }
        for i in 0..locals {
            let id = first + parameters + i;
            let (flags, vtype) = {
                let header = &self.variable_table.get_var_entry(id).header;
                (header.flags, header.variable_type)
            };
            if (flags & 0x1) == 0x0 {
                // println!("reset local {:02X}", id);
                self.variable_table
                    .set_value(id, vtype.create_empty_value());
            }
        }

        Ok(())
    }

    fn goto(&mut self, label: usize) -> Result<(), VMError> {
        if let Some(label) = self.label_table.get(&label) {
            self.cur_ptr = *label;
            Ok(())
        } else {
            Err(VMError::LabelNotFound(label))
        }
    }

    pub fn resolve_file<P: AsRef<Path>>(&self, file: &P) -> String {
        self.icy_board_state
            .board
            .lock()
            .unwrap()
            .resolve_file(file)
    }
}

/// .
/// # Errors
pub fn run<P: AsRef<Path>>(
    file_name: &P,
    prg: &Executable,
    io: &mut dyn PCBoardIO,
    icy_board_state: &mut IcyBoardState,
) -> Res<bool> {
    let Ok(script) = PPEScript::from_ppe_file(prg) else {
        return Ok(false);
    };
    let mut label_table = calc_labe_table(&script.statements);
    for (i, stmt) in script.statements.iter().enumerate() {
        label_table.insert(stmt.span.start * 2, i);
    }
    let file_name = file_name.as_ref().to_path_buf();
    let mut vm = VirtualMachine {
        file_name,
        return_addresses: Vec::new(),
        script,
        io,
        is_running: true,
        fpclear: false,
        icy_board_state,
        pcb_node: None,
        variable_table: prg.variable_table.clone(),
        cur_ptr: 0,
        label_table,
        call_local_value_stack: Vec::new(),
        write_back_stack: Vec::new(),
        push_pop_stack: Vec::new(),
    };
    vm.run()?;
    Ok(true)
}

fn calc_labe_table(statements: &[PPEStatement]) -> HashMap<usize, usize> {
    let mut res = HashMap::new();
    let mut offset = 0;
    for (i, stmt) in statements.iter().enumerate() {
        res.insert(offset, i);
        offset += stmt.command.get_size();
    }
    res
}

pub const U_EXPERT: usize = 1;
pub const U_FSE: usize = 2;
pub const U_FSEP: usize = 3;
pub const U_CLS: usize = 4;
pub const U_EXPDATE: usize = 5;
pub const U_SEC: usize = 6;
pub const U_PAGELEN: usize = 7;
pub const U_EXPSEC: usize = 8;
pub const U_CITY: usize = 9;
pub const U_BDPHONE: usize = 10;
pub const U_HVPHONE: usize = 11;
pub const U_TRANS: usize = 12;
pub const U_CMNT1: usize = 13;
pub const U_CMNT2: usize = 14;
pub const U_PWD: usize = 15;
pub const U_SCROLL: usize = 16;
pub const U_LONGHDR: usize = 17;
pub const U_DEF79: usize = 18;
pub const U_ALIAS: usize = 19;
pub const U_VER: usize = 20;
pub const U_ADDR: usize = 21;
pub const U_NOTES: usize = 22;
pub const U_PWDEXP: usize = 23;
// 3.00
pub const U_ACCOUNT: usize = 24;

// 3.40
pub const U_SHORTDESC: usize = 25;
pub const U_GENDER: usize = 26;
pub const U_BIRTHDATE: usize = 27;
pub const U_EMAIL: usize = 28;
pub const U_WEB: usize = 29;
