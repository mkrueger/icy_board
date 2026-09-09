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
        file_directory::FileDirectory,
        message_area::MessageArea,
        state::{ppl_audio, ppl_error, ppl_http, ppl_regex, ppl_surface, ppl_user},
    };

    fn bind<T: UserData>(registry: &mut UserTypeRegistry, id: usize) {
        let members = registry.types.get_mut(&(id as u8)).expect("builtin metadata missing");
        members.static_receiver = T::STATIC_RECEIVER;
        members.empty_value = T::EMPTY_VALUE;
    }
    bind::<Conference>(&mut registry, CONFERENCE_ID);
    bind::<MessageArea>(&mut registry, MESSAGE_AREA_ID);
    bind::<FileDirectory>(&mut registry, FILE_DIRECTORY_ID);
    bind::<ppl_user::PplUser>(&mut registry, USER_ID);
    bind::<ppl_surface::PplSurface>(&mut registry, SURFACE_ID);
    bind::<ppl_audio::PplAudio>(&mut registry, AUDIO_ID);
    bind::<ppl_error::PplError>(&mut registry, ERROR_ID);
    bind::<ppl_http::PplHttp>(&mut registry, HTTP_ID);
    bind::<ppl_http::PplHttpRequest>(&mut registry, HTTP_REQUEST_ID);
    bind::<ppl_http::PplHttpResponse>(&mut registry, HTTP_RESPONSE_ID);
    bind::<ppl_regex::PplRegex>(&mut registry, REGEX_ID);
    registry.types.get_mut(&(DOOR_ID as u8)).expect("builtin metadata missing").empty_value =
        Some(|| crate::compiler::user_data::user_data_value(crate::icy_board::doors::Door::default(), DOOR_ID));
    registry
}
