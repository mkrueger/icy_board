use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    datetime::IcbDate,
    executable::{VariableType, VariableValue},
    icy_board::user_base::{FSEMode, User, UserContact},
    parser::{CONTACT_ID, USER_ID},
};

macro_rules! member_name {
    ($name:ident, $value:literal) => {
        static $name: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new($value.to_string()));
    };
}

member_name!(NAME, "Name");
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
member_name!(NOTE_COUNT, "NoteCount");
member_name!(GET_NOTE, "GetNote");

member_name!(EXPERT_MODE, "ExpertMode");
member_name!(FULL_SCREEN_EDITOR, "FullScreenEditor");
member_name!(ASK_FOR_EDITOR, "AskForEditor");
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

member_name!(CONTACT_COUNT, "ContactCount");
member_name!(GET_CONTACT, "GetContact");
member_name!(SET_CONTACT, "SetContact");
member_name!(DELETE_CONTACT, "DeleteContact");

/// How many sysop notes a user carries. `PCBoard` called them `U_NOTES`.
const NOTE_COUNT_VALUE: i32 = 5;

/// The caller's own record. It is read live from the session, so a value kept in a
/// variable still answers with what the user became.
#[derive(Clone, Copy, Debug, Default)]
pub struct PplUser;

impl PplUser {
    pub fn value() -> VariableValue {
        user_data_value(PplUser, USER_ID)
    }
}

/// A `CONTACT` record: the service a user can be reached on, and the account.
fn contact_value(contact: &UserContact) -> VariableValue {
    VariableValue {
        vtype: VariableType::UserData(CONTACT_ID as u8),
        data: crate::executable::VariableData::default(),
        generic_data: crate::executable::GenericVariableData::Record(vec![
            VariableValue::new_string(contact.service.clone()),
            VariableValue::new_string(contact.account.clone()),
        ]),
    }
}

/// Service names are compared without regard to case and stored lowercase, so a
/// PPE cannot end up with two entries that mean the same service.
fn normalize_service(service: &str) -> String {
    service.trim().to_ascii_lowercase()
}

impl UserData for PplUser {
    const TYPE_NAME: &'static str = "User";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        for name in [
            &*NAME,
            &*ALIAS,
            &*VERIFY_ANSWER,
            &*STREET1,
            &*STREET2,
            &*CITY,
            &*STATE,
            &*ZIP,
            &*COUNTRY,
            &*BUSINESS_PHONE,
            &*HOME_PHONE,
            &*EMAIL,
            &*WEB,
            &*GENDER,
            &*COMMENT,
            &*SYSOP_COMMENT,
            &*PROTOCOL,
            &*LANGUAGE,
            &*DATE_FORMAT,
        ] {
            registry.add_property(name.clone(), VariableType::String, false);
        }
        for name in [&*BIRTH_DATE, &*EXPIRATION_DATE, &*FIRST_DATE_ON, &*LAST_DATE_ON, &*LAST_DIR_READ] {
            registry.add_property(name.clone(), VariableType::Date, false);
        }
        for name in [
            &*NOTE_COUNT,
            &*PAGE_LENGTH,
            &*SECURITY_LEVEL,
            &*EXPIRED_SECURITY_LEVEL,
            &*TIMES_ON,
            &*MESSAGES_READ,
            &*MESSAGES_LEFT,
            &*UPLOADS,
            &*DOWNLOADS,
            &*MINUTES_TODAY,
            &*CONTACT_COUNT,
        ] {
            registry.add_property(name.clone(), VariableType::Integer, false);
        }
        for name in [&*UPLOAD_BYTES, &*DOWNLOAD_BYTES, &*DOWNLOAD_BYTES_TODAY] {
            registry.add_property(name.clone(), VariableType::Unsigned, false);
        }
        for name in [
            &*EXPERT_MODE,
            &*FULL_SCREEN_EDITOR,
            &*ASK_FOR_EDITOR,
            &*CLEAR_SCREEN,
            &*SCROLL_MESSAGE_BODY,
            &*SHORT_DESCRIPTIONS,
            &*LONG_HEADER,
            &*WIDE_EDITOR,
            &*USE_GRAPHICS,
            &*USE_ALIAS,
        ] {
            registry.add_property(name.clone(), VariableType::Boolean, false);
        }

        registry.add_function(GET_NOTE.clone(), vec![VariableType::Integer], VariableType::String);
        registry.add_function(GET_CONTACT.clone(), vec![VariableType::Integer], VariableType::UserData(CONTACT_ID as u8));
        registry.add_function(SET_CONTACT.clone(), vec![VariableType::String, VariableType::String], VariableType::Boolean);
        registry.add_function(DELETE_CONTACT.clone(), vec![VariableType::String], VariableType::Boolean);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplUser {
    fn get_property_value(&self, vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        // Nobody logged in reads as an empty user, so a member keeps its declared type.
        let no_user;
        let user = match vm.icy_board_state.session.current_user.as_ref() {
            Some(user) => user,
            None => {
                no_user = User::default();
                &no_user
            }
        };
        let string = |value: &str| VariableValue::new_string(value.to_string());
        let date = |value: &chrono::DateTime<chrono::Utc>| VariableValue::new_date(IcbDate::from_utc(value).to_pcboard_date());

        let value = if *name == *NAME {
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
        } else if *name == *FIRST_DATE_ON {
            date(&user.stats.first_date_on)
        } else if *name == *LAST_DATE_ON {
            date(&user.stats.last_on)
        } else if *name == *LAST_DIR_READ {
            date(&user.date_last_dir_read)
        } else if *name == *NOTE_COUNT {
            VariableValue::new_int(NOTE_COUNT_VALUE)
        } else if *name == *PAGE_LENGTH {
            VariableValue::new_int(i32::from(user.page_len))
        } else if *name == *SECURITY_LEVEL {
            VariableValue::new_int(i32::from(user.security_level))
        } else if *name == *EXPIRED_SECURITY_LEVEL {
            VariableValue::new_int(i32::from(user.exp_security_level))
        } else if *name == *TIMES_ON {
            VariableValue::new_int(user.stats.num_times_on as i32)
        } else if *name == *MESSAGES_READ {
            VariableValue::new_int(user.stats.messages_read as i32)
        } else if *name == *MESSAGES_LEFT {
            VariableValue::new_int(user.stats.messages_left as i32)
        } else if *name == *UPLOADS {
            VariableValue::new_int(user.stats.num_uploads as i32)
        } else if *name == *DOWNLOADS {
            VariableValue::new_int(user.stats.num_downloads as i32)
        } else if *name == *MINUTES_TODAY {
            VariableValue::new_int(i32::from(user.stats.minutes_today))
        } else if *name == *CONTACT_COUNT {
            VariableValue::new_int(user.contacts.len() as i32)
        } else if *name == *UPLOAD_BYTES {
            VariableValue::new_unsigned(user.stats.total_upld_bytes)
        } else if *name == *DOWNLOAD_BYTES {
            VariableValue::new_unsigned(user.stats.total_dnld_bytes)
        } else if *name == *DOWNLOAD_BYTES_TODAY {
            VariableValue::new_unsigned(user.stats.today_dnld_bytes.max(0) as u64)
        } else if *name == *EXPERT_MODE {
            VariableValue::new_bool(user.flags.expert_mode)
        } else if *name == *FULL_SCREEN_EDITOR {
            VariableValue::new_bool(user.flags.fse_mode == FSEMode::Yes)
        } else if *name == *ASK_FOR_EDITOR {
            VariableValue::new_bool(user.flags.fse_mode == FSEMode::Ask)
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

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err(format!("USER property {name} is read-only").into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if *name == *GET_NOTE {
            let index = arguments[0].as_int();
            let Some(user) = vm.icy_board_state.session.current_user.as_ref() else {
                return Ok(VariableValue::new_string(String::new()));
            };
            let note = match index {
                0 => &user.custom_comment1,
                1 => &user.custom_comment2,
                2 => &user.custom_comment3,
                3 => &user.custom_comment4,
                4 => &user.custom_comment5,
                _ => return Ok(VariableValue::new_string(String::new())),
            };
            return Ok(VariableValue::new_string(note.clone()));
        }
        if *name == *GET_CONTACT {
            let index = arguments[0].as_int();
            let contact = vm
                .icy_board_state
                .session
                .current_user
                .as_ref()
                .filter(|_| index >= 0)
                .and_then(|user| user.contacts.get(index as usize))
                .cloned()
                .unwrap_or_default();
            return Ok(contact_value(&contact));
        }
        if *name == *SET_CONTACT {
            let service = normalize_service(&arguments[0].as_string());
            let account = arguments[1].as_string().trim().to_string();
            if service.is_empty() || account.is_empty() {
                return Ok(VariableValue::new_bool(false));
            }
            let Some(user) = vm.icy_board_state.session.current_user.as_mut() else {
                return Ok(VariableValue::new_bool(false));
            };
            if let Some(contact) = user.contacts.iter_mut().find(|contact| contact.service == service) {
                contact.account = account;
            } else {
                user.contacts.push(UserContact { service, account });
            }
            user.flags.is_dirty = true;
            return Ok(VariableValue::new_bool(true));
        }
        if *name == *DELETE_CONTACT {
            let service = normalize_service(&arguments[0].as_string());
            let Some(user) = vm.icy_board_state.session.current_user.as_mut() else {
                return Ok(VariableValue::new_bool(false));
            };
            let before = user.contacts.len();
            user.contacts.retain(|contact| contact.service != service);
            let removed = user.contacts.len() != before;
            if removed {
                user.flags.is_dirty = true;
            }
            return Ok(VariableValue::new_bool(removed));
        }
        Err(format!("Unknown USER function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown USER method {name}").into())
    }
}
