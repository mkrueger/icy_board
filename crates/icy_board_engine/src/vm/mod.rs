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
    pub write_back_stack: Vec<PPEExpr>,
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
            PPEExpr::Value(id) | PPEExpr::RoutineReference(id) => Some(decay_array(self.variable_table.get_value(*id).clone())),
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
            PPEExpr::Value(id) | PPEExpr::RoutineReference(id) => Ok(decay_array(self.variable_table.get_value(*id).clone())),
            PPEExpr::RecordLiteral(type_id, fields) => {
                let mut value = crate::executable::create_record_value(*type_id, &self.user_types).ok_or(VMError::InternalVMError)?;
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
                    values[*field_id] = field_value.convert_to(field_type);
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
                let (dim_1, dim_2, dim_3) = self.eval_array_indices(arguments).await?;
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
        let commands = Arc::clone(&self.commands);
        let max_ptr = commands.len();
        while !self.fpclear && self.is_running && self.cur_ptr < max_ptr {
            let p = self.cur_ptr;
            self.cur_ptr += 1;
            // log::info!("{p}: {c}");
            self.execute_statement(&commands[p]).await?;
            self.check_error_trap()?;
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
                    let fields = Arc::make_mut(fields);
                    target = match step {
                        ResolvedLValuePathStep::Member(member_id) => fields.get_mut(member_id).ok_or(VMError::InternalVMError)?,
                        ResolvedLValuePathStep::IndexedMember(member_id, first, second, third) => fields
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
                        let single_pass_value = if pass_flags.count_ones() == 1 {
                            let parameter = pass_flags.trailing_zeros() as usize;
                            (parameter < parameters).then(|| self.variable_table.get_value(first + parameter).clone())
                        } else {
                            None
                        };
                        let mut pass_values = Vec::new();
                        if pass_flags > 0 && single_pass_value.is_none() {
                            for i in 0..parameters {
                                if 1u16.checked_shl(i as u32).is_some_and(|mask| mask & pass_flags != 0) {
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
                                    *self.variable_table.get_value_mut(id) = value;
                                }
                            }
                        }

                        if let Some(value) = single_pass_value {
                            if let Some(argument_expr) = self.write_back_stack.pop() {
                                self.set_variable(&argument_expr, value).await?;
                            } else {
                                return Err(VMError::WriteBackStackEmpty.into());
                            }
                        } else if pass_flags > 0 {
                            for i in (0..parameters).rev() {
                                if 1u16.checked_shl(i as u32).is_some_and(|mask| mask & pass_flags != 0) {
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
                    self.cleanup_foreach_for_jump(*label);
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
                let whole_dynamic_array = match variable.as_ref() {
                    PPEExpr::Value(id) => {
                        self.variable_table.get_var_entry(*id).header.flags & crate::executable::variable_table::VARIABLE_FLAG_DYNAMIC_ARRAY != 0
                    }
                    _ => false,
                };
                let val = if whole_dynamic_array {
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
                let count = Self::foreach_element_count(&collection);
                if count == 0 {
                    self.goto(*end)?;
                } else {
                    let element = Self::foreach_element_at(&collection, 0);
                    self.variable_table.set_value(*variable, element);
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
                    self.variable_table.set_value(variable, element);
                    self.goto(*start)?;
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
            let reg: UserTypeRegistry = UserTypeRegistry::icy_board_registry();
            log::info!("Run PPE {}", file_name.display());

            let mut vm = VirtualMachine::new(file_name, &reg, io, icy_board_state);
            vm.script = script;
            vm.commands = commands;
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
