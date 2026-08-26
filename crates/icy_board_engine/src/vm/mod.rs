use crate::Res;
use crate::ast::BinOp;
use crate::ast::Statement;
use crate::ast::UnaryOp;
use crate::ast::constant::STACK_LIMIT;
use crate::datetime::IcbDate;
use crate::executable::Executable;
use crate::executable::GenericVariableData;
use crate::executable::OnErrorTarget;
use crate::executable::PPECommand;
use crate::executable::PPEExpr;
use crate::executable::PPEScript;
use crate::executable::VariableTable;
use crate::executable::VariableType;
use crate::executable::VariableValue;
use crate::icy_board::lookup_case_insensitive;
use crate::icy_board::state::NodeState;
use crate::icy_board::state::ppl_error::{
    ERR_FORMAT, ERR_INVALID, ERR_IO, ERR_KIND_DBASE, ERR_KIND_FILE, ERR_KIND_GFX, ERR_KIND_STACK, ERR_LIMIT, ERR_STACK, ERR_UNAVAILABLE, ERR_UNSUPPORTED,
    PplError,
};
use crate::icy_board::user_base::FSEMode;
use crate::parser::UserTypeRegistry;
use crate::vm::expressions::to_base_36;
use async_recursion::async_recursion;
use icy_engine::TextBuffer;
use jamjam::jam::JamMessageBase;
use jamjam::jam::msg_header::JamMessageHeader;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;
use thiserror::Error;

pub mod expressions;

pub mod statements;
use crate::icy_board::state::IcyBoardState;
use crate::icy_board::user_base::Password;
use crate::icy_board::user_base::User;

use self::expressions::run_function;
pub use self::statements::*;

pub mod io;

pub mod dbase;
pub use self::io::*;

pub mod errors;
mod tests;

#[derive(Error, Debug, Clone)]
pub enum VMError {
    #[error("Internal VM error")]
    InternalVMError,

    #[error("Label not found (0x{0:X})")]
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
    FileChannelNotOpen(i32),

    #[error("Pass value stack empty")]
    PassValueStackEmpty,

    #[error("Write back stack empty")]
    WriteBackStackEmpty,

    #[error("No user type base expression")]
    NoUserTypeBase,

    #[error("Type not found in registry")]
    TypeNotFoundInRegistry(u8),

    #[error("Object not found (internal VM error) ({0})")]
    NoObjectFound(u8),

    #[error("Member {1} not found for user type {0}")]
    InvalidMemberId(u8, usize),

    #[error("Member {1} of user type {0} is not a function")]
    InvalidMemberFunction(u8, usize),

    #[error("Member {1} of user type {0} expected {2} arguments, got {3}")]
    InvalidMemberArgumentCount(u8, usize, usize, usize),

    #[error("PPE call stack exhausted")]
    StackOverflow,
}

#[derive(Clone, Copy, PartialEq, Debug)]
pub enum TerminalTarget {
    Both,
    User,
    Sysop,
}

pub struct StackFrame {
    pub values: HashMap<unicase::Ascii<String>, VariableValue>,
    pub cur_ptr: usize,
    pub label_table: HashMap<unicase::Ascii<String>, usize>,
}

/// Where `ON ERROR` sends a program when an operation fails.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum ErrorHandler {
    #[default]
    Off,
    /// Jumps for good; the handler ends the program rather than coming back.
    Goto(usize),
    Gosub(usize),
    /// Calls the procedure with the id of its variable table entry.
    Procedure(usize),
}

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
        ReturnAddress { ptr: cur_ptr, id: 0 }
    }
    fn func_call(cur_ptr: usize, proc_id: usize) -> ReturnAddress {
        ReturnAddress { ptr: cur_ptr, id: proc_id }
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
    pub io: &'a mut dyn PCBoardIO,
    pub type_registry: &'a UserTypeRegistry,
    pub file_name: PathBuf,
    pub variable_table: VariableTable,

    pub script: PPEScript,
    pub cur_ptr: usize,
    pub is_running: bool,
    /// Set by STOP, which gives up on a program rather than ending it.
    pub aborted: bool,
    pub fpclear: bool,

    /// currently unused.
    pub use_lmrs: bool,

    pub icy_board_state: &'a mut IcyBoardState,

    pub pcb_node: Option<NodeState>,

    pub return_addresses: Vec<ReturnAddress>,
    pub call_local_value_stack: Vec<VariableValue>,
    pub write_back_stack: Vec<PPEExpr>,
    pub user_types: Vec<Vec<crate::executable::RecordField>>,

    pub label_table: HashMap<usize, usize>,
    pub push_pop_stack: Vec<VariableValue>,

    pub stored_screen: Option<TextBuffer>,

    pub fd_default_in: i32,
    pub fd_default_out: i32,

    pub file_list: VecDeque<String>,
    pub user: User,
    pub cached_msg_header: Option<(i32, i32, u32, JamMessageHeader)>,

    /// What `STACKABORT` last asked for. Aborting is the default; a PPE has to
    /// opt into limping on after it has blown the stack.
    pub abort_on_stack_error: bool,

    /// What the last operation that can fail did, which is what `Error.Last()` hands out.
    pub last_error: PplError,

    /// Set when an operation failed, and cleared once the statement it failed in is over.
    pub error_pending: bool,

    pub error_handler: ErrorHandler,

    /// Whether the handler is running, so that a failure inside it is recorded
    /// rather than sending the program back into the handler again.
    pub in_handler: bool,

    /// The call depth the handler returns to, or `None` for a `GOTO` handler that never does.
    pub handler_depth: Option<usize>,

    /// `Board` is a snapshot of what the board is configured to be, so it is taken once
    /// rather than on every access - building it copies every conference.
    pub board_value: Option<VariableValue>,

    /// The message base the `AREA`/`MSG` calls read through. Opening one is what such a
    /// call costs, so a walk keeps it rather than paying for it once per message.
    message_base: Option<(PathBuf, JamMessageBase)>,

    pub dbase: dbase::DbaseState,
}

impl<'a> VirtualMachine<'a> {
    /// A machine with no program in it, which `run` fills in from an executable and a
    /// caller that only wants one function can use as it is.
    pub fn new(file_name: PathBuf, type_registry: &'a UserTypeRegistry, io: &'a mut dyn PCBoardIO, icy_board_state: &'a mut IcyBoardState) -> Self {
        Self {
            file_name,
            type_registry,
            return_addresses: Vec::new(),
            script: PPEScript::default(),
            io,
            is_running: true,
            aborted: false,
            fpclear: false,
            icy_board_state,
            pcb_node: None,
            variable_table: VariableTable::default(),
            cur_ptr: 0,
            label_table: HashMap::new(),
            call_local_value_stack: Vec::new(),
            write_back_stack: Vec::new(),
            user_types: Vec::new(),
            push_pop_stack: Vec::new(),
            stored_screen: None,
            fd_default_in: 0,
            fd_default_out: 0,
            file_list: VecDeque::new(),
            user: User::default(),
            use_lmrs: true,
            cached_msg_header: None,
            abort_on_stack_error: true,
            board_value: None,
            message_base: None,
            last_error: PplError::default(),
            error_pending: false,
            error_handler: ErrorHandler::Off,
            in_handler: false,
            handler_depth: None,
            dbase: dbase::DbaseState::default(),
        }
    }
}

impl VirtualMachine<'_> {
    fn set_user_variables(&mut self) -> Res<()> {
        if !self.variable_table.has_user_vars() {
            log::warn!("Tried to set user variables, but no user variables defined.");
            return Ok(());
        }
        let cur_user = &self.user;
        self.variable_table.set_value(U_EXPERT, VariableValue::new_bool(cur_user.flags.expert_mode));
        match cur_user.flags.fse_mode {
            FSEMode::Yes => {
                // U_FSE = FSEDefault, U_FSEP = !DontAskFSE. "Yes" means always use the
                // full screen editor without asking, so U_FSEP (ask) must be false.
                self.variable_table.set_value(U_FSE, VariableValue::new_bool(true));
                self.variable_table.set_value(U_FSEP, VariableValue::new_bool(false));
            }
            FSEMode::Ask => {
                self.variable_table.set_value(U_FSE, VariableValue::new_bool(false));
                self.variable_table.set_value(U_FSEP, VariableValue::new_bool(true));
            }
            FSEMode::No => {
                self.variable_table.set_value(U_FSE, VariableValue::new_bool(false));
                self.variable_table.set_value(U_FSEP, VariableValue::new_bool(false));
            }
        }
        self.variable_table.set_value(U_CLS, VariableValue::new_bool(cur_user.flags.msg_clear));

        self.variable_table.set_value(
            U_EXPDATE,
            VariableValue::new_date(IcbDate::from_utc(&cur_user.expiration_date).to_pcboard_date()),
        );

        self.variable_table.set_value(U_SEC, VariableValue::new_int(cur_user.security_level as i32));
        self.variable_table.set_value(U_PAGELEN, VariableValue::new_int(cur_user.page_len as i32));
        self.variable_table
            .set_value(U_EXPSEC, VariableValue::new_int(cur_user.exp_security_level as i32));
        self.variable_table.set_value(U_CITY, VariableValue::new_string(cur_user.city_or_state.clone()));
        self.variable_table
            .set_value(U_BDPHONE, VariableValue::new_string(cur_user.bus_data_phone.clone()));
        self.variable_table
            .set_value(U_HVPHONE, VariableValue::new_string(cur_user.home_voice_phone.clone()));

        self.variable_table.set_value(U_TRANS, VariableValue::new_string(cur_user.protocol.clone()));
        self.variable_table.set_value(U_CMNT1, VariableValue::new_string(cur_user.user_comment.clone()));
        self.variable_table
            .set_value(U_CMNT2, VariableValue::new_string(cur_user.sysop_comment.clone()));
        self.variable_table.get_value_mut(U_PWD).vtype = VariableType::Password;
        self.variable_table
            .set_value(U_PWD, VariableValue::new_password(cur_user.password.password.clone()));

        self.variable_table.set_value(U_SCROLL, VariableValue::new_bool(cur_user.flags.scroll_msg_body));
        self.variable_table
            .set_value(U_LONGHDR, VariableValue::new_bool(!cur_user.flags.use_short_filedescr));

        self.variable_table.set_value(U_DEF79, VariableValue::new_bool(cur_user.flags.wide_editor));
        self.variable_table.set_value(U_ALIAS, VariableValue::new_string(cur_user.alias.clone()));

        self.variable_table.set_value(U_VER, VariableValue::new_string(cur_user.verify_answer.clone()));

        self.variable_table
            .get_var_entry_mut(U_ADDR)
            .value
            .set_array_value(2, 0, 0, VariableValue::new_string(cur_user.city_or_state.clone()))?;

        self.variable_table
            .get_var_entry_mut(U_ADDR)
            .value
            .set_array_value(0, 0, 0, VariableValue::new_string(cur_user.street1.clone()))?;
        self.variable_table
            .get_var_entry_mut(U_ADDR)
            .value
            .set_array_value(1, 0, 0, VariableValue::new_string(cur_user.street2.clone()))?;

        self.variable_table
            .get_var_entry_mut(U_ADDR)
            .value
            .set_array_value(3, 0, 0, VariableValue::new_string(cur_user.state.clone()))?;
        self.variable_table
            .get_var_entry_mut(U_ADDR)
            .value
            .set_array_value(4, 0, 0, VariableValue::new_string(cur_user.zip.clone()))?;
        self.variable_table
            .get_var_entry_mut(U_ADDR)
            .value
            .set_array_value(5, 0, 0, VariableValue::new_string(cur_user.country.clone()))?;

        self.variable_table
            .get_var_entry_mut(U_NOTES)
            .value
            .set_array_value(0, 0, 0, VariableValue::new_string(cur_user.custom_comment1.clone()))?;

        self.variable_table
            .get_var_entry_mut(U_NOTES)
            .value
            .set_array_value(1, 0, 0, VariableValue::new_string(cur_user.custom_comment2.clone()))?;

        self.variable_table
            .get_var_entry_mut(U_NOTES)
            .value
            .set_array_value(2, 0, 0, VariableValue::new_string(cur_user.custom_comment3.clone()))?;

        self.variable_table
            .get_var_entry_mut(U_NOTES)
            .value
            .set_array_value(3, 0, 0, VariableValue::new_string(cur_user.custom_comment4.clone()))?;

        self.variable_table
            .get_var_entry_mut(U_NOTES)
            .value
            .set_array_value(4, 0, 0, VariableValue::new_string(cur_user.custom_comment5.clone()))?;

        let mut i = 0;
        while i < 5 {
            self.variable_table
                .get_var_entry_mut(U_NOTES)
                .value
                .set_array_value(i, 0, 0, VariableValue::new_string(String::new()))?;
            i += 1;
        }

        self.variable_table.set_value(
            U_PWDEXP,
            VariableValue::new_date(IcbDate::from_utc(&cur_user.password.expire_date).to_pcboard_date()),
        );
        if self.variable_table.get_version() >= 300 {
            // PCBoard seems not to set this variable ever.
            // U_ACCOUNT
        }

        if self.variable_table.get_version() >= 340 {
            self.variable_table
                .set_value(U_SHORTDESC, VariableValue::new_bool(cur_user.flags.use_short_filedescr));
            self.variable_table.set_value(U_GENDER, VariableValue::new_string(cur_user.gender.clone()));
            let day = &cur_user.birth_date;
            self.variable_table.set_value(U_BIRTHDATE, VariableValue::new_string(day.to_string()));
            self.variable_table.set_value(U_EMAIL, VariableValue::new_string(cur_user.email.clone()));
            self.variable_table.set_value(U_WEB, VariableValue::new_string(cur_user.web.clone()));
        }
        Ok(())
    }

    pub async fn put_user_variables(&self, cur_user: &mut User) {
        cur_user.flags.expert_mode = self.variable_table.get_value(U_EXPERT).as_bool();
        if self.variable_table.get_value(U_FSE).as_bool() {
            cur_user.flags.fse_mode = FSEMode::Yes;
        } else if self.variable_table.get_value(U_FSEP).as_bool() {
            cur_user.flags.fse_mode = FSEMode::Ask;
        } else {
            cur_user.flags.fse_mode = FSEMode::No;
        }
        cur_user.flags.msg_clear = self.variable_table.get_value(U_CLS).as_bool();

        cur_user.expiration_date = IcbDate::from_pcboard(self.variable_table.get_value(U_EXPDATE).as_int() as u32).to_utc_date_time();
        cur_user.security_level = self.variable_table.get_value(U_SEC).as_int() as u8;
        cur_user.page_len = self.variable_table.get_value(U_PAGELEN).as_int() as u16;
        cur_user.exp_security_level = self.variable_table.get_value(U_EXPSEC).as_int() as u8;

        cur_user.city_or_state = self.variable_table.get_value(U_CITY).as_string();
        cur_user.bus_data_phone = self.variable_table.get_value(U_BDPHONE).as_string();
        cur_user.home_voice_phone = self.variable_table.get_value(U_HVPHONE).as_string();
        cur_user.protocol = self.variable_table.get_value(U_TRANS).as_string();
        cur_user.user_comment = self.variable_table.get_value(U_CMNT1).as_string();
        cur_user.sysop_comment = self.variable_table.get_value(U_CMNT2).as_string();

        let pwd_value = self.variable_table.get_value(U_PWD);
        cur_user.password.password = if let GenericVariableData::Password(ref pwd) = pwd_value.generic_data {
            match pwd {
                // A secret a PPE carried over from elsewhere is stored the way the board
                // stores any password it is told, rather than as it stands.
                Password::PlainText(s) | Password::Protected(s) => self.icy_board_state.create_password(s).await,
                pwd => pwd.clone(),
            }
        } else {
            // Fallback: create a password from the string representation
            Password::new_argon2(pwd_value.as_string())
        };

        cur_user.flags.scroll_msg_body = self.variable_table.get_value(U_SCROLL).as_bool();
        cur_user.flags.use_short_filedescr = self.variable_table.get_value(U_LONGHDR).as_bool();
        cur_user.flags.wide_editor = self.variable_table.get_value(U_DEF79).as_bool();
        cur_user.alias = self.variable_table.get_value(U_ALIAS).as_string();
        cur_user.verify_answer = self.variable_table.get_value(U_VER).as_string();
        cur_user.street1 = self.variable_table.get_value(U_ADDR).get_array_value(0, 0, 0).as_string();
        cur_user.street2 = self.variable_table.get_value(U_ADDR).get_array_value(1, 0, 0).as_string();
        /* TODO?
        cur_user.city = self
            .variable_table
            .get_value(U_ADDR)
            .get_array_value(2, 0, 0)
            .as_string();
        */
        cur_user.state = self.variable_table.get_value(U_ADDR).get_array_value(3, 0, 0).as_string();
        cur_user.zip = self.variable_table.get_value(U_ADDR).get_array_value(4, 0, 0).as_string();
        cur_user.country = self.variable_table.get_value(U_ADDR).get_array_value(6, 0, 0).as_string();
        cur_user.custom_comment1 = self.variable_table.get_value(U_NOTES).get_array_value(0, 0, 0).as_string();
        cur_user.custom_comment2 = self.variable_table.get_value(U_NOTES).get_array_value(1, 0, 0).as_string();
        cur_user.custom_comment3 = self.variable_table.get_value(U_NOTES).get_array_value(2, 0, 0).as_string();
        cur_user.custom_comment4 = self.variable_table.get_value(U_NOTES).get_array_value(3, 0, 0).as_string();
        cur_user.custom_comment5 = self.variable_table.get_value(U_NOTES).get_array_value(4, 0, 0).as_string();
        cur_user.password.expire_date = IcbDate::from_pcboard(self.variable_table.get_value(U_PWDEXP).as_int() as u32).to_utc_date_time();

        if self.variable_table.get_version() >= 300 {
            // PCBoard seems not to set this variable ever.
            // U_ACCOUNT
        }

        if self.variable_table.get_version() >= 340 {
            cur_user.flags.use_short_filedescr = self.variable_table.get_value(U_SHORTDESC).as_bool();

            cur_user.gender = self.variable_table.get_value(U_GENDER).as_string();
            cur_user.birth_date = IcbDate::parse(&self.variable_table.get_value(U_BIRTHDATE).as_string()).to_utc_date_time();
            cur_user.email = self.variable_table.get_value(U_EMAIL).as_string();
            cur_user.web = self.variable_table.get_value(U_WEB).as_string();
        }
    }

    /// The expression nodes that can never await, walked without the boxed future the
    /// async path needs for every single node.
    ///
    /// `None` means the tree holds a call, so the caller runs the whole expression again
    /// asynchronously. Repeating it is safe because every node handled here is a pure
    /// read or an arithmetic operation - the nodes that could have a side effect are
    /// exactly the ones this refuses.
    fn eval_expr_sync(&mut self, expr: &PPEExpr) -> Option<VariableValue> {
        match expr {
            PPEExpr::Value(id) | PPEExpr::RoutineReference(id) => Some(self.variable_table.get_value(*id).clone()),
            PPEExpr::UnaryExpression(op, expr) => {
                let value = self.eval_expr_sync(expr)?;
                Some(Self::apply_unary_op(*op, value))
            }
            PPEExpr::BinaryExpression(op, left, right) => {
                let left_value = self.eval_expr_sync(left)?;
                let right_value = self.eval_expr_sync(right)?;
                Some(Self::apply_bin_op(*op, left_value, right_value))
            }
            PPEExpr::Dim(id, dims) => {
                let dim_1 = self.eval_expr_sync(&dims[0])?.as_int() as usize;
                let dim_2 = if dims.len() >= 2 {
                    self.eval_expr_sync(&dims[1])?.as_int() as usize
                } else {
                    0
                };
                let dim_3 = if dims.len() >= 3 {
                    self.eval_expr_sync(&dims[2])?.as_int() as usize
                } else {
                    0
                };
                Some(self.variable_table.get_value(*id).get_array_value(dim_1, dim_2, dim_3))
            }
            _ => None,
        }
    }

    fn apply_unary_op(op: UnaryOp, value: VariableValue) -> VariableValue {
        match op {
            UnaryOp::Not => value.not(),
            UnaryOp::Minus => -value,
            UnaryOp::Plus => value,
        }
    }

    fn apply_bin_op(op: BinOp, left: VariableValue, right: VariableValue) -> VariableValue {
        match op {
            BinOp::Add => left + right,
            BinOp::Sub => left - right,
            BinOp::Mul => left * right,
            BinOp::Div => left / right,
            BinOp::Mod => left % right,
            BinOp::PoW => left.pow(right),
            BinOp::Eq => VariableValue::new_bool(left == right),
            BinOp::NotEq => VariableValue::new_bool(left != right),
            // Both sides are evaluated before this runs, so these do not short-circuit.
            BinOp::Or => VariableValue::new_bool(left.as_bool() || right.as_bool()),
            BinOp::And => VariableValue::new_bool(left.as_bool() && right.as_bool()),
            BinOp::Lower => VariableValue::new_bool(left < right),
            BinOp::LowerEq => VariableValue::new_bool(left <= right),
            BinOp::Greater => VariableValue::new_bool(left > right),
            BinOp::GreaterEq => VariableValue::new_bool(left >= right),
        }
    }

    #[async_recursion(?Send)]
    pub async fn eval_expr(&mut self, expr: &PPEExpr) -> Res<VariableValue> {
        if let Some(value) = self.eval_expr_sync(expr) {
            return Ok(value);
        }
        match expr {
            PPEExpr::Invalid => Err(VMError::InternalVMError.into()),
            PPEExpr::Value(id) | PPEExpr::RoutineReference(id) => Ok(self.variable_table.get_value(*id).clone()),
            PPEExpr::RecordLiteral(type_id, fields) => {
                let mut value = crate::executable::create_record_value(*type_id, &self.user_types).ok_or(VMError::InternalVMError)?;
                let GenericVariableData::Record(values) = &mut value.generic_data else {
                    return Err(VMError::InternalVMError.into());
                };
                for (field_id, expression) in fields {
                    let field_type = values.get(*field_id).ok_or(VMError::InvalidMemberId(*type_id, *field_id))?.vtype;
                    values[*field_id] = self.eval_expr(expression).await?.convert_to(field_type);
                }
                Ok(value)
            }

            PPEExpr::Member(base_expr, member_id) => {
                let val = self.eval_expr(base_expr).await?;
                let VariableType::UserData(type_id) = val.get_type() else {
                    log::error!("No user type base for value: {val:?} on expr {base_expr:?}");
                    return Err(VMError::NoUserTypeBase.into());
                };
                if let GenericVariableData::Record(fields) = &val.generic_data {
                    let Some(field) = fields.get(*member_id) else {
                        return Err(VMError::InvalidMemberId(type_id, *member_id).into());
                    };
                    return Ok(field.clone());
                }
                let Some(registry) = self.type_registry.get_type_from_id(type_id) else {
                    log::error!("No user data registry entry for value: {val:?} type :{type_id} on expr {base_expr:?}");
                    return Err(VMError::TypeNotFoundInRegistry(type_id).into());
                };
                let GenericVariableData::UserData(object) = val.generic_data else {
                    // should never happen.
                    return Err(VMError::NoObjectFound(type_id).into());
                };

                let Some(member) = registry.id_table.get(*member_id) else {
                    return Err(VMError::InvalidMemberId(type_id, *member_id).into());
                };

                match member {
                    crate::compiler::user_data::UserDataEntry::Field(name) | crate::compiler::user_data::UserDataEntry::Getter(name) => {
                        let val = object.get_property_value(self, name);
                        return val;
                    }
                    crate::compiler::user_data::UserDataEntry::Procedure(_) | crate::compiler::user_data::UserDataEntry::Function(_) => {
                        return Ok(VariableValue {
                            vtype: val.vtype,
                            data: val.data,
                            generic_data: GenericVariableData::UserData(object),
                        });
                    }
                }
            }

            PPEExpr::IndexedMember(base_expr, id, arguments) => {
                let val = self.eval_expr(base_expr).await?;
                let GenericVariableData::Record(fields) = &val.generic_data else {
                    return Err(VMError::NoUserTypeBase.into());
                };
                let field = fields.get(*id).ok_or(VMError::InternalVMError)?;
                let dim_1 = self.eval_expr(&arguments[0]).await?.as_int() as usize;
                let dim_2 = if arguments.len() >= 2 {
                    self.eval_expr(&arguments[1]).await?.as_int() as usize
                } else {
                    0
                };
                let dim_3 = if arguments.len() >= 3 {
                    self.eval_expr(&arguments[2]).await?.as_int() as usize
                } else {
                    0
                };
                Ok(field.get_array_value(dim_1, dim_2, dim_3))
            }

            PPEExpr::MemberFunctionCall(base_expr, arguments, id) => {
                let val = self.eval_expr(base_expr).await?;
                let VariableType::UserData(type_id) = val.get_type() else {
                    log::error!("No user type base for value: {val:?} on expr {base_expr:?}");
                    return Err(VMError::NoUserTypeBase.into());
                };
                let Some(registry) = self.type_registry.get_type_from_id(type_id) else {
                    log::error!("No user data registry entry for value: {val:?} type :{type_id} on expr {base_expr:?}");
                    return Err(VMError::TypeNotFoundInRegistry(type_id).into());
                };
                let GenericVariableData::UserData(object) = val.generic_data else {
                    // should never happen.
                    return Err(VMError::NoObjectFound(type_id).into());
                };

                let Some(member) = registry.id_table.get(*id) else {
                    return Err(VMError::InvalidMemberId(type_id, *id).into());
                };
                if let crate::compiler::user_data::UserDataEntry::Field(name) = member {
                    if arguments.len() != 1 {
                        return Err(VMError::InvalidMemberArgumentCount(type_id, *id, 1, arguments.len()).into());
                    }
                    let value = self.eval_expr(&arguments[0]).await?;
                    object.set_property_value(self, name, value).await?;
                    return Ok(VariableValue::new_bool(true));
                }
                let crate::compiler::user_data::UserDataEntry::Function(name) = member else {
                    return Err(VMError::InvalidMemberFunction(type_id, *id).into());
                };
                let Some(function) = registry.functions.get(name) else {
                    return Err(VMError::InvalidMemberFunction(type_id, *id).into());
                };
                if arguments.len() < function.required || arguments.len() > function.parameters.len() {
                    return Err(VMError::InvalidMemberArgumentCount(type_id, *id, function.parameters.len(), arguments.len()).into());
                }

                let mut args = Vec::new();
                for arg in arguments {
                    args.push(self.eval_expr(arg).await?);
                }

                return object.call_function(self, name, &args).await;
            }

            PPEExpr::UnaryExpression(op, expr) => {
                let value = self.eval_expr(expr).await?;
                Ok(Self::apply_unary_op(*op, value))
            }
            PPEExpr::BinaryExpression(op, left, right) => {
                let left_value = self.eval_expr(left).await?;
                let right_value = self.eval_expr(right).await?;
                Ok(Self::apply_bin_op(*op, left_value, right_value))
            }
            PPEExpr::Dim(id, dims) => {
                let dim_1 = self.eval_expr(&dims[0]).await?.as_int() as usize;
                let dim_2 = if dims.len() >= 2 {
                    self.eval_expr(&dims[1]).await?.as_int() as usize
                } else {
                    0
                };
                let dim_3 = if dims.len() >= 3 {
                    self.eval_expr(&dims[2]).await?.as_int() as usize
                } else {
                    0
                };
                Ok(self.variable_table.get_value(*id).get_array_value(dim_1, dim_2, dim_3))
            }

            PPEExpr::PredefinedFunctionCall(func, arguments) => match run_function(func.opcode, self, arguments).await {
                Ok(val) => Ok(val),
                Err(e) => Err(VMError::ErrorInFunctionCall(func.name.to_string(), e.to_string()).into()),
            },

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
                if !self.has_stack_room()? {
                    // No room to call; the function's return variable keeps whatever it held.
                    return Ok(self.variable_table.get_value(return_var_id).clone());
                }
                self.prepare_call(locals, parameters, first, arguments, 0).await?;

                self.return_addresses.push(ReturnAddress::func_call(self.cur_ptr, *func_id));
                self.goto(proc_offset)?;
                self.run().await?;
                self.fpclear = false;
                Ok(self.variable_table.get_value(return_var_id).clone())
            }
        }
    }

    #[async_recursion(?Send)]
    async fn run(&mut self) -> Res<()> {
        let max_ptr = self.script.statements.len();
        while !self.fpclear && self.is_running && self.cur_ptr < max_ptr {
            let p = self.cur_ptr;
            self.cur_ptr += 1;
            let c = self.script.statements[p].command.clone();
            // log::info!("{p}: {c}");
            self.execute_statement(&c).await?;
            self.check_error_trap()?;
        }
        Ok(())
    }

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
    fn check_error_trap(&mut self) -> Res<()> {
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

    async fn eval_array_indices(&mut self, dimensions: &[PPEExpr]) -> Res<(usize, usize, usize)> {
        if dimensions.is_empty() || dimensions.len() > 3 {
            return Err(VMError::InternalVMError.into());
        }
        let first = self.eval_expr(&dimensions[0]).await?.as_int() as usize;
        let second = if dimensions.len() >= 2 {
            self.eval_expr(&dimensions[1]).await?.as_int() as usize
        } else {
            0
        };
        let third = if dimensions.len() >= 3 {
            self.eval_expr(&dimensions[2]).await?.as_int() as usize
        } else {
            0
        };
        Ok((first, second, third))
    }

    async fn set_variable(&mut self, variable: &PPEExpr, value: VariableValue) -> Res<()> {
        match variable {
            PPEExpr::Value(id) => {
                self.variable_table.set_value(*id, value);
            }
            PPEExpr::Dim(id, dims) => {
                let dim_1 = self.eval_expr(&dims[0]).await?.as_int() as usize;
                let dim_2 = if dims.len() >= 2 {
                    self.eval_expr(&dims[1]).await?.as_int() as usize
                } else {
                    0
                };
                let dim_3 = if dims.len() >= 3 {
                    self.eval_expr(&dims[2]).await?.as_int() as usize
                } else {
                    0
                };
                self.variable_table.get_var_entry_mut(*id).value.set_array_value(dim_1, dim_2, dim_3, value)?;
            }
            PPEExpr::Member(_, _) | PPEExpr::IndexedMember(_, _, _) => {
                if let PPEExpr::Member(base, member_id) = variable {
                    let base_value = self.eval_expr(base).await?;
                    if let VariableType::UserData(type_id) = base_value.get_type()
                        && let GenericVariableData::UserData(object) = base_value.generic_data
                    {
                        let registry = self.type_registry.get_type_from_id(type_id).ok_or(VMError::TypeNotFoundInRegistry(type_id))?;
                        let Some(crate::compiler::user_data::UserDataEntry::Field(name)) = registry.id_table.get(*member_id) else {
                            return Err(VMError::InvalidMemberId(type_id, *member_id).into());
                        };
                        return object.set_property_value(self, name, value).await;
                    }
                }

                enum PathStep<'a> {
                    Member(usize),
                    IndexedMember(usize, &'a [PPEExpr]),
                }
                enum ResolvedPathStep {
                    Member(usize),
                    IndexedMember(usize, usize, usize, usize),
                }

                let mut path = Vec::new();
                let mut root = variable;
                loop {
                    match root {
                        PPEExpr::Member(base, member_id) => {
                            path.push(PathStep::Member(*member_id));
                            root = base;
                        }
                        PPEExpr::IndexedMember(base, member_id, dimensions) => {
                            path.push(PathStep::IndexedMember(*member_id, dimensions));
                            root = base;
                        }
                        _ => break,
                    }
                }
                path.reverse();

                let (root_id, root_indices) = match root {
                    PPEExpr::Value(id) => (*id, None),
                    PPEExpr::Dim(id, dimensions) => (*id, Some(self.eval_array_indices(dimensions).await?)),
                    _ => return Err(VMError::InternalVMError.into()),
                };
                let mut resolved_path = Vec::with_capacity(path.len());
                for step in path {
                    resolved_path.push(match step {
                        PathStep::Member(member_id) => ResolvedPathStep::Member(member_id),
                        PathStep::IndexedMember(member_id, dimensions) => {
                            let (first, second, third) = self.eval_array_indices(dimensions).await?;
                            ResolvedPathStep::IndexedMember(member_id, first, second, third)
                        }
                    });
                }

                let root = &mut self.variable_table.get_var_entry_mut(root_id).value;
                let mut target = if let Some((first, second, third)) = root_indices {
                    root.get_array_value_mut(first, second, third).ok_or(VMError::InternalVMError)?
                } else {
                    root
                };
                for step in resolved_path {
                    let GenericVariableData::Record(fields) = &mut target.generic_data else {
                        return Err(VMError::NoUserTypeBase.into());
                    };
                    target = match step {
                        ResolvedPathStep::Member(member_id) => fields.get_mut(member_id).ok_or(VMError::InternalVMError)?,
                        ResolvedPathStep::IndexedMember(member_id, first, second, third) => fields
                            .get_mut(member_id)
                            .ok_or(VMError::InternalVMError)?
                            .get_array_value_mut(first, second, third)
                            .ok_or(VMError::InternalVMError)?,
                    };
                }
                let field_type = target.vtype;
                *target = value.convert_to(field_type);
            }
            _ => {
                return Err(VMError::InternalVMError.into());
            }
        }
        Ok(())
    }

    async fn execute_statement(&mut self, stmt: &PPECommand) -> Res<()> {
        match stmt {
            PPECommand::End => {
                self.is_running = false;
            }

            PPECommand::Stop => {
                self.aborted = true;
                self.is_running = false;
            }

            PPECommand::MemberCall(expr) => {
                self.eval_expr(expr).await?;
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
                                        self.set_variable(&argument_expr, val).await?;
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
                let value = match self.eval_expr_sync(expr) {
                    Some(value) => value,
                    None => self.eval_expr(expr).await?,
                };
                if !value.as_bool() {
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
                self.prepare_call(locals, parameters, first, arguments, pass_flags).await?;

                if self.push_return_address(ReturnAddress::func_call(self.cur_ptr, *proc_id))? {
                    self.goto(proc_offset)?;
                }
            }

            PPECommand::PredefinedCall(proc, arguments) => {
                run_predefined_statement(proc.opcode, self, arguments).await?;
            }

            PPECommand::Goto(label) => {
                self.goto(*label)?;
            }
            PPECommand::Gosub(label) => {
                if self.push_return_address(ReturnAddress::gosub(self.cur_ptr))? {
                    self.goto(*label)?;
                }
            }
            PPECommand::OnError(target) => {
                self.error_handler = match target {
                    OnErrorTarget::Off => ErrorHandler::Off,
                    OnErrorTarget::Goto(label) => ErrorHandler::Goto(*label),
                    OnErrorTarget::Gosub(label) => ErrorHandler::Gosub(*label),
                    OnErrorTarget::Procedure(id) => ErrorHandler::Procedure(*id),
                };
            }
            PPECommand::Let(variable, expr) => {
                let val = match self.eval_expr_sync(expr) {
                    Some(value) => value,
                    None => self.eval_expr(expr).await?,
                };
                self.set_variable(variable, val).await?;
            }
        }

        Ok(())
    }

    #[allow(clippy::needless_range_loop)]
    async fn prepare_call(&mut self, locals: usize, parameters: usize, first: usize, arguments: &[PPEExpr], pass_flags: u16) -> Res<()> {
        self.save_call_frame(locals, parameters, first);
        for i in 0..parameters {
            let id = first + i;
            let value = self.eval_expr(&arguments[i]).await?;
            self.variable_table.set_value(id, value);

            if (1 << i) & pass_flags != 0 {
                self.write_back_stack.push(arguments[i].clone());
            }
        }
        self.init_call_locals(locals, parameters, first);

        Ok(())
    }

    /// The same, for a call the VM makes itself and so has the arguments of already.
    fn prepare_call_with_values(&mut self, locals: usize, parameters: usize, first: usize, arguments: Vec<VariableValue>) {
        self.save_call_frame(locals, parameters, first);
        for (i, value) in arguments.into_iter().take(parameters).enumerate() {
            self.variable_table.set_value(first + i, value);
        }
        self.init_call_locals(locals, parameters, first);
    }

    fn save_call_frame(&mut self, locals: usize, parameters: usize, first: usize) {
        for i in 0..(locals + parameters) {
            let id = first + i;
            if self.variable_table.get_var_entry(id).header.flags & 0x1 == 0x0 {
                let val = self.variable_table.get_value(id).clone();
                self.call_local_value_stack.push(val);
            }
        }
    }

    fn init_call_locals(&mut self, locals: usize, parameters: usize, first: usize) {
        for i in 0..locals {
            let id = first + parameters + i;
            let (flags, vtype) = {
                let header = &self.variable_table.get_var_entry(id).header;
                (header.flags, header.variable_type)
            };
            if (flags & 0x1) == 0x0 {
                let entry = self.variable_table.get_var_entry(id);
                // A record keeps the fields its type declares, emptied out again.
                let val = if matches!(vtype, VariableType::UserData(type_id) if crate::parser::is_user_declared_type(type_id) || type_id as usize == crate::parser::CONTACT_ID)
                {
                    entry.value.emptied()
                } else {
                    let mut val = vtype.create_empty_value();
                    val.generic_data = entry.header.create_generic_data().unwrap_or(GenericVariableData::None);
                    val
                };
                self.variable_table.set_value(id, val);
            }
        }
    }

    fn goto(&mut self, label: usize) -> Result<(), VMError> {
        if let Some(label) = self.label_table.get(&label) {
            self.cur_ptr = *label;
            Ok(())
        } else {
            Err(VMError::LabelNotFound(label))
        }
    }

    pub async fn resolve_file<P: AsRef<Path>>(&self, file: &P) -> PathBuf {
        // A PPE that built this name with MID or a fixed width field hands us the padding
        // as well, and PCBoard opens the file regardless. Verified against PCBoard 15.4.
        let mut file = file.as_ref().to_string_lossy().trim_end().to_string();
        if std::path::MAIN_SEPARATOR == '/' {
            file = file.replace('\\', "/");
        } else if std::path::MAIN_SEPARATOR == '\\' {
            file = file.replace('/', "\\");
        }

        let board_root = self.icy_board_state.get_board().await.root_path.clone();
        let resolved = if let Some(stripped) = file.strip_prefix("C:/") {
            log::warn!("Absolute path detected: {file}, change the src file.");
            self.icy_board_state.get_board().await.resolve_file(&PathBuf::from(stripped))
        } else {
            self.icy_board_state.get_board().await.resolve_file(&file)
        };
        if resolved.exists() {
            return resolved;
        }
        // A bare name is the board's, the way PCBoard read it from its own directory - only a
        // path that leads somewhere else is worth looking for below the PPE.
        if !file.contains(std::path::MAIN_SEPARATOR) {
            return resolved;
        }
        self.resolve_below_ppe(&file, &board_root).unwrap_or(resolved)
    }

    /// An imported PPE still names its files the way they lay on the sysop's DOS drive.
    /// Whatever is left of such a path below the PPE's own directory - or below the board,
    /// which is where the PPE directories went - is where the file lives now.
    fn resolve_below_ppe(&self, file: &str, board_root: &Path) -> Option<PathBuf> {
        let dir = self.file_name.parent()?;
        let from_dos_drive = file.starts_with('/') || file.chars().nth(1) == Some(':');
        let mut rel = PathBuf::from(file);
        loop {
            if rel.is_relative() && !rel.as_os_str().is_empty() {
                for base in [dir, board_root] {
                    let candidate = lookup_case_insensitive(&base.join(&rel));
                    if candidate.exists() {
                        return Some(candidate);
                    }
                }
            }
            // Only a path off a drive holds directories that are gone; a relative one
            // says where the file is and is taken as it stands.
            if !from_dos_drive {
                return None;
            }
            let mut components = rel.components();
            components.next()?;
            let rest = components.as_path().to_path_buf();
            if rest.as_os_str().is_empty() {
                // Nothing of the path is left, so a file still to be written goes next to the PPE.
                let name = PathBuf::from(file).file_name()?.to_os_string();
                return Some(dir.join(name));
            }
            rel = rest;
        }
    }

    /// Whether another call fits on the stack.
    ///
    /// Once it is exhausted the PPE either ends here or, if it turned
    /// `STACKABORT` off, skips the call and carries on with the next statement.
    /// That is as far as "continue after a stack error" can sensibly go.
    fn has_stack_room(&mut self) -> Res<bool> {
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
    fn push_return_address(&mut self, address: ReturnAddress) -> Res<bool> {
        if !self.has_stack_room()? {
            return Ok(false);
        }
        self.return_addresses.push(address);
        Ok(true)
    }

    /// The message base a conference/area pair addresses, or `None` when the
    /// PPE named one that does not exist.
    pub async fn message_base_path(&self, conference: i32, area: i32) -> Option<PathBuf> {
        let board = self.icy_board_state.get_board().await;
        let conf = board.conferences.get(conference as usize)?;
        Some(conf.areas.as_ref()?.get(area as usize)?.path.clone())
    }

    /// Runs `read` against the open message base for `path`, opening it when the last
    /// call was for another area. The handle is what a walk saves: reading a message
    /// costs a seek rather than opening the base again.
    pub fn with_message_base<R>(&mut self, path: &Path, read: impl FnOnce(&mut JamMessageBase) -> jamjam::Result<R>) -> jamjam::Result<R> {
        if self.message_base.as_ref().is_none_or(|(cached, _)| cached != path) {
            self.message_base = Some((path.to_path_buf(), JamMessageBase::open(path)?));
        }
        let Some((_, base)) = self.message_base.as_mut() else {
            unreachable!("the base was just cached")
        };
        read(base)
    }

    /// Forgets the open message base, so the next read opens it again. A write goes
    /// through its own handle, which leaves what this one knows behind.
    pub fn invalidate_message_base(&mut self) {
        self.message_base = None;
        self.cached_msg_header = None;
    }

    async fn set_rip_mouseregion(
        &mut self,
        num: i32,
        x1: i32,
        y1: i32,
        x2: i32,
        y2: i32,
        font_x: i32,
        font_y: i32,
        invert: bool,
        clear: bool,
        text: String,
    ) -> Res<()> {
        let rip_cmd = format!(
            "|M{}{}{}{}{}{}{}{}{}",
            to_base_36(2, num),
            to_base_36(2, (x1 - 1) * font_x),
            to_base_36(2, (y1 - 1) * font_y),
            to_base_36(2, (x2 - 1) * font_x),
            to_base_36(2, (y2 - 1) * font_y),
            i32::from(invert),
            i32::from(clear),
            "00000", // unused
            text
        );
        self.icy_board_state
            .write_raw(TerminalTarget::Both, rip_cmd.chars().collect::<Vec<char>>().as_slice())
            .await
    }
}
/// .
/// # Errors
/// Runs a PPE. Answers `false` when the program gave up with STOP, which is what tells a
/// script questionnaire to drop the answers it collected.
pub async fn run<P: AsRef<Path>>(file_name: &P, prg: &Executable, io: &mut dyn PCBoardIO, icy_board_state: &mut IcyBoardState) -> Res<bool> {
    match PPEScript::from_ppe_file(prg) {
        Ok(script) => {
            let mut label_table = HashMap::new();
            for (i, stmt) in script.statements.iter().enumerate() {
                label_table.insert(stmt.span.start * 2, i);
            }
            let user = if let Some(user) = &icy_board_state.session.current_user {
                user.clone()
            } else {
                User::default()
            };
            let file_name = file_name.as_ref().to_path_buf();
            let reg: UserTypeRegistry = UserTypeRegistry::icy_board_registry();
            log::info!("Run PPE {}", file_name.display());

            let mut vm = VirtualMachine::new(file_name, &reg, io, icy_board_state);
            vm.script = script;
            vm.variable_table = prg.variable_table.clone();
            vm.label_table = label_table;
            vm.user_types = prg.user_types.clone();
            vm.user = user;

            vm.run().await?;
            Ok(!vm.aborted)
        }
        Err(e) => {
            log::error!("Error loading PPE file '{}': {}", file_name.as_ref().display(), e);
            Err(Box::new(VMError::InternalVMError))
        }
    }
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
