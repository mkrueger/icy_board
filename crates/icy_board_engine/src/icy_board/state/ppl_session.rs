use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    icy_board::{file_directory::FileDirectory, message_area::MessageArea},
    parser::{CONFERENCE_ID, FILE_DIRECTORY_ID, MESSAGE_AREA_ID, SESSION_ID, USER_ID},
};

macro_rules! member_name {
    ($name:ident, $value:literal) => {
        static $name: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new($value.to_string()));
    };
}

member_name!(CONFERENCE, "Conference");
member_name!(USER, "User");
member_name!(AREA, "Area");
member_name!(DIRECTORY, "Directory");
member_name!(USER_NAME, "UserName");
member_name!(ALIAS_NAME, "AliasName");
member_name!(SECURITY_LEVEL, "SecurityLevel");
member_name!(NODE, "Node");
member_name!(MINUTES_LEFT, "MinutesLeft");
member_name!(PAGE_LENGTH, "PageLength");
member_name!(LANGUAGE, "Language");
member_name!(IS_LOCAL, "IsLocal");
member_name!(IS_SYSOP, "IsSysop");

/// This call, as it stands right now. Unlike `Board` it is read live, so a
/// value kept in a variable still answers with what the session became.
#[derive(Clone, Copy, Debug, Default)]
pub struct PplSession;

impl PplSession {
    pub fn value() -> VariableValue {
        user_data_value(PplSession, SESSION_ID)
    }
}

impl UserData for PplSession {
    const TYPE_NAME: &'static str = "Session";
    const INSTANCE_PROVIDER: Option<crate::executable::FuncOpCode> = Some(crate::executable::FuncOpCode::Session);

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_property(CONFERENCE.clone(), VariableType::UserData(CONFERENCE_ID as u8), false);
        registry.add_property(USER.clone(), VariableType::UserData(USER_ID as u8), false);
        registry.add_property(AREA.clone(), VariableType::UserData(MESSAGE_AREA_ID as u8), false);
        registry.add_property(DIRECTORY.clone(), VariableType::UserData(FILE_DIRECTORY_ID as u8), false);
        for name in [&*USER_NAME, &*ALIAS_NAME, &*LANGUAGE] {
            registry.add_property(name.clone(), VariableType::UnboundedString, false);
        }
        for name in [&*SECURITY_LEVEL, &*NODE, &*MINUTES_LEFT, &*PAGE_LENGTH] {
            registry.add_property(name.clone(), VariableType::Integer, false);
        }
        for name in [&*IS_LOCAL, &*IS_SYSOP] {
            registry.add_property(name.clone(), VariableType::Boolean, false);
        }
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplSession {
    fn get_property_value(&self, vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        let session = &vm.icy_board_state.session;
        let value = if *name == *CONFERENCE {
            let mut conference = session.current_conference.clone();
            conference.number = session.current_conference_number as usize;
            user_data_value(conference, CONFERENCE_ID)
        } else if *name == *USER {
            super::ppl_user::PplUser::value()
        } else if *name == *AREA {
            let area = session
                .current_conference
                .areas
                .as_ref()
                .and_then(|areas| areas.get(session.current_message_area).cloned());
            let mut area = area.unwrap_or_else(MessageArea::default);
            area.number = session.current_message_area;
            area.valid = session
                .current_conference
                .areas
                .as_ref()
                .is_some_and(|areas| areas.get(session.current_message_area).is_some());
            user_data_value(area, MESSAGE_AREA_ID)
        } else if *name == *DIRECTORY {
            let directory = session
                .current_conference
                .directories
                .as_ref()
                .and_then(|directories| directories.get(session.current_file_directory).cloned());
            let mut directory = directory.unwrap_or_else(FileDirectory::default);
            directory.number = session.current_file_directory;
            directory.valid = session
                .current_conference
                .directories
                .as_ref()
                .is_some_and(|directories| directories.get(session.current_file_directory).is_some());
            user_data_value(directory, FILE_DIRECTORY_ID)
        } else if *name == *USER_NAME {
            VariableValue::new_unbounded_string(session.user_name.clone())
        } else if *name == *ALIAS_NAME {
            VariableValue::new_unbounded_string(session.alias_name.clone())
        } else if *name == *SECURITY_LEVEL {
            VariableValue::new_int(i32::from(session.cur_security))
        } else if *name == *NODE {
            // The same one-based node number `PCBNODE()` reports.
            VariableValue::new_int(vm.icy_board_state.node as i32 + 1)
        } else if *name == *MINUTES_LEFT {
            VariableValue::new_int(session.minutes_left())
        } else if *name == *PAGE_LENGTH {
            VariableValue::new_int(i32::from(session.page_len))
        } else if *name == *LANGUAGE {
            VariableValue::new_unbounded_string(session.language.clone())
        } else if *name == *IS_LOCAL {
            VariableValue::new_bool(session.is_local)
        } else if *name == *IS_SYSOP {
            VariableValue::new_bool(session.is_sysop)
        } else {
            return Err(format!("Unknown SESSION property {name}").into());
        };
        Ok(value)
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err(format!("SESSION property {name} is read-only").into())
    }

    async fn call_function(
        &self,
        _vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        _arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        Err(format!("Unknown SESSION function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown SESSION method {name}").into())
    }
}
