pub use icy_board_ppl::parser::*;

#[cfg(all(test, feature = "bbs"))]
mod runtime_tests;

/// The compiler's metadata registry, with BBS object factories attached for execution.
pub fn icy_board_registry() -> UserTypeRegistry {
    let registry = UserTypeRegistry::icy_board_registry();
    #[cfg(feature = "bbs")]
    let registry = runtime_factories(registry);
    registry
}

#[cfg(feature = "bbs")]
fn runtime_factories(mut registry: UserTypeRegistry) -> UserTypeRegistry {
    use crate::compiler::user_data::UserData;
    use crate::icy_board::{
        conferences::Conference,
        doors::Door,
        file_directory::FileDirectory,
        message_area::MessageArea,
        state::{ppl_audio, ppl_board, ppl_error, ppl_events, ppl_files, ppl_http, ppl_message, ppl_regex, ppl_surface, ppl_terminal_info, ppl_user},
    };

    fn bind<T: UserData>(registry: &mut UserTypeRegistry, id: usize) {
        let members = registry.types.get_mut(&(id as u32)).expect("builtin metadata missing");
        members.static_receiver = T::STATIC_RECEIVER;
        members.empty_value = T::EMPTY_VALUE;
    }
    bind::<Conference>(&mut registry, CONFERENCE_ID);
    bind::<MessageArea>(&mut registry, MESSAGE_AREA_ID);
    bind::<FileDirectory>(&mut registry, FILE_DIRECTORY_ID);
    bind::<ppl_files::PplFileEntry>(&mut registry, FILE_ENTRY_ID);
    bind::<ppl_files::PplFilePage>(&mut registry, FILE_PAGE_ID);
    bind::<Door>(&mut registry, DOOR_ID);
    bind::<ppl_user::PplUser>(&mut registry, USER_ID);
    bind::<ppl_surface::PplSurface>(&mut registry, SURFACE_ID);
    bind::<ppl_audio::PplAudio>(&mut registry, AUDIO_ID);
    bind::<ppl_events::PplEvent>(&mut registry, EVENT_ID);
    bind::<ppl_error::PplError>(&mut registry, ERROR_ID);
    bind::<ppl_terminal_info::PplTerminalInfo>(&mut registry, TERM_INFO_ID);
    bind::<ppl_board::PplBoard>(&mut registry, BOARD_ID);
    bind::<ppl_message::PplMessage>(&mut registry, MSG_ID);
    bind::<ppl_http::PplHttp>(&mut registry, HTTP_ID);
    bind::<ppl_http::PplHttpRequest>(&mut registry, HTTP_REQUEST_ID);
    bind::<ppl_http::PplHttpResponse>(&mut registry, HTTP_RESPONSE_ID);
    bind::<ppl_regex::PplRegex>(&mut registry, REGEX_ID);
    bind::<ppl_regex::PplRegexMatch>(&mut registry, REGEX_MATCH_ID);
    // These facades otherwise dispatch into the live session even without a handle.
    macro_rules! inert {
        ($($id:ident),+ $(,)?) => {$(
            registry.types.get_mut(&($id as u32)).expect("builtin metadata missing").empty_value = Some(inert_value::<$id>);
        )+};
    }
    inert!(TERM_INPUT_ID, TERMINAL_ID, GFX_ID, MARGINS_ID, PALETTE_ID, MACROS_ID, SESSION_ID, HTTP_ID);
    registry
}

#[cfg(feature = "bbs")]
struct InertHost<const ID: usize>;

#[cfg(feature = "bbs")]
fn inert_value<const ID: usize>() -> crate::executable::VariableValue {
    crate::compiler::user_data::user_data_value(InertHost::<ID>, ID)
}

#[cfg(feature = "bbs")]
impl<const ID: usize> InertHost<ID> {
    fn empty(vm: &crate::vm::VirtualMachine<'_>, vtype: crate::executable::VariableType, rank: u8) -> crate::Res<crate::executable::VariableValue> {
        let mut value = vm.type_default(vtype)?;
        if rank != 0 {
            value.generic_data = crate::executable::VarHeader {
                dim: rank,
                flags: crate::executable::variable_table::VARIABLE_FLAG_DYNAMIC_ARRAY,
                ..Default::default()
            }
            .create_generic_data()
            .ok_or(crate::vm::VMError::InternalVMError)?;
        }
        Ok(value)
    }

    fn fail(vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>) {
        use crate::icy_board::state::ppl_error::{ERR_INVALID, ERR_KIND_FONT, ERR_KIND_GFX, ERR_KIND_NET, ERR_KIND_TERM, ERR_KIND_USER, PplError};
        let kind = match ID {
            GFX_ID => ERR_KIND_GFX,
            HTTP_ID => ERR_KIND_NET,
            SESSION_ID => ERR_KIND_USER,
            TERMINAL_ID if name.as_str().eq_ignore_ascii_case("SetFont") || name.as_str().eq_ignore_ascii_case("LoadFont") => ERR_KIND_FONT,
            _ => ERR_KIND_TERM,
        };
        vm.set_error(PplError::new(kind, ERR_INVALID, format!("uninitialized host object {ID}: {name}")));
    }
}

#[cfg(feature = "bbs")]
#[async_trait::async_trait(?Send)]
impl<const ID: usize> crate::compiler::user_data::UserDataValue for InertHost<ID> {
    fn get_property_value(&self, vm: &crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>) -> crate::Res<crate::executable::VariableValue> {
        let members = vm
            .type_registry
            .get_type_from_id(ID as u32)
            .ok_or(crate::vm::VMError::NoObjectFound(ID as u32))?;
        let vtype = members.fields.get(name).ok_or_else(|| format!("Unknown host property {ID}.{name}"))?;
        Self::empty(vm, *vtype, members.field_ranks.get(name).copied().unwrap_or_default())
    }

    async fn set_property_value(
        &self,
        _vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        _value: crate::executable::VariableValue,
    ) -> crate::Res<()> {
        Err(format!("Host property {ID}.{name} is read-only").into())
    }

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        _arguments: &[crate::executable::VariableValue],
    ) -> crate::Res<crate::executable::VariableValue> {
        let members = vm
            .type_registry
            .get_type_from_id(ID as u32)
            .ok_or(crate::vm::VMError::NoObjectFound(ID as u32))?;
        let function = members.functions.get(name).ok_or_else(|| format!("Unknown host function {ID}.{name}"))?;
        let value = Self::empty(vm, function.return_type, function.return_rank)?;
        Self::fail(vm, name);
        Ok(value)
    }

    async fn call_method(
        &mut self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        _arguments: &[crate::executable::VariableValue],
    ) -> crate::Res<()> {
        let members = vm
            .type_registry
            .get_type_from_id(ID as u32)
            .ok_or(crate::vm::VMError::NoObjectFound(ID as u32))?;
        if !members.procedures.contains_key(name) {
            return Err(format!("Unknown host method {ID}.{name}").into());
        }
        Self::fail(vm, name);
        Ok(())
    }
}
