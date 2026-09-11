use async_trait::async_trait;

#[cfg(test)]
#[path = "../../vm/tests/message_api.rs"]
mod message_api_tests;

use crate::{
    compiler::user_data::{UserData, UserDataMemberRegistry, UserDataValue, user_data_value},
    executable::VariableValue,
    icy_board::{file_directory::FileDirectory, message_area::MessageArea},
    parser::{CONFERENCE_ID, FILE_DIRECTORY_ID, MESSAGE_AREA_ID, SESSION_ID},
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
member_name!(REQUEST_PASSWORD_RECOVERY, "RequestPasswordRecovery");

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
        crate::parser::board_catalog::register_members(SESSION_ID, registry);
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
            area.conference_number = session.current_conference_number as usize;
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
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue> {
        if ["PostMessage", "ReplyMessage", "EditMessage", "ReplyHeader"]
            .iter()
            .any(|method| name.as_ref().eq_ignore_ascii_case(method))
        {
            return message_api::call(vm, name.as_ref(), arguments).await;
        }
        if *name == *REQUEST_PASSWORD_RECOVERY {
            let user_name = arguments[0].as_string();
            let (enabled, index, service) = {
                let board = vm.icy_board_state.get_board().await;
                (
                    board.config.password_recovery.enabled,
                    board.users.find_by_name(&user_name),
                    board.password_recovery_service.clone(),
                )
            };
            if enabled {
                if let Some(index) = index {
                    // Delivery and eligibility details stay in the service's log, never in Error.Last().
                    let _ = service.issue(&vm.icy_board_state.board, index, chrono::Utc::now()).await;
                } else {
                    log::info!("PPL recovery email: user not found");
                }
            }
            vm.operation_succeeded();
            return Ok(VariableValue::new_bool(enabled));
        }
        Err(format!("Unknown SESSION function {name}").into())
    }

    async fn call_method(&mut self, _vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, _arguments: &[VariableValue]) -> crate::Res<()> {
        Err(format!("Unknown SESSION method {name}").into())
    }
}

mod message_api {
    use super::*;
    use crate::{
        Res,
        executable::GenericVariableData,
        icy_board::{
            conferences::Conference,
            state::{
                IcyBoardState,
                ppl_error::*,
                ppl_message::{MessageHeader, PplMessage},
            },
        },
    };

    fn object<T: Clone + 'static>(value: &VariableValue) -> Res<T> {
        let GenericVariableData::UserData(object) = &value.generic_data else {
            return Err("Invalid message API object".into());
        };
        let crate::executable::VariableType::UserData(type_id) = value.vtype else {
            return Err("Invalid message API type".into());
        };
        let object = crate::compiler::user_data::runtime_object(object.as_ref(), type_id)?;
        (object as &dyn std::any::Any)
            .downcast_ref::<T>()
            .cloned()
            .ok_or_else(|| "Invalid message API receiver".into())
    }

    struct Context<'a> {
        state: &'a mut IcyBoardState,
        conference: Conference,
        number: u16,
        area: usize,
        tokens: std::collections::VecDeque<String>,
    }

    impl Drop for Context<'_> {
        fn drop(&mut self) {
            self.state.session.current_conference = self.conference.clone();
            self.state.session.current_conference_number = self.number;
            self.state.session.current_message_area = self.area;
            self.state.session.tokens = std::mem::take(&mut self.tokens);
        }
    }

    fn invalid(message: &str) -> Box<dyn std::error::Error + Send + Sync> {
        std::io::Error::new(std::io::ErrorKind::InvalidInput, message).into()
    }

    fn denied(message: &str) -> Box<dyn std::error::Error + Send + Sync> {
        std::io::Error::new(std::io::ErrorKind::PermissionDenied, message).into()
    }

    pub(super) async fn call(vm: &mut crate::vm::VirtualMachine<'_>, name: &str, arguments: &[VariableValue]) -> Res<VariableValue> {
        let mut saved = None;
        let result = invoke(vm.icy_board_state, name, arguments, &mut saved).await;
        if saved.is_some() {
            vm.invalidate_message_base();
        }
        let value = saved.map(PplMessage::value);
        match result {
            Ok(header) => {
                vm.operation_succeeded();
                Ok(header.or(value).unwrap_or_else(PplMessage::missing))
            }
            Err(error) => {
                let code = if let Some(error) = error.downcast_ref::<std::io::Error>() {
                    match error.kind() {
                        std::io::ErrorKind::PermissionDenied => ERR_DENIED,
                        std::io::ErrorKind::InvalidInput | std::io::ErrorKind::AlreadyExists | std::io::ErrorKind::NotFound => ERR_INVALID,
                        std::io::ErrorKind::TimedOut => ERR_TIMEOUT,
                        std::io::ErrorKind::FileTooLarge => ERR_LIMIT,
                        _ => ERR_IO,
                    }
                } else if error.is::<crate::icy_board::state::user_commands::pcb::message_attachment::MessageCreditDenied>() {
                    ERR_DENIED
                } else {
                    ERR_IO
                };
                vm.set_error(PplError::new(ERR_KIND_MSG, code, error.to_string()));
                Ok(value.unwrap_or_else(|| {
                    if name.eq_ignore_ascii_case("ReplyHeader") {
                        MessageHeader::default().value()
                    } else {
                        PplMessage::missing()
                    }
                }))
            }
        }
    }

    async fn invoke(state: &mut IcyBoardState, name: &str, arguments: &[VariableValue], saved: &mut Option<PplMessage>) -> Res<Option<VariableValue>> {
        let post = name.eq_ignore_ascii_case("PostMessage");
        let reply_header = name.eq_ignore_ascii_case("ReplyHeader");
        let edit = name.eq_ignore_ascii_case("EditMessage");
        let source = if post { None } else { Some(object::<PplMessage>(&arguments[0])?) };
        let (conference_number, area_number, source_path) = if post {
            let area = object::<MessageArea>(&arguments[0])?;
            if !area.valid {
                return Err(invalid("Invalid AREA"));
            }
            (area.conference_number, area.number, area.path)
        } else {
            let source = source.as_ref().unwrap();
            if !source.valid {
                return Err(invalid("Invalid MSG"));
            }
            let (conference, area) = source.area.ok_or_else(|| invalid("MSG has no area"))?;
            (conference, area, source.path.clone())
        };
        let conference_number = u16::try_from(conference_number).map_err(|_| invalid("Invalid conference"))?;
        let conference = state
            .get_board()
            .await
            .conferences
            .get(conference_number as usize)
            .cloned()
            .ok_or_else(|| invalid("Missing conference"))?;
        let area = conference
            .areas
            .as_ref()
            .and_then(|areas| areas.get(area_number))
            .ok_or_else(|| invalid("Missing area"))?;
        if !state.action_conference_access(conference_number, &conference) || !area.req_level_to_list.session_can_access(&state.session) {
            return Err(denied("Message area access denied"));
        }
        let current_path = state.resolve_path(&area.path);
        let original_path = state.resolve_path(&source_path);
        if current_path != original_path {
            return Err(invalid("Message area configuration changed"));
        }
        if !reply_header {
            if (!edit && !state.session.user_command_level.cmd_e.session_can_access(&state.session))
                || state.read_action_target(conference_number, area_number).await?.is_none()
            {
                return Err(denied("Writing to this message area is not permitted"));
            }
        }
        let mut header = arguments.get(1).map(MessageHeader::from_value).transpose()?;
        let initial_text = arguments
            .get(2)
            .map(VariableValue::as_string)
            .unwrap_or_default()
            .replace("\r\n", "\n")
            .replace('\r', "\n");
        if initial_text.lines().count() > state.get_board().await.config.message.max_msg_lines.max(1) as usize {
            return Err(std::io::Error::new(std::io::ErrorKind::FileTooLarge, "Initial text exceeds the message line limit").into());
        }
        if let Some(header) = &mut header {
            if [&header.from, &header.to, &header.subject]
                .iter()
                .any(|value| value.chars().any(char::is_control))
            {
                return Err(invalid("Message header contains control characters"));
            }
            if header.from.is_empty() {
                header.from = state.session.get_username_or_alias();
            }
            if header.from.to_ascii_uppercase().contains("@USER@") {
                return Err(invalid("Message sender cannot contain @USER@"));
            }
            let allowed_from = if edit {
                source
                    .as_ref()
                    .and_then(|source| source.stored_header.as_ref())
                    .and_then(|header| header.from())
                    .map(ToString::to_string)
                    .unwrap_or_default()
            } else {
                state.session.get_username_or_alias()
            };
            if !header.from.eq_ignore_ascii_case(&allowed_from)
                && !state
                    .get_board()
                    .await
                    .config
                    .sysop_command_level
                    .edit_message_headers
                    .session_can_access(&state.session)
            {
                return Err(denied("Changing the message sender is not permitted"));
            }
            if edit && let Some(original) = source.as_ref().and_then(|source| source.stored_header.as_ref()) {
                let changed = header.from != original.from().map(ToString::to_string).unwrap_or_default()
                    || header.to != original.to().map(ToString::to_string).unwrap_or_default()
                    || header.subject != original.subject().map(ToString::to_string).unwrap_or_default()
                    || header.is_private != original.is_private();
                if changed && !state.session.user_command_level.cmd_e.session_can_access(&state.session) {
                    return Err(denied("Editing message headers is not permitted"));
                }
                if header.is_private != original.is_private() {
                    if !state
                        .get_board()
                        .await
                        .config
                        .sysop_command_level
                        .protect_unprotect_messages
                        .session_can_access(&state.session)
                    {
                        return Err(denied("Changing message privacy is not permitted"));
                    }
                    if header.is_private && (conference.disallow_private_msgs || header.to.eq_ignore_ascii_case("ALL"))
                        || !header.is_private && conference.private_msgs
                    {
                        return Err(denied("Message privacy conflicts with the target area"));
                    }
                }
            }
        }
        let previous = state.session.current_conference.clone();
        let number = state.session.current_conference_number;
        let area = state.session.current_message_area;
        let tokens = std::mem::take(&mut state.session.tokens);
        state.session.current_conference = conference;
        state.session.current_conference_number = conference_number;
        state.session.current_message_area = area_number;
        let context = Context {
            state,
            conference: previous,
            number,
            area,
            tokens,
        };
        if post {
            context.state.enter_message_with_defaults(header.as_ref(), &initial_text, saved).await?;
        } else {
            let mut source = source.unwrap();
            source.path = current_path;
            if reply_header {
                let header = context
                    .state
                    .authorized_message_snapshot(&source)
                    .await?
                    .map(|message| context.state.message_reply_defaults(message.header()))
                    .unwrap_or_default();
                return Ok(Some(header.value()));
            } else if edit {
                context.state.edit_message_snapshot(&source, header.as_ref(), saved).await?;
            } else {
                context
                    .state
                    .reply_from_base_with_defaults(
                        &source.path,
                        source.number,
                        false,
                        false,
                        header.as_ref(),
                        &initial_text,
                        source.stored_header.as_ref(),
                        saved,
                    )
                    .await?;
            }
        }
        Ok(None)
    }
}
