use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    datetime::IcbDate,
    executable::{VariableType, VariableValue},
    icy_board::{
        state::ppl_error::{ERR_INVALID, ERR_IO, ERR_KIND_USER, ERR_LIMIT, ERR_UNAVAILABLE, PplError},
        user_base::{FSEMode, MAX_CONTACTS, User, UserContact},
    },
    parser::{CONTACT_ID, USER_ID},
};

macro_rules! member_name {
    ($name:ident, $value:literal) => {
        static $name: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new($value.to_string()));
    };
}

member_name!(NAME, "Name");
member_name!(VALID, "Valid");
member_name!(RECORD_NUMBER, "RecordNumber");
member_name!(ALIAS, "Alias");
member_name!(VERIFY_ANSWER, "VerifyAnswer");

member_name!(STREET1, "Street1");
member_name!(STREET2, "Street2");
member_name!(CITY, "City");
member_name!(STATE, "State");
member_name!(ZIP, "Zip");
member_name!(COUNTRY, "Country");

member_name!(BUSINESS_PHONE, "BusinessPhone");
member_name!(HOME_PHONE, "HomePhone");
member_name!(EMAIL, "Email");
member_name!(WEB, "Web");
member_name!(GENDER, "Gender");
member_name!(BIRTH_DATE, "BirthDate");

member_name!(COMMENT, "Comment");
member_name!(SYSOP_COMMENT, "SysopComment");
member_name!(NOTES, "Notes");

member_name!(EXPERT_MODE, "ExpertMode");
member_name!(EDITOR_MODE, "EditorMode");
member_name!(CLEAR_SCREEN, "ClearScreen");
member_name!(SCROLL_MESSAGE_BODY, "ScrollMessageBody");
member_name!(SHORT_DESCRIPTIONS, "ShortDescriptions");
member_name!(LONG_HEADER, "LongHeader");
member_name!(WIDE_EDITOR, "WideEditor");
member_name!(USE_GRAPHICS, "UseGraphics");
member_name!(USE_ALIAS, "UseAlias");
member_name!(PAGE_LENGTH, "PageLength");
member_name!(PROTOCOL, "Protocol");
member_name!(LANGUAGE, "Language");
member_name!(DATE_FORMAT, "DateFormat");

member_name!(SECURITY_LEVEL, "SecurityLevel");
member_name!(EXPIRED_SECURITY_LEVEL, "ExpiredSecurityLevel");
member_name!(EXPIRATION_DATE, "ExpirationDate");
member_name!(PASSWORD_EXPIRES, "PasswordExpires");
member_name!(SET_PASSWORD, "SetPassword");

member_name!(TIMES_ON, "TimesOn");
member_name!(FIRST_DATE_ON, "FirstDateOn");
member_name!(LAST_DATE_ON, "LastDateOn");
member_name!(LAST_DIR_READ, "LastDirRead");
member_name!(MESSAGES_READ, "MessagesRead");
member_name!(MESSAGES_LEFT, "MessagesLeft");
member_name!(UPLOADS, "Uploads");
member_name!(DOWNLOADS, "Downloads");
member_name!(UPLOAD_BYTES, "UploadBytes");
member_name!(DOWNLOAD_BYTES, "DownloadBytes");
member_name!(DOWNLOAD_BYTES_TODAY, "DownloadBytesToday");
member_name!(MINUTES_TODAY, "MinutesToday");

member_name!(CONTACTS, "Contacts");
member_name!(ADD_CONTACT, "AddContact");
member_name!(REMOVE_CONTACT, "RemoveContact");
member_name!(SET_NOTE, "SetNote");

/// A user record. `Session.User` is live; entries from `Board.Users` are snapshots.
#[derive(Clone, Default)]
pub enum PplUser {
    #[default]
    Current,
    Snapshot {
        user: std::sync::Arc<User>,
        valid: bool,
        index: usize,
    },
}

impl PplUser {
    pub fn value() -> VariableValue {
        user_data_value(PplUser::Current, USER_ID)
    }

    fn snapshot(user: std::sync::Arc<User>, valid: bool, index: usize) -> Self {
        Self::Snapshot { user, valid, index }
    }

    fn user<'a>(&'a self, vm: &'a crate::vm::VirtualMachine) -> Option<&'a User> {
        match self {
            Self::Current => vm.icy_board_state.session.current_user.as_ref(),
            Self::Snapshot { user, .. } => Some(user),
        }
    }

    fn valid(&self, vm: &crate::vm::VirtualMachine) -> bool {
        match self {
            Self::Current => vm.icy_board_state.session.current_user.is_some(),
            Self::Snapshot { valid, .. } => *valid,
        }
    }

    /// Stage mutations on a copy so rejected arguments cannot alter the live user.
    fn writable_user(&self, vm: &mut crate::vm::VirtualMachine<'_>) -> Option<User> {
        if matches!(self, Self::Snapshot { .. }) {
            vm.set_error(PplError::new(ERR_KIND_USER, ERR_INVALID, "Board.Users entries are read-only"));
            return None;
        }
        let user = vm.icy_board_state.session.current_user.clone();
        if user.is_none() {
            vm.set_error(PplError::new(ERR_KIND_USER, ERR_UNAVAILABLE, "no current user"));
        }
        user
    }

    async fn save_user(vm: &mut crate::vm::VirtualMachine<'_>, mut user: User) -> bool {
        user.flags.is_dirty = true;
        let previous = vm.icy_board_state.session.current_user.replace(user);
        match vm.icy_board_state.persist_current_user().await {
            Ok(()) => {
                vm.operation_succeeded();
                true
            }
            Err(error) => {
                // persist_current_user also restores the board's in-memory entry.
                vm.icy_board_state.session.current_user = previous;
                vm.set_error(PplError::new(ERR_KIND_USER, ERR_IO, format!("failed to save user: {error}")));
                false
            }
        }
    }
}

/// A `CONTACT` record: the service a user can be reached on, and the account.
fn contact_value(contact: &UserContact) -> VariableValue {
    VariableValue {
        vtype: VariableType::UserData(CONTACT_ID as u8),
        data: crate::executable::VariableData::default(),
        generic_data: crate::executable::GenericVariableData::Record(std::sync::Arc::new(vec![
            VariableValue::new_unbounded_string(contact.service.clone()),
            VariableValue::new_unbounded_string(contact.account.clone()),
        ])),
    }
}

/// Service names are compared without regard to case and stored lowercase, so a
/// PPE cannot end up with two entries that mean the same service.
fn normalize_service(service: &str) -> String {
    service.trim().to_ascii_lowercase()
}

pub fn user_array_value(users: &[User]) -> VariableValue {
    VariableValue::new_vector(
        VariableType::UserData(USER_ID as u8),
        users
            .iter()
            .cloned()
            .enumerate()
            .map(|(index, user)| user_data_value(PplUser::snapshot(std::sync::Arc::new(user), true, index), USER_ID))
            .collect(),
    )
}

/// The `EditorMode` enum values, which follow `FSEMode`'s own order.
fn editor_mode_to_int(mode: &FSEMode) -> i32 {
    match mode {
        FSEMode::Yes => 0,
        FSEMode::No => 1,
        FSEMode::Ask => 2,
    }
}

fn editor_mode_from_int(value: i32) -> FSEMode {
    match value {
        1 => FSEMode::No,
        2 => FSEMode::Ask,
        _ => FSEMode::Yes,
    }
}

impl UserData for PplUser {
    const TYPE_NAME: &'static str = "User";
    const EMPTY_VALUE: Option<fn() -> VariableValue> = Some(|| user_data_value(PplUser::snapshot(std::sync::Arc::new(User::default()), false, 0), USER_ID));

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        crate::parser::board_catalog::register_members(USER_ID, registry);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplUser {
    fn get_property_value(&self, vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        // Nobody logged in reads as an empty user, so a member keeps its declared type.
        let no_user;
        let user = match self.user(vm) {
            Some(user) => user,
            None => {
                no_user = User::default();
                &no_user
            }
        };
        let string = |value: &str| VariableValue::new_unbounded_string(value.to_string());
        let date = |value: &chrono::DateTime<chrono::Utc>| VariableValue::new_date(IcbDate::from_utc(value).to_pcboard_date());

        let value = if *name == *VALID {
            VariableValue::new_bool(self.valid(vm))
        } else if *name == *RECORD_NUMBER {
            let number = match self {
                Self::Current => vm.icy_board_state.session.cur_user_id + 1,
                Self::Snapshot { index, .. } => *index as i32 + 1,
            };
            VariableValue::new_int(number.max(0))
        } else if *name == *NAME {
            string(&user.name)
        } else if *name == *ALIAS {
            string(&user.alias)
        } else if *name == *VERIFY_ANSWER {
            string(&user.verify_answer)
        } else if *name == *STREET1 {
            string(&user.street1)
        } else if *name == *STREET2 {
            string(&user.street2)
        } else if *name == *CITY {
            string(&user.city_or_state)
        } else if *name == *STATE {
            string(&user.state)
        } else if *name == *ZIP {
            string(&user.zip)
        } else if *name == *COUNTRY {
            string(&user.country)
        } else if *name == *BUSINESS_PHONE {
            string(&user.bus_data_phone)
        } else if *name == *HOME_PHONE {
            string(&user.home_voice_phone)
        } else if *name == *EMAIL {
            string(&user.email)
        } else if *name == *WEB {
            string(&user.web)
        } else if *name == *GENDER {
            string(&user.gender)
        } else if *name == *COMMENT {
            string(&user.user_comment)
        } else if *name == *SYSOP_COMMENT {
            string(&user.sysop_comment)
        } else if *name == *PROTOCOL {
            string(&user.protocol)
        } else if *name == *LANGUAGE {
            string(&user.language)
        } else if *name == *DATE_FORMAT {
            string(&user.date_format)
        } else if *name == *BIRTH_DATE {
            date(&user.birth_date)
        } else if *name == *EXPIRATION_DATE {
            date(&user.expiration_date)
        } else if *name == *PASSWORD_EXPIRES {
            date(&user.password.expire_date)
        } else if *name == *FIRST_DATE_ON {
            date(&user.stats.first_date_on)
        } else if *name == *LAST_DATE_ON {
            date(&user.stats.last_on)
        } else if *name == *LAST_DIR_READ {
            date(&user.date_last_dir_read)
        } else if *name == *NOTES {
            VariableValue::new_vector(
                VariableType::UnboundedString,
                [
                    &user.custom_comment1,
                    &user.custom_comment2,
                    &user.custom_comment3,
                    &user.custom_comment4,
                    &user.custom_comment5,
                ]
                .into_iter()
                .map(|note| VariableValue::new_unbounded_string(note.clone()))
                .collect(),
            )
        } else if *name == *PAGE_LENGTH {
            VariableValue::new_int(i32::from(user.page_len))
        } else if *name == *SECURITY_LEVEL {
            VariableValue::new_int(i32::from(user.security_level))
        } else if *name == *EXPIRED_SECURITY_LEVEL {
            VariableValue::new_int(i32::from(user.exp_security_level))
        } else if *name == *TIMES_ON {
            VariableValue::new_ulong(user.stats.num_times_on)
        } else if *name == *MESSAGES_READ {
            VariableValue::new_ulong(user.stats.messages_read)
        } else if *name == *MESSAGES_LEFT {
            VariableValue::new_ulong(user.stats.messages_left)
        } else if *name == *UPLOADS {
            VariableValue::new_ulong(user.stats.num_uploads)
        } else if *name == *DOWNLOADS {
            VariableValue::new_ulong(user.stats.num_downloads)
        } else if *name == *MINUTES_TODAY {
            VariableValue::new_int(i32::from(user.stats.minutes_today))
        } else if *name == *CONTACTS {
            VariableValue::new_vector(VariableType::UserData(CONTACT_ID as u8), user.contacts.iter().map(contact_value).collect())
        } else if *name == *UPLOAD_BYTES {
            VariableValue::new_ulong(user.stats.total_upld_bytes)
        } else if *name == *DOWNLOAD_BYTES {
            VariableValue::new_ulong(user.stats.total_dnld_bytes)
        } else if *name == *DOWNLOAD_BYTES_TODAY {
            VariableValue::new_ulong(user.stats.today_dnld_bytes.max(0) as u64)
        } else if *name == *EXPERT_MODE {
            VariableValue::new_bool(user.flags.expert_mode)
        } else if *name == *EDITOR_MODE {
            VariableValue::new_int(editor_mode_to_int(&user.flags.fse_mode))
        } else if *name == *CLEAR_SCREEN {
            VariableValue::new_bool(user.flags.msg_clear)
        } else if *name == *SCROLL_MESSAGE_BODY {
            VariableValue::new_bool(user.flags.scroll_msg_body)
        } else if *name == *SHORT_DESCRIPTIONS {
            VariableValue::new_bool(user.flags.use_short_filedescr)
        } else if *name == *LONG_HEADER {
            VariableValue::new_bool(user.flags.long_msg_header)
        } else if *name == *WIDE_EDITOR {
            VariableValue::new_bool(user.flags.wide_editor)
        } else if *name == *USE_GRAPHICS {
            VariableValue::new_bool(user.flags.use_graphics)
        } else if *name == *USE_ALIAS {
            VariableValue::new_bool(user.flags.use_alias)
        } else {
            return Err(format!("Unknown USER property {name}").into());
        };
        Ok(value)
    }

    async fn set_property_value(&self, vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, val: VariableValue) -> crate::Res<()> {
        let Some(mut user) = self.writable_user(vm) else {
            return Ok(());
        };
        let number = val.as_int();
        let invalid_range = if *name == *PAGE_LENGTH && u16::try_from(number).is_err() {
            Some("PageLength must be between 0 and 65535")
        } else if (*name == *SECURITY_LEVEL || *name == *EXPIRED_SECURITY_LEVEL) && u8::try_from(number).is_err() {
            Some("security levels must be between 0 and 255")
        } else if *name == *EDITOR_MODE && !(0..=2).contains(&number) {
            Some("unsupported EditorMode value")
        } else {
            None
        };
        if let Some(message) = invalid_range {
            vm.set_error(PplError::new(ERR_KIND_USER, ERR_INVALID, message));
            return Ok(());
        }
        let text = || val.as_string();
        let date = || IcbDate::from_pcboard(val.as_int() as u32).to_utc_date_time();

        if *name == *ALIAS {
            user.alias = text();
        } else if *name == *VERIFY_ANSWER {
            user.verify_answer = text();
        } else if *name == *STREET1 {
            user.street1 = text();
        } else if *name == *STREET2 {
            user.street2 = text();
        } else if *name == *CITY {
            user.city_or_state = text();
        } else if *name == *STATE {
            user.state = text();
        } else if *name == *ZIP {
            user.zip = text();
        } else if *name == *COUNTRY {
            user.country = text();
        } else if *name == *BUSINESS_PHONE {
            user.bus_data_phone = text();
        } else if *name == *HOME_PHONE {
            user.home_voice_phone = text();
        } else if *name == *EMAIL {
            user.email = text();
        } else if *name == *WEB {
            user.web = text();
        } else if *name == *GENDER {
            user.gender = text();
        } else if *name == *COMMENT {
            user.user_comment = text();
        } else if *name == *SYSOP_COMMENT {
            user.sysop_comment = text();
        } else if *name == *PROTOCOL {
            user.protocol = text();
        } else if *name == *BIRTH_DATE {
            user.birth_date = date();
        } else if *name == *EXPIRATION_DATE {
            user.expiration_date = date();
        } else if *name == *PASSWORD_EXPIRES {
            user.password.expire_date = date();
        } else if *name == *PAGE_LENGTH {
            user.page_len = number as u16;
        } else if *name == *SECURITY_LEVEL {
            user.security_level = number as u8;
        } else if *name == *EXPIRED_SECURITY_LEVEL {
            user.exp_security_level = number as u8;
        } else if *name == *EXPERT_MODE {
            user.flags.expert_mode = val.as_bool();
        } else if *name == *EDITOR_MODE {
            user.flags.fse_mode = editor_mode_from_int(val.as_int());
        } else if *name == *CLEAR_SCREEN {
            user.flags.msg_clear = val.as_bool();
        } else if *name == *SCROLL_MESSAGE_BODY {
            user.flags.scroll_msg_body = val.as_bool();
        } else if *name == *SHORT_DESCRIPTIONS {
            user.flags.use_short_filedescr = val.as_bool();
        } else if *name == *LONG_HEADER {
            user.flags.long_msg_header = val.as_bool();
        } else if *name == *WIDE_EDITOR {
            user.flags.wide_editor = val.as_bool();
        } else {
            return Err(format!("USER property {name} is read-only").into());
        }
        Self::save_user(vm, user).await;
        Ok(())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if *name != *SET_PASSWORD && *name != *ADD_CONTACT && *name != *REMOVE_CONTACT && *name != *SET_NOTE {
            return Err(format!("Unknown USER function {name}").into());
        }
        let Some(mut user) = self.writable_user(vm) else {
            return Ok(VariableValue::new_bool(false));
        };
        if *name == *SET_PASSWORD {
            let plain = arguments[0].as_string();
            if plain.is_empty() {
                vm.set_error(PplError::new(ERR_KIND_USER, ERR_INVALID, "password cannot be empty"));
                return Ok(VariableValue::new_bool(false));
            }
            // Hashing depends on board configuration, so ask the board rather than
            // storing whatever the PPE handed us.
            let password = vm.icy_board_state.create_password(plain).await;
            user.password.password = password;
        } else if *name == *ADD_CONTACT {
            let service = normalize_service(&arguments[0].as_string());
            let account = arguments[1].as_string().trim().to_string();
            if service.is_empty() || account.is_empty() {
                vm.set_error(PplError::new(ERR_KIND_USER, ERR_INVALID, "contact service and account cannot be empty"));
                return Ok(VariableValue::new_bool(false));
            }
            if !user.add_contact(UserContact { service, account }) {
                vm.set_error(PplError::new(
                    ERR_KIND_USER,
                    ERR_LIMIT,
                    format!("a user may hold at most {MAX_CONTACTS} contacts"),
                ));
                return Ok(VariableValue::new_bool(false));
            }
        } else if *name == *REMOVE_CONTACT {
            let Ok(index) = usize::try_from(arguments[0].as_int()) else {
                vm.set_error(PplError::new(ERR_KIND_USER, ERR_INVALID, "contact index is out of range"));
                return Ok(VariableValue::new_bool(false));
            };
            if index >= user.contacts.len() {
                vm.set_error(PplError::new(ERR_KIND_USER, ERR_INVALID, "contact index is out of range"));
                return Ok(VariableValue::new_bool(false));
            }
            user.contacts.remove(index);
        } else if *name == *SET_NOTE {
            let index = arguments[0].as_int();
            let text = arguments[1].as_string();
            let note = match index {
                0 => &mut user.custom_comment1,
                1 => &mut user.custom_comment2,
                2 => &mut user.custom_comment3,
                3 => &mut user.custom_comment4,
                4 => &mut user.custom_comment5,
                _ => {
                    vm.set_error(PplError::new(ERR_KIND_USER, ERR_INVALID, "note index must be between 0 and 4"));
                    return Ok(VariableValue::new_bool(false));
                }
            };
            *note = text;
        }
        Ok(VariableValue::new_bool(Self::save_user(vm, user).await))
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown USER method {name}").into())
    }
}
