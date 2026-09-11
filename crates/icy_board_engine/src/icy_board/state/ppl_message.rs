use std::path::PathBuf;

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use jamjam::jam::{attributes as jam_attributes, msg_header::JamMessageHeader};

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    datetime::{IcbDate, IcbTime},
    executable::{GenericVariableData, VariableData, VariableType, VariableValue},
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

#[derive(Clone, Debug, Default)]
pub(crate) struct MessageHeader {
    pub from: String,
    pub to: String,
    pub subject: String,
    pub is_private: bool,
}

impl MessageHeader {
    pub(crate) fn from_value(value: &VariableValue) -> crate::Res<Self> {
        let GenericVariableData::Record(fields) = &value.generic_data else {
            return Err("Invalid MSGHEADER value".into());
        };
        if fields.len() != 4 {
            return Err("Invalid MSGHEADER fields".into());
        }
        Ok(Self {
            from: fields[0].as_string(),
            to: fields[1].as_string(),
            subject: fields[2].as_string(),
            is_private: fields[3].as_bool(),
        })
    }

    pub(crate) fn from_header(header: &JamMessageHeader) -> Self {
        Self {
            from: header.from().map(ToString::to_string).unwrap_or_default(),
            to: header.to().map(ToString::to_string).unwrap_or_default(),
            subject: header.subject().map(ToString::to_string).unwrap_or_default(),
            is_private: header.is_private(),
        }
    }

    pub(crate) fn value(self) -> VariableValue {
        VariableValue {
            vtype: VariableType::UserData(crate::parser::MSG_HEADER_ID as u32),
            data: VariableData::default(),
            generic_data: GenericVariableData::Record(
                vec![
                    VariableValue::new_unbounded_string(self.from),
                    VariableValue::new_unbounded_string(self.to),
                    VariableValue::new_unbounded_string(self.subject),
                    VariableValue::new_bool(self.is_private),
                ]
                .into(),
            ),
        }
    }

    pub(crate) fn apply(&self, header: &mut JamMessageHeader) {
        header.set_from(self.from.clone().into());
        header.set_to(self.to.clone().into());
        header.set_subject(self.subject.clone().into());
        header.attributes = (header.attributes & !jam_attributes::MSG_PRIVATE) | if self.is_private { jam_attributes::MSG_PRIVATE } else { 0 };
    }
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
member_name!(HEADER, "Header");

/// One message, read out of its area. The header travels with the value; the body
/// stays in the base until `Text()` asks for it, so listing headers does not pay
/// for every message it walks past.
#[derive(Clone, Debug, Default)]
pub struct PplMessage {
    /// Where the body can be fetched from, empty for a message that is not there.
    pub(crate) path: PathBuf,
    pub(crate) valid: bool,
    pub(crate) number: u32,
    pub(crate) stored_header: Option<JamMessageHeader>,
    pub(crate) area: Option<(usize, usize)>,
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
            stored_header: Some(header.clone()),
            area: None,
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

    pub(crate) fn in_area(mut self, conference: usize, area: usize) -> Self {
        self.area = Some((conference, area));
        self
    }

    pub(crate) fn append(base: &mut jamjam::jam::JamMessageBase, message: &jamjam::jam::JamMessage) -> jamjam::Result<Self> {
        base.transaction(|base| {
            let mut header = message.header().clone();
            header.offset =
                u32::try_from(std::fs::metadata(base.path().with_extension("jdt"))?.len()).map_err(|_| std::io::Error::other("JAM text file is full"))?;
            header.txt_len = u32::try_from(message.text().len()).map_err(|_| std::io::Error::other("Message is too large"))?;
            header.message_number = base.write_message(message)?;
            Ok(Self::from_header(base.path(), &header))
        })
    }

    fn written_at(&self) -> DateTime<Utc> {
        DateTime::from_timestamp(self.written, 0).unwrap_or_default()
    }
}

#[cfg(test)]
mod header_tests {
    use super::*;

    #[test]
    fn appended_message_snapshot_matches_persisted_header() {
        let directory = tempfile::tempdir().unwrap();
        let mut base = jamjam::jam::JamMessageBase::create(directory.path().join("messages")).unwrap();
        for text in ["First body", "Longer second body"] {
            let message = jamjam::jam::JamMessage::default().with_subject("Snapshot".into()).with_text(text.into());
            let saved = PplMessage::append(&mut base, &message).unwrap();
            let stored = base.read_header(saved.number).unwrap();
            assert_eq!(saved.stored_header.as_ref().unwrap().offset, stored.offset);
            assert_eq!(saved.size, stored.txt_len);
            assert_eq!(saved.subject, stored.subject().unwrap().to_string());
            assert_eq!(base.read_message_text(&stored).unwrap(), *message.text());
        }
    }
}

impl UserData for PplMessage {
    const TYPE_NAME: &'static str = "Msg";
    const EMPTY_VALUE: Option<fn() -> VariableValue> = Some(Self::missing);

    fn register_members<F: UserDataMemberRegistry>(registry: &mut F) {
        crate::parser::board_catalog::register_members(crate::parser::MSG_ID, registry);
    }
}

#[async_trait(?Send)]
impl UserDataValue for PplMessage {
    fn get_property_value(&self, _vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> crate::Res<VariableValue> {
        let value = if *name == *NUMBER {
            VariableValue::new_long(i64::from(self.number))
        } else if *name == *VALID {
            VariableValue::new_bool(self.valid)
        } else if *name == *FROM {
            VariableValue::new_unbounded_string(self.from.clone())
        } else if *name == *TO {
            VariableValue::new_unbounded_string(self.to.clone())
        } else if *name == *SUBJECT {
            VariableValue::new_unbounded_string(self.subject.clone())
        } else if *name == *STATUS {
            VariableValue::new_unbounded_string(self.status.clone())
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
            VariableValue::new_long(i64::from(self.reply_to))
        } else if *name == *SIZE {
            VariableValue::new_long(i64::from(self.size))
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
        } else if *name == *HEADER {
            self.stored_header.as_ref().map(MessageHeader::from_header).unwrap_or_default().value()
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
                return Ok(VariableValue::new_unbounded_string(String::new()));
            }
            let text = vm.with_message_base(&self.path, |base| match base.read_header(self.number) {
                Ok(header) => base.read_message_text(&header).map(|text| Some(text.to_string())),
                Err(error) if message_is_missing(&error) => Ok(None),
                Err(error) => Err(error),
            });
            return Ok(match text {
                Ok(text) => {
                    vm.operation_succeeded();
                    VariableValue::new_unbounded_string(text.unwrap_or_default())
                }
                Err(error) => {
                    vm.set_error(message_error(&format!("can't read message {} from", self.number), &self.path, &error));
                    VariableValue::new_unbounded_string(String::new())
                }
            });
        }
        Err(format!("Unknown MSG function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown MSG method {name}").into())
    }
}
