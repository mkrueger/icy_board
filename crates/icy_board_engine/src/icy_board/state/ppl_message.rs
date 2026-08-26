use std::path::PathBuf;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use jamjam::jam::{JamMessageBase, attributes as jam_attributes, msg_header::JamMessageHeader};

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    datetime::{IcbDate, IcbTime},
    executable::{VariableData, VariableType, VariableValue},
    icy_board::state::ppl_error::{ERR_FORMAT, ERR_IO, ERR_KIND_MSG, PplError},
    parser::MSG_ID,
    vm::expressions::predefined_functions::message_status,
};

pub fn message_is_missing(error: &jamjam::Error) -> bool {
    matches!(
        error,
        jamjam::Error::Jam(jamjam::jam::JamError::MessageNumberOutOfRange(..) | jamjam::jam::JamError::MessageDeleted)
    )
}

pub fn message_error(action: &str, path: &std::path::Path, error: &jamjam::Error) -> PplError {
    let code = if matches!(error, jamjam::Error::Io(_)) { ERR_IO } else { ERR_FORMAT };
    PplError::new(ERR_KIND_MSG, code, format!("{action} {}: {error}", path.display()))
}

macro_rules! member_name {
    ($name:ident, $value:literal) => {
        static $name: std::sync::LazyLock<unicase::Ascii<String>> = std::sync::LazyLock::new(|| unicase::Ascii::new($value.to_string()));
    };
}

member_name!(VALID, "Valid");
member_name!(NUMBER, "Number");
member_name!(FROM, "From");
member_name!(TO, "To");
member_name!(SUBJECT, "Subject");
member_name!(DATE, "Date");
member_name!(TIME, "Time");
member_name!(REPLY_TO, "ReplyTo");
member_name!(STATUS, "Status");
member_name!(IS_PRIVATE, "IsPrivate");
member_name!(IS_READ, "IsRead");
member_name!(IS_DELETED, "IsDeleted");
member_name!(IS_ECHO, "IsEcho");
member_name!(NEEDS_PASSWORD, "NeedsPassword");
member_name!(SIZE, "Size");
member_name!(TEXT, "Text");

/// One message, read out of its area. The header travels with the value; the body
/// stays in the base until `Text()` asks for it, so listing headers does not pay
/// for every message it walks past.
#[derive(Clone, Debug, Default)]
pub struct PplMessage {
    /// Where the body can be fetched from, empty for a message that is not there.
    path: PathBuf,
    valid: bool,
    number: u32,
    from: String,
    to: String,
    subject: String,
    written: i64,
    reply_to: u32,
    status: String,
    is_private: bool,
    is_read: bool,
    is_deleted: bool,
    is_echo: bool,
    needs_password: bool,
    size: u32,
}

impl PplMessage {
    pub fn from_header(path: &std::path::Path, header: &JamMessageHeader) -> Self {
        Self {
            path: path.to_path_buf(),
            valid: true,
            number: header.message_number,
            from: header.from().map(ToString::to_string).unwrap_or_default(),
            to: header.to().map(ToString::to_string).unwrap_or_default(),
            subject: header.subject().map(ToString::to_string).unwrap_or_default(),
            written: header.date_written as i64,
            reply_to: header.reply_to,
            status: message_status(header).to_string(),
            is_private: header.is_private(),
            is_read: header.is_read(),
            is_deleted: header.is_deleted(),
            is_echo: header.attributes & jam_attributes::MSG_TYPEECHO != 0,
            needs_password: header.needs_password(),
            size: header.txt_len,
        }
    }

    /// What a number nobody has answers with, so a walk can read `Valid` instead of failing.
    pub fn missing() -> VariableValue {
        Self::default().value()
    }

    pub fn value(self) -> VariableValue {
        user_data_value(self, MSG_ID)
    }

    fn written_at(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.written, 0).unwrap_or_default()
    }
}

impl UserData for PplMessage {
    const TYPE_NAME: &'static str = "Msg";

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        registry.add_property(NUMBER.clone(), VariableType::Integer, false);
        registry.add_property(VALID.clone(), VariableType::Boolean, false);
        for name in [&*FROM, &*TO, &*SUBJECT, &*STATUS] {
            registry.add_property(name.clone(), VariableType::String, false);
        }
        registry.add_property(DATE.clone(), VariableType::Date, false);
        registry.add_property(TIME.clone(), VariableType::Time, false);
        registry.add_property(REPLY_TO.clone(), VariableType::Integer, false);
        registry.add_property(SIZE.clone(), VariableType::Integer, false);
        for name in [&*IS_PRIVATE, &*IS_READ, &*IS_DELETED, &*IS_ECHO, &*NEEDS_PASSWORD] {
            registry.add_property(name.clone(), VariableType::Boolean, false);
        }
        registry.add_function(TEXT.clone(), Vec::new(), VariableType::String);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplMessage {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        let value = if *name == *NUMBER {
            VariableValue::new_int(self.number as i32)
        } else if *name == *VALID {
            VariableValue::new_bool(self.valid)
        } else if *name == *FROM {
            VariableValue::new_string(self.from.clone())
        } else if *name == *TO {
            VariableValue::new_string(self.to.clone())
        } else if *name == *SUBJECT {
            VariableValue::new_string(self.subject.clone())
        } else if *name == *STATUS {
            VariableValue::new_string(self.status.clone())
        } else if *name == *DATE {
            // A message that is not there has no date; zero is what prints as 00/00/00.
            let date = if self.valid {
                IcbDate::from_utc(&self.written_at()).to_pcboard_date()
            } else {
                0
            };
            VariableValue::new(VariableType::Date, VariableData::from_int(date))
        } else if *name == *TIME {
            let time = if self.valid {
                IcbTime::from_naive(self.written_at().naive_utc()).to_pcboard_time()
            } else {
                0
            };
            VariableValue::new(VariableType::Time, VariableData::from_int(time))
        } else if *name == *REPLY_TO {
            VariableValue::new_int(self.reply_to as i32)
        } else if *name == *SIZE {
            VariableValue::new_int(self.size as i32)
        } else if *name == *IS_PRIVATE {
            VariableValue::new_bool(self.is_private)
        } else if *name == *IS_READ {
            VariableValue::new_bool(self.is_read)
        } else if *name == *IS_DELETED {
            VariableValue::new_bool(self.is_deleted)
        } else if *name == *IS_ECHO {
            VariableValue::new_bool(self.is_echo)
        } else if *name == *NEEDS_PASSWORD {
            VariableValue::new_bool(self.needs_password)
        } else {
            return Err(format!("Unknown MSG property {name}").into());
        };
        Ok(value)
    }

    async fn set_property_value(&self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _val: VariableValue) -> crate::Res<()> {
        Err(format!("MSG property {name} is read-only").into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        _arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if *name == *TEXT {
            if !self.valid {
                vm.operation_succeeded();
                return Ok(VariableValue::new_string(String::new()));
            }
            let base = match JamMessageBase::open(&self.path) {
                Ok(base) => base,
                Err(error) => {
                    vm.set_error(message_error("can't open message base", &self.path, &error));
                    return Ok(VariableValue::new_string(String::new()));
                }
            };
            let header = match base.read_header(self.number) {
                Ok(header) => header,
                Err(error) if message_is_missing(&error) => {
                    vm.operation_succeeded();
                    return Ok(VariableValue::new_string(String::new()));
                }
                Err(error) => {
                    vm.set_error(message_error(&format!("can't read message {} from", self.number), &self.path, &error));
                    return Ok(VariableValue::new_string(String::new()));
                }
            };
            return Ok(match base.read_message_text(&header) {
                Ok(text) => {
                    vm.operation_succeeded();
                    VariableValue::new_string(text.to_string())
                }
                Err(error) => {
                    vm.set_error(message_error(&format!("can't read message {} text from", self.number), &self.path, &error));
                    VariableValue::new_string(String::new())
                }
            });
        }
        Err(format!("Unknown MSG function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown MSG method {name}").into())
    }
}
