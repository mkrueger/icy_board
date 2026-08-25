use async_trait::async_trait;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::{VariableType, VariableValue},
    parser::{CONFERENCE_ID, SESSION_ID},
};

macro_rules! member_name {
    ($name:ident, $value:literal) => {
        static $name: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new($value.to_string()));
    };
}

member_name!(CONFERENCE, "Conference");
member_name!(CONFERENCE_NUMBER, "ConferenceNumber");
member_name!(MESSAGE_AREA, "MessageArea");
member_name!(FILE_DIRECTORY, "FileDirectory");
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
        for name in [&*USER_NAME, &*ALIAS_NAME, &*LANGUAGE] {
            registry.add_property(name.clone(), VariableType::String, false);
        }
        for name in [
            &*CONFERENCE_NUMBER,
            &*MESSAGE_AREA,
            &*FILE_DIRECTORY,
            &*SECURITY_LEVEL,
            &*NODE,
            &*MINUTES_LEFT,
            &*PAGE_LENGTH,
        ] {
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
            user_data_value(session.current_conference.clone(), CONFERENCE_ID)
        } else if *name == *CONFERENCE_NUMBER {
            VariableValue::new_int(i32::from(session.current_conference_number))
        } else if *name == *MESSAGE_AREA {
            VariableValue::new_int(session.current_message_area as i32)
        } else if *name == *FILE_DIRECTORY {
            VariableValue::new_int(session.current_file_directory as i32)
        } else if *name == *USER_NAME {
            VariableValue::new_string(session.user_name.clone())
        } else if *name == *ALIAS_NAME {
            VariableValue::new_string(session.alias_name.clone())
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
            VariableValue::new_string(session.language.clone())
        } else if *name == *IS_LOCAL {
            VariableValue::new_bool(session.is_local)
        } else if *name == *IS_SYSOP {
            VariableValue::new_bool(session.is_sysop)
        } else {
            return Err(format!("Unknown SESSION property {name}").into());
        };
        Ok(value)
    }

    fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, _name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err("SESSION properties are read-only".into())
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
