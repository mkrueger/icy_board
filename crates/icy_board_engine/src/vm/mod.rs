use crate::Res;
use crate::ast::BinOp;
use crate::ast::Statement;
use crate::ast::UnaryOp;
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
use crate::icy_board::state::NodeState;
use crate::icy_board::state::ppl_error::PplError;
use crate::icy_board::user_base::FSEMode;
use crate::parser::UserTypeRegistry;
use async_recursion::async_recursion;
use icy_engine::TextBuffer;
use jamjam::jam::JamMessageBase;
use jamjam::jam::msg_header::JamMessageHeader;
use std::collections::HashMap;
use std::collections::VecDeque;
use std::path::Path;
use std::path::PathBuf;
use std::sync::Arc;

pub mod expressions;

pub mod statements;
use crate::icy_board::state::IcyBoardState;
use crate::icy_board::user_base::Password;
use crate::icy_board::user_base::User;

use self::expressions::run_function;
pub use self::statements::*;

mod call_stack;
mod error_handling;
mod host_defaults;
pub mod io;
mod record_io;
mod resources;
mod types;
pub use types::{ErrorHandler, ReturnAddress, StackFrame, TerminalTarget, VMError};
mod user_variable_ids;
pub use user_variable_ids::*;

pub mod dbase;
pub use self::io::*;

pub mod errors;
mod tests;
#[cfg(test)]
#[path = "tests/user_snapshots.rs"]
mod user_snapshots;

pub fn calc_stmt_table(blk: &[Statement]) -> HashMap<unicase::Ascii<String>, usize> {
    let mut res = HashMap::new();
    for (i, stmt) in blk.iter().enumerate() {
        if let Statement::Label(label) = stmt {
            res.insert(label.get_label().clone(), i);
        }
    }
    res
}

/// `PCBoard` reads a variable written without subscripts as its first element
/// (`cVAR::getVal(0,0,0)`), so a bare array decays the same way here.
fn decay_array(value: VariableValue) -> VariableValue {
    match value.generic_data {
        GenericVariableData::Dim1(_) | GenericVariableData::Dim2(_) | GenericVariableData::Dim3(_) => value.get_array_value(0, 0, 0),
        _ => value,
    }
}

enum LValuePathStep<'a> {
    Member(usize),
    IndexedMember(usize, &'a [PPEExpr]),
}

enum ResolvedLValuePathStep {
    Member(usize),
    IndexedMember(usize, usize, usize, usize),
}

pub struct WriteBackTarget {
    root_id: usize,
    root_indices: Option<(usize, usize, usize)>,
    path: Vec<ResolvedLValuePathStep>,
}

struct ForEachFrame {
    variable: usize,
    collection: VariableValue,
    index: usize,
    count: usize,
    body_start: usize,
    end: usize,
    call_depth: usize,
}

pub struct VirtualMachine<'a> {
    pub io: &'a mut dyn PCBoardIO,
    pub type_registry: &'a UserTypeRegistry,
    pub file_name: PathBuf,
    pub variable_table: VariableTable,

    pub script: PPEScript,
    commands: Arc<[PPECommand]>,
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
    pub write_back_stack: Vec<WriteBackTarget>,
    pub user_types: Vec<Vec<crate::executable::RecordField>>,

    pub label_table: HashMap<usize, usize>,
    last_jump: Option<(usize, usize)>,
    pub push_pop_stack: Vec<VariableValue>,
    foreach_stack: Vec<ForEachFrame>,

    pub stored_screen: Option<TextBuffer>,

    pub fd_default_in: i32,
    pub fd_default_out: i32,

    pub file_list: VecDeque<String>,
    pub user: User,
    // ACCOUNT refreshes `user`, but must not advance the optimistic edit baseline.
    user_baseline: User,
    user_variable_baseline: Vec<VariableValue>,
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
        let user = icy_board_state.session.current_user.clone().unwrap_or_default();
        Self {
            file_name,
            type_registry,
            return_addresses: Vec::new(),
            script: PPEScript::default(),
            commands: Arc::from([]),
            io,
            is_running: true,
            aborted: false,
            fpclear: false,
            icy_board_state,
            pcb_node: None,
            variable_table: VariableTable::default(),
            cur_ptr: 0,
            label_table: HashMap::new(),
            last_jump: None,
            call_local_value_stack: Vec::new(),
            write_back_stack: Vec::new(),
            user_types: Vec::new(),
            push_pop_stack: Vec::new(),
            foreach_stack: Vec::new(),
            stored_screen: None,
            fd_default_in: 0,
            fd_default_out: 0,
            file_list: VecDeque::new(),
            user_baseline: user.clone(),
            user,
            user_variable_baseline: Vec::new(),
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
    fn select_user(&mut self, user: User) {
        self.user_baseline = user.clone();
        self.user = user;
    }

    fn snapshot_user_variables(&mut self) {
        self.user_variable_baseline = if self.variable_table.has_user_vars() {
            let last = if self.variable_table.get_version() >= 340 {
                U_WEB
            } else if self.variable_table.get_version() >= 300 {
                U_ACCOUNT
            } else {
                U_PWDEXP
            };
            (1..=last).map(|id| self.variable_table.get_value(id).clone()).collect()
        } else {
            Vec::new()
        };
    }

    fn user_variable_changed(&self, id: usize, element: Option<usize>) -> bool {
        let Some(base) = self.user_variable_baseline.get(id - 1) else {
            return true;
        };
        let value = self.variable_table.get_value(id);
        if let Some(index) = element {
            return value.get_array_value(index, 0, 0) != base.get_array_value(index, 0, 0);
        }
        if let (GenericVariableData::Password(left), GenericVariableData::Password(right)) = (&value.generic_data, &base.generic_data) {
            // Password equality verifies secrets; here only identical storage records are unchanged.
            return !matches!((left, right),
                (Password::PlainText(l), Password::PlainText(r)) | (Password::Protected(l), Password::Protected(r))
                | (Password::BCrypt(l), Password::BCrypt(r)) | (Password::Argon2(l), Password::Argon2(r)) if l == r);
        }
        value != base
    }

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
        self.snapshot_user_variables();
        Ok(())
    }

    pub async fn put_user_variables(&self, cur_user: &mut User) {
        if !self.variable_table.has_user_vars() {
            return;
        }
        macro_rules! field {
            ($id:ident, $target:expr, $value:expr) => {
                if self.user_variable_changed($id, None) {
                    $target = $value;
                }
            };
        }
        macro_rules! string {
            ($id:ident, $target:expr) => {
                field!($id, $target, self.variable_table.get_value($id).as_string());
            };
        }
        macro_rules! boolean {
            ($id:ident, $target:expr) => {
                field!($id, $target, self.variable_table.get_value($id).as_bool());
            };
        }
        macro_rules! element {
            ($id:ident, $index:expr, $target:expr) => {
                if self.user_variable_changed($id, Some($index)) {
                    $target = self.variable_table.get_value($id).get_array_value($index, 0, 0).as_string();
                }
            };
        }
        boolean!(U_EXPERT, cur_user.flags.expert_mode);
        if self.user_variable_changed(U_FSE, None) || self.user_variable_changed(U_FSEP, None) {
            cur_user.flags.fse_mode = if self.variable_table.get_value(U_FSE).as_bool() {
                FSEMode::Yes
            } else if self.variable_table.get_value(U_FSEP).as_bool() {
                FSEMode::Ask
            } else {
                FSEMode::No
            };
        }
        boolean!(U_CLS, cur_user.flags.msg_clear);
        field!(
            U_EXPDATE,
            cur_user.expiration_date,
            IcbDate::from_pcboard(self.variable_table.get_value(U_EXPDATE).as_int() as u32).to_utc_date_time()
        );
        field!(U_SEC, cur_user.security_level, self.variable_table.get_value(U_SEC).as_int() as u8);
        field!(U_PAGELEN, cur_user.page_len, self.variable_table.get_value(U_PAGELEN).as_int() as u16);
        field!(U_EXPSEC, cur_user.exp_security_level, self.variable_table.get_value(U_EXPSEC).as_int() as u8);
        string!(U_CITY, cur_user.city_or_state);
        string!(U_BDPHONE, cur_user.bus_data_phone);
        string!(U_HVPHONE, cur_user.home_voice_phone);
        string!(U_TRANS, cur_user.protocol);
        string!(U_CMNT1, cur_user.user_comment);
        string!(U_CMNT2, cur_user.sysop_comment);

        if self.user_variable_changed(U_PWD, None) {
            let pwd_value = self.variable_table.get_value(U_PWD);
            cur_user.password.password = if let GenericVariableData::Password(ref pwd) = pwd_value.generic_data {
                match pwd {
                    Password::PlainText(s) | Password::Protected(s) => self.icy_board_state.create_password(s).await,
                    pwd => pwd.clone(),
                }
            } else {
                Password::new_argon2(pwd_value.as_string())
            };
        }

        boolean!(U_SCROLL, cur_user.flags.scroll_msg_body);
        field!(
            U_LONGHDR,
            cur_user.flags.use_short_filedescr,
            !self.variable_table.get_value(U_LONGHDR).as_bool()
        );
        boolean!(U_DEF79, cur_user.flags.wide_editor);
        string!(U_ALIAS, cur_user.alias);
        string!(U_VER, cur_user.verify_answer);
        element!(U_ADDR, 0, cur_user.street1);
        element!(U_ADDR, 1, cur_user.street2);
        // U_CITY and U_ADDR(2) are the same field; whichever the PPE changed wins.
        element!(U_ADDR, 2, cur_user.city_or_state);
        element!(U_ADDR, 3, cur_user.state);
        element!(U_ADDR, 4, cur_user.zip);
        element!(U_ADDR, 5, cur_user.country);
        element!(U_NOTES, 0, cur_user.custom_comment1);
        element!(U_NOTES, 1, cur_user.custom_comment2);
        element!(U_NOTES, 2, cur_user.custom_comment3);
        element!(U_NOTES, 3, cur_user.custom_comment4);
        element!(U_NOTES, 4, cur_user.custom_comment5);
        field!(
            U_PWDEXP,
            cur_user.password.expire_date,
            IcbDate::from_pcboard(self.variable_table.get_value(U_PWDEXP).as_int() as u32).to_utc_date_time()
        );

        // U_ACCOUNT is not loaded or stored by PCBoard.
        if self.variable_table.get_version() >= 340 {
            boolean!(U_SHORTDESC, cur_user.flags.use_short_filedescr);
            string!(U_GENDER, cur_user.gender);
            field!(
                U_BIRTHDATE,
                cur_user.birth_date,
                IcbDate::parse(&self.variable_table.get_value(U_BIRTHDATE).as_string()).to_utc_date_time()
            );
            string!(U_EMAIL, cur_user.email);
            string!(U_WEB, cur_user.web);
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
            PPEExpr::Value(id) | PPEExpr::RoutineReference(id) => {
                let value = self.variable_table.get_value(*id);
                if self.variable_table.get_version() >= 400 && value.get_dimensions() > 0 && matches!(value.vtype, VariableType::UserData(_)) {
                    return None;
                }
                Some(if value.get_dimensions() > 0 && self.variable_table.is_enum(value.vtype) {
                    self.variable_table.array_value(value, 0, 0, 0)
                } else {
                    decay_array(value.clone())
                })
            }
            PPEExpr::UnaryExpression(op, expr) => {
                let value = self.eval_expr_sync(expr)?;
                Some(Self::apply_unary_op(*op, value))
            }
            PPEExpr::BinaryExpression(op, left, right) => {
                let left_value = self.eval_expr_sync(left)?;
                if let Some(result) = op.short_circuit_result(left_value.as_bool()) {
                    return Some(VariableValue::new_bool(result));
                }
                let right_value = self.eval_expr_sync(right)?;
                Some(Self::apply_bin_op(*op, left_value, right_value))
            }
            PPEExpr::Dim(id, dims) => {
                if self.variable_table.get_version() >= 400 && matches!(self.variable_table.get_value(*id).vtype, VariableType::UserData(_)) {
                    return None;
                }
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
                Some(self.variable_table.array_value(self.variable_table.get_value(*id), dim_1, dim_2, dim_3))
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
            BinOp::Or | BinOp::ShortOr => VariableValue::new_bool(left.as_bool() || right.as_bool()),
            BinOp::And | BinOp::ShortAnd => VariableValue::new_bool(left.as_bool() && right.as_bool()),
            BinOp::Lower => VariableValue::new_bool(left < right),
            BinOp::LowerEq => VariableValue::new_bool(left <= right),
            BinOp::Greater => VariableValue::new_bool(left > right),
            BinOp::GreaterEq => VariableValue::new_bool(left >= right),
        }
    }

    pub async fn eval_expr(&mut self, expr: &PPEExpr) -> Res<VariableValue> {
        if let Some(value) = self.eval_expr_sync(expr) {
            return Ok(value);
        }
        self.eval_expr_async(expr).await
    }

    #[async_recursion(?Send)]
    async fn eval_expr_async(&mut self, expr: &PPEExpr) -> Res<VariableValue> {
        match expr {
            PPEExpr::Invalid => Err(VMError::InternalVMError.into()),
            PPEExpr::Value(id) | PPEExpr::RoutineReference(id) => {
                let value = self.variable_table.get_value(*id);
                if self.variable_table.get_version() >= 400 && value.get_dimensions() > 0 && matches!(value.vtype, VariableType::UserData(_)) {
                    self.read_array_element(value, 0, 0, 0)
                } else {
                    Ok(self.eval_expr_sync(&PPEExpr::Value(*id)).unwrap())
                }
            }
            PPEExpr::RecordLiteral(type_id, fields) => {
                let mut value = self.type_default(VariableType::UserData(*type_id))?;
                let GenericVariableData::Record(values) = &mut value.generic_data else {
                    return Err(VMError::InternalVMError.into());
                };
                let values = Arc::make_mut(values);
                for (field_id, expression) in fields {
                    let field = values.get(*field_id).ok_or(VMError::InvalidMemberId(*type_id, *field_id))?;
                    let field_type = field.vtype;
                    let is_array = matches!(
                        field.generic_data,
                        GenericVariableData::Dim1(_) | GenericVariableData::Dim2(_) | GenericVariableData::Dim3(_)
                    );
                    let field_value = if is_array {
                        self.eval_array_operand(expression).await?
                    } else {
                        self.eval_expr(expression).await?
                    };
                    self.check_record_field_value(*type_id, *field_id, &field_value)?;
                    values[*field_id] = self.variable_table.checked_enum_value(field_type, field_value)?.convert_to(field_type);
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
                        let val = crate::compiler::user_data::runtime_object(object.as_ref(), type_id)?.get_property_value(self, name);
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
                let (dim_1, dim_2, dim_3) = self.eval_array_indices(arguments).await?;
                self.read_array_element(field, dim_1, dim_2, dim_3)
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
                let object = crate::compiler::user_data::runtime_object(object.as_ref(), type_id)?;
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
                if let Some(result) = op.short_circuit_result(left_value.as_bool()) {
                    return Ok(VariableValue::new_bool(result));
                }
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
                self.read_array_element(self.variable_table.get_value(*id), dim_1, dim_2, dim_3)
            }

            PPEExpr::PredefinedFunctionCall(func, arguments)
                if matches!(
                    func.opcode,
                    crate::executable::FuncOpCode::ArrayValueAt2 | crate::executable::FuncOpCode::ArrayValueAt3
                ) =>
            {
                let array = self.eval_array_operand(&arguments[0]).await?;
                let indices = &arguments[1..];
                if array.get_dimensions() as usize != indices.len() {
                    return Err(VMError::InvalidArrayDimensionCount(indices.len()).into());
                }
                let (first, second, third) = self.eval_array_indices(indices).await?;
                self.read_array_element(&array, first, second, third)
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
                // Modern array/record results belong to the current invocation. A recursive
                // call must return its value without replacing the outer result.
                let saved_value_result = (self.variable_table.get_version() >= 400
                    && (self.variable_table.get_var_entry(return_var_id).header.dim > 0
                        || matches!(self.variable_table.get_value(return_var_id).generic_data, GenericVariableData::Record(_))))
                .then(|| self.variable_table.get_value(return_var_id).clone());
                self.prepare_call(locals, parameters, first, arguments, &[]).await?;

                self.return_addresses.push(ReturnAddress::func_call(self.cur_ptr, *func_id));
                self.goto(proc_offset)?;
                self.run_statements(false).await?;
                self.fpclear = false;
                let result = self.variable_table.get_value(return_var_id).clone();
                if let Some(previous) = saved_value_result {
                    *self.variable_table.get_value_mut(return_var_id) = previous;
                }
                Ok(result)
            }
        }
    }

    async fn run(&mut self) -> Res<()> {
        self.run_statements(true).await
    }

    #[async_recursion(?Send)]
    async fn run_statements(&mut self, trap_errors: bool) -> Res<()> {
        let commands = Arc::clone(&self.commands);
        let max_ptr = commands.len();
        while !self.fpclear && self.is_running && self.cur_ptr < max_ptr {
            if self.icy_board_state.session.request_logoff {
                return Err(icy_net::NetError::ConnectionClosed.into());
            }
            let p = self.cur_ptr;
            self.cur_ptr += 1;
            // log::info!("{p}: {c}");
            self.execute_statement(&commands[p]).await?;
            if self.icy_board_state.session.request_logoff {
                return Err(icy_net::NetError::ConnectionClosed.into());
            }
            if trap_errors {
                self.check_error_trap()?;
            } else {
                self.publish_operation_result();
            }
        }
        Ok(())
    }

    async fn eval_array_indices(&mut self, dimensions: &[PPEExpr]) -> Res<(usize, usize, usize)> {
        if dimensions.is_empty() || dimensions.len() > 3 {
            return Err(VMError::InvalidArrayDimensionCount(dimensions.len()).into());
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

    /// The value of an expression without the bare-array decay, for the built-ins that
    /// take a whole array rather than one of its elements.
    pub async fn eval_array_operand(&mut self, expr: &PPEExpr) -> Res<VariableValue> {
        if let PPEExpr::Value(id) = expr {
            return Ok(self.variable_table.get_value(*id).clone());
        }
        self.eval_expr(expr).await
    }

    async fn set_variable(&mut self, variable: &PPEExpr, value: VariableValue) -> Res<()> {
        match variable {
            PPEExpr::Value(id) => {
                // Classic assignments to routine descriptors discard scalar values.
                // The caller has already evaluated the RHS, including side effects.
                // Keep same-kind routine references assignable (also in runtime 400).
                let target_type = self.variable_table.get_var_entry(*id).header.variable_type;
                let value = self.variable_table.checked_enum_value(target_type, value)?;
                if matches!(target_type, VariableType::Function | VariableType::Procedure) && value.get_type() != target_type {
                    return Ok(());
                }
                self.check_nominal_assignment(target_type, value.vtype)?;
                self.check_record_assignment(target_type, self.variable_table.get_var_entry(*id).header.dim, &value)?;
                // Writing a bare array without subscripts reaches its first element,
                // the way reading one does; replacing it would drop the other elements.
                let target = self.variable_table.get_value(*id);
                if matches!(
                    target.generic_data,
                    GenericVariableData::Dim1(_) | GenericVariableData::Dim2(_) | GenericVariableData::Dim3(_)
                ) && !matches!(
                    value.generic_data,
                    GenericVariableData::Dim1(_) | GenericVariableData::Dim2(_) | GenericVariableData::Dim3(_)
                ) {
                    self.variable_table.get_var_entry_mut(*id).value.set_array_value(0, 0, 0, value)?;
                } else {
                    self.variable_table.set_value(*id, value);
                }
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
                let target_type = self.variable_table.get_var_entry(*id).header.variable_type;
                let value = self.variable_table.checked_enum_value(target_type, value)?;
                self.check_nominal_assignment(target_type, value.vtype)?;
                self.check_record_assignment(target_type, 0, &value)?;
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
                        return crate::compiler::user_data::runtime_object(object.as_ref(), type_id)?
                            .set_property_value(self, name, value)
                            .await;
                    }
                }

                let mut path = Vec::new();
                let mut root = variable;
                loop {
                    match root {
                        PPEExpr::Member(base, member_id) => {
                            path.push(LValuePathStep::Member(*member_id));
                            root = base;
                        }
                        PPEExpr::IndexedMember(base, member_id, dimensions) => {
                            path.push(LValuePathStep::IndexedMember(*member_id, dimensions));
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
                        LValuePathStep::Member(member_id) => ResolvedLValuePathStep::Member(member_id),
                        LValuePathStep::IndexedMember(member_id, dimensions) => {
                            let (first, second, third) = self.eval_array_indices(dimensions).await?;
                            ResolvedLValuePathStep::IndexedMember(member_id, first, second, third)
                        }
                    });
                }

                self.set_record_target(
                    &WriteBackTarget {
                        root_id,
                        root_indices,
                        path: resolved_path,
                    },
                    value,
                )?;
            }
            _ => {
                return Err(VMError::InternalVMError.into());
            }
        }
        Ok(())
    }

    fn set_record_target(&mut self, destination: &WriteBackTarget, value: VariableValue) -> Res<()> {
        let root_id = destination.root_id;
        let mut root_value = self.variable_table.get_value(root_id).clone();
        let root = &mut root_value;
        let mut target = if let Some((first, second, third)) = destination.root_indices {
            root.get_array_value_mut(first, second, third).ok_or(VMError::InternalVMError)?
        } else {
            root
        };
        let mut field_layout = None;
        for step in &destination.path {
            let VariableType::UserData(type_id) = target.vtype else {
                return Err(VMError::NoUserTypeBase.into());
            };
            let GenericVariableData::Record(fields) = &mut target.generic_data else {
                return Err(VMError::NoUserTypeBase.into());
            };
            let fields = Arc::make_mut(fields);
            target = match step {
                ResolvedLValuePathStep::Member(member_id) => {
                    field_layout = Some(self.record_field_layout(type_id, *member_id)?);
                    fields.get_mut(*member_id).ok_or(VMError::InternalVMError)?
                }
                ResolvedLValuePathStep::IndexedMember(member_id, first, second, third) => {
                    let field = self.record_field_layout(type_id, *member_id)?;
                    if field.dim == 0 {
                        return Err(VMError::RecordFieldShapeMismatch.into());
                    }
                    field_layout = Some(crate::executable::RecordField::scalar(field.variable_type));
                    fields
                        .get_mut(*member_id)
                        .ok_or(VMError::InternalVMError)?
                        .get_array_value_mut(*first, *second, *third)
                        .ok_or(VMError::InternalVMError)?
                }
            };
        }
        let field = field_layout.ok_or(VMError::InternalVMError)?;
        let field_type = field.variable_type;
        self.check_record_value(field, &value)?;
        *target = self.variable_table.checked_enum_value(field_type, value)?.convert_to(field_type);
        self.variable_table.set_value(root_id, root_value);
        Ok(())
    }

    async fn execute_statement(&mut self, stmt: &PPECommand) -> Res<()> {
        match stmt {
            PPECommand::End => {
                self.foreach_stack.clear();
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
                let call_depth = self.return_addresses.len();
                self.foreach_stack.retain(|frame| frame.call_depth < call_depth);
                if let Some(addr) = self.return_addresses.pop() {
                    self.cur_ptr = addr.get_ptr();
                    let proc_id = addr.get_id();
                    if proc_id > 0 {
                        let locals;
                        let first;
                        let parameters;
                        let return_var_id;
                        let is_func;
                        unsafe {
                            let proc = &self.variable_table.get_var_entry(proc_id);
                            first = (proc.value.data.procedure_value.first_var_id + 1) as usize;
                            locals = proc.value.data.procedure_value.local_variables as usize;
                            parameters = proc.value.data.procedure_value.parameters as usize;
                            if proc.header.variable_type == VariableType::Function {
                                is_func = true;
                                return_var_id = proc.value.data.function_value.return_var as usize;
                            } else {
                                is_func = false;
                                return_var_id = 0;
                            }
                        }

                        let pass_modes = self.variable_table.parameter_modes(proc_id, parameters);
                        let pass_count = pass_modes.iter().filter(|mode| **mode).count();
                        let legacy_copy_out = self.variable_table.get_version() < 400;
                        if legacy_copy_out {
                            for parameter in (0..parameters).rev() {
                                if pass_modes[parameter] {
                                    let value = self.call_parameter_value(first + parameter);
                                    let target = self.write_back_stack.pop().ok_or(VMError::WriteBackStackEmpty)?;
                                    self.set_write_back_target(&target, value).await?;
                                }
                            }
                        }
                        let single_pass_value = if !legacy_copy_out && pass_count == 1 {
                            let parameter = pass_modes.iter().position(|mode| *mode).unwrap();
                            (parameter < parameters).then(|| self.call_parameter_value(first + parameter))
                        } else {
                            None
                        };
                        let mut pass_values = Vec::new();
                        if !legacy_copy_out && pass_count > 0 && single_pass_value.is_none() {
                            for i in 0..parameters {
                                if pass_modes[i] {
                                    let id = first + i;
                                    let val = self.call_parameter_value(id);
                                    pass_values.push(val);
                                }
                            }
                        }

                        // write back locals + parameters
                        for i in (0..(locals + parameters)).rev() {
                            let id = first + i;
                            let legacy_parameter = i < parameters && self.is_legacy_array_parameter(id);
                            if legacy_parameter
                                || self.variable_table.get_var_entry(id).header.flags & crate::executable::variable_table::VARIABLE_FLAG_STATIC == 0
                            {
                                let Some(value) = self.call_local_value_stack.pop() else {
                                    return Err(VMError::PushPopStackEmpty.into());
                                };
                                if id != return_var_id {
                                    if legacy_parameter {
                                        self.set_call_parameter(id, value)?;
                                    } else {
                                        *self.variable_table.get_value_mut(id) = value;
                                    }
                                }
                            }
                        }

                        if let Some(value) = single_pass_value {
                            if let Some(argument_expr) = self.write_back_stack.pop() {
                                self.set_write_back_target(&argument_expr, value).await?;
                            } else {
                                return Err(VMError::WriteBackStackEmpty.into());
                            }
                        } else if !legacy_copy_out && pass_count > 0 {
                            for i in (0..parameters).rev() {
                                if pass_modes[i] {
                                    let Some(val) = pass_values.pop() else {
                                        return Err(VMError::PassValueStackEmpty.into());
                                    };
                                    if let Some(argument_expr) = self.write_back_stack.pop() {
                                        self.set_write_back_target(&argument_expr, val).await?;
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
                    self.cleanup_foreach_for_jump(*label);
                    self.goto(*label)?;
                }
            }

            PPECommand::ProcedureCall(proc_id, arguments) => {
                let proc_offset;
                let locals;
                let parameters;
                let first;

                unsafe {
                    let proc = &self.variable_table.get_var_entry(*proc_id);
                    proc_offset = proc.value.data.procedure_value.start_offset as usize;
                    first = (proc.value.data.procedure_value.first_var_id + 1) as usize;
                    locals = proc.value.data.procedure_value.local_variables as usize;
                    parameters = proc.value.data.procedure_value.parameters as usize;
                }
                let pass_modes = self.variable_table.parameter_modes(*proc_id, parameters);
                self.prepare_call(locals, parameters, first, arguments, &pass_modes).await?;

                if self.push_return_address(ReturnAddress::func_call(self.cur_ptr, *proc_id))? {
                    self.goto(proc_offset)?;
                }
            }

            PPECommand::PredefinedCall(proc, arguments) => {
                run_predefined_statement(proc.opcode, self, arguments).await?;
            }

            PPECommand::Goto(label) => {
                self.cleanup_foreach_for_jump(*label);
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
                // 4.00 type-checks whole-array operands before emitting LET.
                // Preserve the RHS value for bounded targets and record fields
                // too; classic PPEs retain their historical bare-array decay.
                let val = if self.variable_table.get_version() >= 400 {
                    self.eval_array_operand(expr).await?
                } else {
                    match self.eval_expr_sync(expr) {
                        Some(value) => value,
                        None => self.eval_expr(expr).await?,
                    }
                };
                self.set_variable(variable, val).await?;
            }
            PPECommand::ForEach(variable, collection, end) => {
                let collection = self.eval_array_operand(collection).await?;
                let header = &self.variable_table.get_var_entry(*variable).header;
                if collection.get_dimensions() == 0 || header.dim != 0 || matches!(header.variable_type, VariableType::Function | VariableType::Procedure) {
                    return Err(VMError::InvalidForEach.into());
                }
                self.check_nominal_assignment(header.variable_type, collection.vtype)?;
                let count = Self::foreach_element_count(&collection);
                if count == 0 {
                    self.goto(*end)?;
                } else {
                    let element = Self::foreach_element_at(&collection, 0);
                    self.set_variable(&PPEExpr::Value(*variable), element).await?;
                    let body_start = self.script.statements.get(self.cur_ptr).map_or(*end, |statement| statement.span.start * 2);
                    self.foreach_stack.push(ForEachFrame {
                        variable: *variable,
                        collection,
                        index: 0,
                        count,
                        body_start,
                        end: *end,
                        call_depth: self.return_addresses.len(),
                    });
                }
            }
            PPECommand::NextForEach(start) => {
                let Some(frame) = self.foreach_stack.last_mut() else {
                    return Err(VMError::InternalVMError.into());
                };
                frame.index += 1;
                if frame.index >= frame.count {
                    let end = frame.end;
                    self.foreach_stack.pop();
                    self.goto(end)?;
                } else {
                    let variable = frame.variable;
                    let index = frame.index;
                    let element = Self::foreach_element_at(&frame.collection, index);
                    self.set_variable(&PPEExpr::Value(variable), element).await?;
                    self.goto(*start)?;
                }
            }
        }

        Ok(())
    }

    fn check_nominal_assignment(&self, expected: VariableType, actual: VariableType) -> Res<()> {
        if expected != actual && (matches!(expected, VariableType::UserData(_)) || matches!(actual, VariableType::UserData(_))) {
            return Err(VMError::AssignmentTypeMismatch(expected, actual).into());
        }
        Ok(())
    }

    fn record_field_layout(&self, type_id: u32, field_id: usize) -> Res<crate::executable::RecordField> {
        if type_id as usize == crate::parser::CONTACT_ID && field_id < 2 {
            return Ok(crate::executable::RecordField::scalar(VariableType::UnboundedString));
        }
        (type_id as usize)
            .checked_sub(crate::parser::FIRST_USER_TYPE_ID)
            .and_then(|index| self.user_types.get(index))
            .and_then(|fields| fields.get(field_id))
            .copied()
            .ok_or_else(|| VMError::InvalidMemberId(type_id, field_id).into())
    }

    fn check_record_field_value(&self, type_id: u32, field_id: usize, value: &VariableValue) -> Res<()> {
        self.check_record_value(self.record_field_layout(type_id, field_id)?, value)
    }

    fn check_record_assignment(&self, expected: VariableType, rank: u8, value: &VariableValue) -> Res<()> {
        if matches!(expected, VariableType::UserData(id) if crate::parser::is_user_declared_type(id)) && !self.variable_table.is_enum(expected) {
            let mut field = crate::executable::RecordField::scalar(expected);
            // Bare scalar writes to array variables still address element zero.
            field.dim = if value.get_dimensions() == 0 { 0 } else { rank };
            field.is_dynamic = field.dim > 0;
            self.check_record_value(field, value)?;
        }
        Ok(())
    }

    /// Validate against declarations, not current storage: only dynamic fields may change bounds.
    fn check_record_value(&self, field: crate::executable::RecordField, value: &VariableValue) -> Res<()> {
        let expected = field.variable_type;
        let is_enum = self.variable_table.is_enum(expected);
        if is_enum {
            self.variable_table.checked_enum_value(expected, value.clone())?;
        } else {
            self.check_nominal_assignment(expected, value.vtype)?;
        }
        if field.element_count().is_none() || value.get_dimensions() != field.dim {
            return Err(VMError::RecordFieldShapeMismatch.into());
        }
        let compatible_element_type = |actual| {
            actual == expected
                || (is_enum && actual == VariableType::Integer)
                || (matches!(expected, VariableType::String | VariableType::BigStr | VariableType::UnboundedString)
                    && matches!(actual, VariableType::String | VariableType::BigStr | VariableType::UnboundedString))
        };
        let check_element = |element: &VariableValue| -> Res<()> {
            if !compatible_element_type(element.vtype) {
                return Err(VMError::AssignmentTypeMismatch(expected, element.vtype).into());
            }
            self.check_record_value(crate::executable::RecordField::scalar(expected), element)
        };
        if field.dim > 0 && !compatible_element_type(value.vtype) {
            return Err(VMError::AssignmentTypeMismatch(expected, value.vtype).into());
        }
        let bounds = [field.vector_size as usize + 1, field.matrix_size as usize + 1, field.cube_size as usize + 1];
        match &value.generic_data {
            GenericVariableData::Dim1(values) => {
                if !field.is_dynamic && values.len() != bounds[0] {
                    return Err(VMError::RecordFieldShapeMismatch.into());
                }
                for element in values.iter() {
                    check_element(element)?;
                }
            }
            GenericVariableData::Dim2(values) => {
                let columns = if field.is_dynamic { values.first().map_or(0, Vec::len) } else { bounds[1] };
                if !field.is_dynamic && values.len() != bounds[0] {
                    return Err(VMError::RecordFieldShapeMismatch.into());
                }
                for row in values.iter() {
                    if row.len() != columns {
                        return Err(VMError::RecordFieldShapeMismatch.into());
                    }
                    for element in row {
                        check_element(element)?;
                    }
                }
            }
            GenericVariableData::Dim3(values) => {
                let rows = if field.is_dynamic { values.first().map_or(0, Vec::len) } else { bounds[1] };
                let columns = if field.is_dynamic {
                    values.first().and_then(|plane| plane.first()).map_or(0, Vec::len)
                } else {
                    bounds[2]
                };
                if !field.is_dynamic && values.len() != bounds[0] {
                    return Err(VMError::RecordFieldShapeMismatch.into());
                }
                for plane in values.iter() {
                    if plane.len() != rows {
                        return Err(VMError::RecordFieldShapeMismatch.into());
                    }
                    for row in plane {
                        if row.len() != columns {
                            return Err(VMError::RecordFieldShapeMismatch.into());
                        }
                        for element in row {
                            check_element(element)?;
                        }
                    }
                }
            }
            _ => {
                let record_type = match expected {
                    VariableType::UserData(id) if !is_enum && (crate::parser::is_user_declared_type(id) || id as usize == crate::parser::CONTACT_ID) => {
                        Some(id)
                    }
                    _ => None,
                };
                if let Some(type_id) = record_type {
                    let field_count = if type_id as usize == crate::parser::CONTACT_ID {
                        2
                    } else {
                        self.user_types
                            .get(type_id as usize - crate::parser::FIRST_USER_TYPE_ID)
                            .ok_or(VMError::TypeNotFoundInRegistry(type_id))?
                            .len()
                    };
                    let GenericVariableData::Record(values) = &value.generic_data else {
                        return Err(VMError::RecordFieldShapeMismatch.into());
                    };
                    if values.len() != field_count {
                        return Err(VMError::RecordFieldShapeMismatch.into());
                    }
                    for (field_id, value) in values.iter().enumerate() {
                        self.check_record_field_value(type_id, field_id, value)?;
                    }
                } else if matches!(value.generic_data, GenericVariableData::Record(_)) {
                    return Err(VMError::RecordFieldShapeMismatch.into());
                }
            }
        }
        Ok(())
    }

    fn foreach_element_count(collection: &VariableValue) -> usize {
        match &collection.generic_data {
            GenericVariableData::Dim1(items) => items.len(),
            GenericVariableData::Dim2(items) => items.iter().map(Vec::len).sum(),
            GenericVariableData::Dim3(items) => items.iter().flatten().map(Vec::len).sum(),
            _ => 1,
        }
    }

    fn foreach_element_at(collection: &VariableValue, index: usize) -> VariableValue {
        let element = match &collection.generic_data {
            GenericVariableData::Dim1(items) => items.get(index).cloned(),
            GenericVariableData::Dim2(items) => items.first().and_then(|row| {
                let columns = row.len();
                (columns > 0).then(|| items.get(index / columns)?.get(index % columns).cloned()).flatten()
            }),
            GenericVariableData::Dim3(items) => items.first().and_then(|plane| {
                let rows = plane.len();
                let columns = plane.first()?.len();
                let plane_size = rows.checked_mul(columns)?;
                (plane_size > 0)
                    .then(|| items.get(index / plane_size)?.get(index % plane_size / columns)?.get(index % columns).cloned())
                    .flatten()
            }),
            _ if index == 0 => Some(collection.clone()),
            _ => None,
        };
        element.unwrap_or_else(|| collection.vtype.create_empty_value())
    }

    fn cleanup_foreach_for_jump(&mut self, target: usize) {
        let call_depth = self.return_addresses.len();
        while self
            .foreach_stack
            .last()
            .is_some_and(|frame| frame.call_depth == call_depth && !(frame.body_start <= target && target < frame.end))
        {
            self.foreach_stack.pop();
        }
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
            for (index, statement) in script.statements.iter().enumerate() {
                label_table.insert(statement.span.start * 2, index);
            }
            let commands = script.statements.iter().map(|statement| statement.command.clone()).collect::<Vec<_>>().into();
            let user = if let Some(user) = &icy_board_state.session.current_user {
                user.clone()
            } else {
                User::default()
            };
            let file_name = file_name.as_ref().to_path_buf();
            let reg = crate::parser::icy_board_registry();
            log::info!("Run PPE {}", file_name.display());

            let mut vm = VirtualMachine::new(file_name, &reg, io, icy_board_state);
            vm.script = script;
            vm.commands = commands;
            vm.variable_table = prg.variable_table.clone();
            vm.label_table = label_table;
            vm.user_types = prg.user_types.clone();
            vm.initialize_host_defaults()?;
            vm.select_user(user);
            vm.snapshot_user_variables();

            vm.run().await?;
            Ok(!vm.aborted)
        }
        Err(e) => {
            log::error!("Error loading PPE file '{}': {}", file_name.as_ref().display(), e);
            Err(Box::new(VMError::InternalVMError))
        }
    }
}

#[cfg(test)]
mod followup_invariants {
    use super::*;
    use crate::executable::{RecordField, TableEntry, VariableData, create_record_value};
    use icy_net::{ConnectionType, channel::ChannelConnection};

    async fn state() -> IcyBoardState {
        let bbs = Arc::new(tokio::sync::Mutex::new(crate::icy_board::bbs::BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (_peer, connection) = ChannelConnection::create_pair();
        IcyBoardState::new(
            bbs,
            Arc::new(tokio::sync::Mutex::new(crate::icy_board::IcyBoard::new())),
            nodes,
            node,
            Box::new(connection),
        )
        .await
    }

    fn s1_record(type_id: u32, fields: Vec<VariableValue>) -> VariableValue {
        VariableValue {
            vtype: VariableType::UserData(type_id),
            data: VariableData::default(),
            generic_data: GenericVariableData::Record(Arc::new(fields)),
        }
    }

    fn s1_array(element: VariableValue, rank: u8, bound: usize) -> VariableValue {
        VariableValue {
            vtype: element.vtype,
            data: VariableData::default(),
            generic_data: GenericVariableData::create_array(element, rank, bound, bound, bound).unwrap(),
        }
    }

    fn s1_push(vm: &mut VirtualMachine<'_>, value: VariableValue) -> PPEExpr {
        let mut entry = TableEntry::default();
        entry.header.id = vm.variable_table.get_entries().len() + 1;
        entry.header.variable_type = value.vtype;
        entry.header.dim = value.get_dimensions();
        entry.value = value;
        let expression = PPEExpr::Value(entry.header.id);
        vm.variable_table.push(entry);
        expression
    }

    fn s1_layouts(vm: &mut VirtualMachine<'_>, rank: u8) {
        vm.variable_table.set_version(400);
        vm.variable_table.enums.insert(102, vec![7, 9]);
        vm.user_types = vec![
            vec![
                RecordField {
                    dim: 1,
                    vector_size: 1,
                    ..RecordField::scalar(VariableType::Integer)
                },
                RecordField {
                    dim: rank,
                    is_dynamic: true,
                    ..RecordField::scalar(VariableType::Integer)
                },
                RecordField::scalar(VariableType::UserData(102)),
            ],
            vec![
                RecordField::scalar(VariableType::UserData(100)),
                RecordField {
                    dim: 1,
                    ..RecordField::scalar(VariableType::UserData(100))
                },
            ],
            vec![],
        ];
        vm.variable_table.fill_in_records(&vm.user_types);
    }

    #[tokio::test]
    async fn s1_dynamic_field_bytecode_accepts_resizing_through_nested_and_whole_record_writes() {
        let mut state = state().await;
        let registry = crate::parser::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        for rank in 1..=3 {
            for path in ["field", "indexed field", "array root field", "child", "literal", "record", "array", "element"] {
                let mut vm = VirtualMachine::new("s1-shapes.ppe".into(), &registry, &mut io, &mut state);
                s1_layouts(&mut vm, rank);
                let initial = create_record_value(101, &vm.user_types, &vm.variable_table.enums).unwrap();
                let root = s1_push(&mut vm, initial.clone());
                let roots = s1_push(&mut vm, s1_array(initial.clone(), 1, 0));
                let alias = s1_push(&mut vm, initial.clone());
                let zero = s1_push(&mut vm, VariableValue::new_int(0));
                let items = s1_array(VariableValue::new_int(42), rank, 2);
                let child = s1_record(
                    100,
                    vec![
                        s1_array(VariableValue::new_int(0), 1, 1),
                        items.clone(),
                        VariableValue::new_enum(VariableType::UserData(102), 7, 7),
                    ],
                );
                let outer = s1_record(101, vec![child.clone(), s1_array(child.clone(), 1, 0)]);
                let PPEExpr::Value(roots_id) = roots else { unreachable!() };
                let element = PPEExpr::Dim(roots_id, vec![zero.clone()]);
                let (target, expected) = match path {
                    "field" | "literal" => (PPEExpr::Member(Box::new(PPEExpr::Member(Box::new(root.clone()), 0)), 1), items),
                    "indexed field" => (
                        PPEExpr::Member(Box::new(PPEExpr::IndexedMember(Box::new(root.clone()), 1, vec![zero.clone()])), 1),
                        items,
                    ),
                    "array root field" => (PPEExpr::Member(Box::new(PPEExpr::Member(Box::new(element), 0)), 1), items),
                    "child" => (PPEExpr::Member(Box::new(root.clone()), 0), child),
                    "record" => (root.clone(), outer),
                    "array" => (PPEExpr::Value(roots_id), s1_array(outer, 1, 2)),
                    "element" => (element, outer),
                    _ => unreachable!(),
                };
                let source = s1_push(&mut vm, expected.clone());
                let command = if path == "literal" {
                    PPECommand::Let(
                        Box::new(PPEExpr::Member(Box::new(root.clone()), 0)),
                        Box::new(PPEExpr::RecordLiteral(100, vec![(1, source)])),
                    )
                } else {
                    PPECommand::Let(Box::new(target.clone()), Box::new(source))
                };
                vm.execute_statement(&command).await.unwrap();
                assert_eq!(expected, vm.eval_array_operand(&target).await.unwrap(), "rank={rank}, {path}");
                assert_eq!(initial, vm.eval_expr(&alias).await.unwrap(), "shared copy changed: rank={rank}, {path}");
            }
        }
    }

    #[tokio::test]
    async fn s1_review_string_field_writes_convert_each_element_without_accepting_other_types() {
        let mut state = state().await;
        let registry = crate::parser::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        let string_types = [VariableType::String, VariableType::BigStr, VariableType::UnboundedString];
        for rank in 1..=3 {
            for target_type in string_types {
                for source_type in string_types {
                    let mut vm = VirtualMachine::new("string-fields.ppe".into(), &registry, &mut io, &mut state);
                    vm.variable_table.set_version(400);
                    vm.user_types = vec![vec![RecordField {
                        dim: rank,
                        ..RecordField::scalar(target_type)
                    }]];
                    let initial = create_record_value(100, &vm.user_types, &vm.variable_table.enums).unwrap();
                    let root = s1_push(&mut vm, initial);
                    let target = PPEExpr::Member(Box::new(root.clone()), 0);
                    let element = VariableValue::new_unbounded_string("é".repeat(3000)).convert_to(source_type);
                    let expected = element.clone().convert_to(target_type);
                    let source_array = s1_array(element, rank, 0);
                    let source = s1_push(&mut vm, source_array.clone());
                    vm.execute_statement(&PPECommand::Let(Box::new(target.clone()), Box::new(source)))
                        .await
                        .unwrap();
                    let actual = vm.eval_array_operand(&target).await.unwrap();
                    assert_eq!(actual.vtype, target_type);
                    let actual_element = actual.get_array_value(0, 0, 0);
                    assert_eq!(actual_element.vtype, target_type);
                    assert_eq!(actual_element.as_string(), expected.as_string());
                    let before = vm.eval_expr(&root).await.unwrap();
                    for invalid in ["array", "element"] {
                        let mut array = source_array.clone();
                        if invalid == "array" {
                            array.vtype = VariableType::Integer;
                        } else {
                            *array.get_array_value_mut(0, 0, 0).unwrap() = VariableValue::new_int(42);
                        }
                        let source = s1_push(&mut vm, array);
                        assert!(
                            vm.execute_statement(&PPECommand::Let(Box::new(target.clone()), Box::new(source)))
                                .await
                                .is_err()
                        );
                        assert_eq!(vm.eval_expr(&root).await.unwrap(), before, "{invalid}, rank {rank}");
                    }
                }
            }
        }
    }

    #[tokio::test]
    async fn s1_invalid_record_bytecode_is_transactional_for_all_assignment_paths() {
        let mut state = state().await;
        let registry = crate::parser::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        for rank in 1..=3 {
            for invalid in [
                "rank",
                "array type",
                "element type",
                "nested fixed shape",
                "enum",
                "nominal",
                "field count",
                "ragged",
            ] {
                if invalid == "ragged" && rank == 1 {
                    continue;
                }
                for path in ["child", "indexed child", "record", "array", "element", "array root child", "literal"] {
                    let mut vm = VirtualMachine::new("s1-atomic.ppe".into(), &registry, &mut io, &mut state);
                    s1_layouts(&mut vm, rank);
                    let initial = create_record_value(101, &vm.user_types, &vm.variable_table.enums).unwrap();
                    let root = s1_push(&mut vm, initial.clone());
                    let roots = s1_push(&mut vm, s1_array(initial.clone(), 1, 0));
                    s1_push(&mut vm, initial);
                    let zero = s1_push(&mut vm, VariableValue::new_int(0));
                    let mut child = s1_record(
                        100,
                        vec![
                            s1_array(VariableValue::new_int(0), 1, 1),
                            s1_array(VariableValue::new_int(42), rank, 2),
                            VariableValue::new_enum(VariableType::UserData(102), 7, 7),
                        ],
                    );
                    let GenericVariableData::Record(fields) = &mut child.generic_data else {
                        unreachable!()
                    };
                    let fields = Arc::make_mut(fields);
                    match invalid {
                        "rank" => fields[1] = s1_array(VariableValue::new_int(0), rank % 3 + 1, 0),
                        "array type" => fields[1] = s1_array(VariableValue::new_bool(false), rank, 0),
                        "element type" => *fields[1].get_array_value_mut(2, 2, 2).unwrap() = VariableValue::new_bool(false),
                        "nested fixed shape" => fields[0] = s1_array(VariableValue::new_int(0), 1, 2),
                        "enum" => fields[2] = VariableValue::new_enum(VariableType::UserData(99), 99, 7),
                        "nominal" => child.vtype = VariableType::UserData(101),
                        "field count" => {
                            fields.pop();
                        }
                        "ragged" => match &mut fields[1].generic_data {
                            GenericVariableData::Dim2(rows) => {
                                Arc::make_mut(rows)[1].pop();
                            }
                            GenericVariableData::Dim3(planes) => {
                                Arc::make_mut(planes)[1][1].pop();
                            }
                            _ => unreachable!(),
                        },
                        _ => unreachable!(),
                    }
                    let outer = s1_record(101, vec![child.clone(), s1_array(child.clone(), 1, 0)]);
                    let PPEExpr::Value(roots_id) = roots else { unreachable!() };
                    let element = PPEExpr::Dim(roots_id, vec![zero.clone()]);
                    let (target, value) = match path {
                        "child" | "literal" => (PPEExpr::Member(Box::new(root.clone()), 0), child),
                        "indexed child" => (PPEExpr::IndexedMember(Box::new(root.clone()), 1, vec![zero]), child),
                        "record" => (root, outer),
                        "array" => (PPEExpr::Value(roots_id), s1_array(outer, 1, 1)),
                        "element" => (element, outer),
                        "array root child" => (PPEExpr::Member(Box::new(element), 0), child),
                        _ => unreachable!(),
                    };
                    let source = s1_push(&mut vm, value);
                    let before: Vec<_> = vm.variable_table.get_entries().iter().map(|entry| entry.value.clone()).collect();
                    let command = if path == "literal" {
                        PPECommand::Let(Box::new(PPEExpr::Value(1)), Box::new(PPEExpr::RecordLiteral(101, vec![(0, source)])))
                    } else {
                        PPECommand::Let(Box::new(target), Box::new(source))
                    };
                    assert!(vm.execute_statement(&command).await.is_err(), "rank={rank}, {invalid}, {path}");
                    let after: Vec<_> = vm.variable_table.get_entries().iter().map(|entry| entry.value.clone()).collect();
                    assert_eq!(before, after, "rank={rank}, {invalid}, {path}");
                }
            }
        }
    }

    #[tokio::test]
    async fn s1_dynamic_record_and_enum_arrays_check_empty_types_and_every_element() {
        let mut state = state().await;
        let registry = crate::parser::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        for rank in 1..=3 {
            for field_id in 0..2 {
                let mut vm = VirtualMachine::new("s1-elements.ppe".into(), &registry, &mut io, &mut state);
                s1_layouts(&mut vm, rank);
                vm.user_types.push(
                    [100, 102]
                        .map(|type_id| RecordField {
                            dim: rank,
                            is_dynamic: true,
                            ..RecordField::scalar(VariableType::UserData(type_id))
                        })
                        .to_vec(),
                );
                vm.variable_table.fill_in_records(&vm.user_types);
                let initial = create_record_value(103, &vm.user_types, &vm.variable_table.enums).unwrap();
                let root = s1_push(&mut vm, initial);
                let target = PPEExpr::Member(Box::new(root.clone()), field_id);
                let empty = vm.eval_array_operand(&target).await.unwrap();
                let element_type = if field_id == 0 { 100 } else { 102 };
                let element = create_record_value(element_type, &vm.user_types, &vm.variable_table.enums).unwrap();
                let valid = s1_array(element.clone(), rank, 1);
                for value in [valid.clone(), empty.clone(), valid.clone()] {
                    let source = s1_push(&mut vm, value.clone());
                    vm.execute_statement(&PPECommand::Let(Box::new(target.clone()), Box::new(source)))
                        .await
                        .unwrap();
                    assert_eq!(value, vm.eval_array_operand(&target).await.unwrap());
                }
                let mut invalid = valid;
                let leaf = invalid.get_array_value_mut(1, 1, 1).unwrap();
                if let GenericVariableData::Record(fields) = &mut leaf.generic_data {
                    Arc::make_mut(fields)[0] = s1_array(VariableValue::new_int(0), 1, 2);
                } else {
                    *leaf = VariableValue::new_enum(VariableType::UserData(99), 99, 7);
                }
                let wrong_empty = VariableValue {
                    vtype: VariableType::Boolean,
                    ..empty
                };
                for value in [invalid, wrong_empty] {
                    let before = vm.eval_expr(&root).await.unwrap();
                    let source = s1_push(&mut vm, value);
                    assert!(
                        vm.execute_statement(&PPECommand::Let(Box::new(target.clone()), Box::new(source)))
                            .await
                            .is_err()
                    );
                    assert_eq!(before, vm.eval_expr(&root).await.unwrap(), "rank={rank}, field={field_id}");
                }
            }
        }
    }

    #[tokio::test]
    async fn s1_record_io_bytecode_rejects_layouts_before_reading_or_writing() {
        use crate::icy_board::state::ppl_error::ERR_FORMAT;

        let mut state = state().await;
        let registry = crate::parser::icy_board_registry();
        let directory = tempfile::tempdir().unwrap();
        let file = directory.path().join("record.dat");
        let bytes = b"1234\n5678\n";
        for field in [
            RecordField::scalar(VariableType::UserData(crate::parser::CONTACT_ID as u32)),
            RecordField::scalar(VariableType::UserData(30)),
            RecordField {
                dim: 1,
                is_dynamic: true,
                ..RecordField::scalar(VariableType::Integer)
            },
            RecordField {
                dim: 2,
                is_dynamic: true,
                ..RecordField::scalar(VariableType::Integer)
            },
            RecordField {
                dim: 3,
                is_dynamic: true,
                ..RecordField::scalar(VariableType::Integer)
            },
        ] {
            for type_id in [100, 101] {
                for operation in ["FGETREC", "FPUTREC", "FREADREC", "FWRITEREC"] {
                    std::fs::write(&file, bytes).unwrap();
                    let mut io = DiskIO::new(".", None);
                    io.fopen(1, file.to_str().unwrap(), 2, 0).unwrap();
                    assert!(!io.ferr(1));
                    let mut vm = VirtualMachine::new("s1-io.ppe".into(), &registry, &mut io, &mut state);
                    vm.user_types = vec![vec![field], vec![RecordField::scalar(VariableType::UserData(100))]];
                    vm.variable_table.fill_in_records(&vm.user_types);
                    let value = create_record_value(type_id, &vm.user_types, &vm.variable_table.enums).unwrap();
                    let target = s1_push(&mut vm, value.clone());
                    let channel = s1_push(&mut vm, VariableValue::new_int(1));
                    let args = [channel, target.clone()];
                    match operation {
                        "FGETREC" => statements::predefined_procedures::fgetrec(&mut vm, &args).await.unwrap(),
                        "FPUTREC" => statements::predefined_procedures::fputrec(&mut vm, &args).await.unwrap(),
                        "FREADREC" => statements::predefined_procedures::freadrec(&mut vm, &args).await.unwrap(),
                        "FWRITEREC" => statements::predefined_procedures::fwriterec(&mut vm, &args).await.unwrap(),
                        _ => unreachable!(),
                    }
                    assert_eq!(ERR_FORMAT, vm.last_error.code, "{operation}, {field:?}");
                    assert!(vm.last_error.message.contains("cannot be stored"), "{operation}");
                    assert!(vm.io.ferr(1), "{operation}");
                    assert_eq!(0, vm.io.ftell(1).unwrap(), "{operation}");
                    let after = vm.eval_expr(&target).await.unwrap();
                    assert_eq!(value.vtype, after.vtype, "{operation}");
                    let (GenericVariableData::Record(before), GenericVariableData::Record(after)) = (&value.generic_data, &after.generic_data) else {
                        panic!("record expected")
                    };
                    // Host values are not structurally comparable; rejected I/O must retain the original storage.
                    assert!(Arc::ptr_eq(before, after), "{operation}");
                    assert_eq!(bytes.as_slice(), std::fs::read(&file).unwrap(), "{operation}");
                    vm.io.fclose(1).unwrap();
                }
            }
        }
    }

    #[tokio::test]
    async fn foreach_bytecode_checks_rank_nominal_type_and_enum_representation() {
        let mut state = state().await;
        let registry = crate::parser::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        for invalid in ["scalar source", "array target", "nominal target", "invalid enum"] {
            let executable = tests::compile("ENUM Bits\n One=1\nENDENUM\nBits values[0]\nBits item\nFOREACH item IN values\nPRINT item\nNEXT\n");
            let script = PPEScript::from_ppe_file(&executable).unwrap();
            let command = script
                .statements
                .iter()
                .find_map(|statement| match &statement.command {
                    PPECommand::ForEach(_, _, _) => Some(statement.command.clone()),
                    _ => None,
                })
                .unwrap();
            let PPECommand::ForEach(target, collection, _) = &command else {
                unreachable!()
            };
            let PPEExpr::Value(source) = collection.as_ref() else {
                panic!("unexpected FOREACH lowering")
            };
            let mut vm = VirtualMachine::new("foreach.ppe".into(), &registry, &mut io, &mut state);
            vm.variable_table = executable.variable_table;
            match invalid {
                "scalar source" => *vm.variable_table.get_value_mut(*source) = vm.variable_table.get_value(*source).get_array_value(0, 0, 0),
                "array target" => vm.variable_table.get_var_entry_mut(*target).header.dim = 1,
                "nominal target" => vm.variable_table.get_var_entry_mut(*target).header.variable_type = VariableType::Integer,
                "invalid enum" => {
                    let element = vm.variable_table.get_value_mut(*source).get_array_value_mut(0, 0, 0).unwrap();
                    *element = VariableValue::new_string("99".to_string());
                }
                _ => unreachable!(),
            }
            let before = vm.variable_table.get_value(*target).clone();
            let error = vm.execute_statement(&command).await.unwrap_err().to_string();
            assert!(
                error.contains("FOREACH") || error.contains("assign") || error.contains("enum type"),
                "{invalid}: {error}"
            );
            assert_eq!(before, *vm.variable_table.get_value(*target), "{invalid}");
            assert!(vm.foreach_stack.is_empty());
        }
    }

    #[tokio::test]
    async fn nested_fixed_field_failures_leave_root_and_shared_copies_unchanged() {
        let mut state = state().await;
        let registry = crate::parser::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        for (rank, bounds) in [(1, "1"), (2, "1,1"), (3, "1,1,1")] {
            for assignment in [
                "box.child.items=a",
                "box.child=Inner {items=a}",
                "box=Outer {child=Inner {items=a}}",
                "boxes[0].child.items=a",
                "box.children[0].items=a",
            ] {
                let source = format!(
                    "TYPE Inner\n INTEGER items[{bounds}]\nENDTYPE\nTYPE Outer\n INTEGER marker\n Inner child\n Inner children[0]\nENDTYPE\nOuter box\nOuter boxes[0]\nINTEGER a[{bounds}]\n{assignment}\n"
                );
                let executable = tests::compile(&source);
                let script = PPEScript::from_ppe_file(&executable).unwrap();
                let command = script
                    .statements
                    .iter()
                    .rev()
                    .find_map(|statement| match &statement.command {
                        PPECommand::Let(_, _) => Some(statement.command.clone()),
                        _ => None,
                    })
                    .unwrap();
                let mut vm = VirtualMachine::new("atomic.ppe".into(), &registry, &mut io, &mut state);
                vm.variable_table = executable.variable_table;
                vm.user_types = executable.user_types;
                let source_id = vm
                    .variable_table
                    .get_entries()
                    .iter()
                    .find(|entry| entry.header.variable_type == VariableType::Integer && entry.header.dim == rank)
                    .unwrap()
                    .header
                    .id as usize;
                vm.variable_table.get_value_mut(source_id).redim(rank, 2, 2, 2);
                let before: Vec<_> = vm.variable_table.get_entries().iter().map(|entry| entry.value.clone()).collect();
                let error = vm.execute_statement(&command).await.unwrap_err().to_string();
                assert!(error.contains("fixed shape"), "{assignment}: {error}");
                let after: Vec<_> = vm.variable_table.get_entries().iter().map(|entry| entry.value.clone()).collect();
                assert_eq!(before, after, "rank={rank}, {assignment}");
            }
        }
    }
}
