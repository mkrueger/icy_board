use super::*;
use crate::{
    compiler::user_data::runtime_object,
    executable::{GenericVariableData, VariableType},
};

#[test]
fn runtime_registry_preserves_core_type_identity_and_metadata() {
    let runtime: std::sync::Arc<icy_board_ppl::parser::UserTypeRegistry> = std::sync::Arc::new(icy_board_registry());
    let metadata = UserTypeRegistry::icy_board_registry();
    assert_eq!(runtime.registered_types, metadata.registered_types);
    assert_eq!(runtime.enums(), metadata.enums());
    assert_eq!(runtime.types.len(), 24);
    for (id, count) in [
        (CONFERENCE_ID, 16),
        (MESSAGE_AREA_ID, 15),
        (FILE_DIRECTORY_ID, 9),
        (DOOR_ID, 7),
        (SURFACE_ID, 18),
        (EVENT_ID, 23),
        (AUDIO_ID, 10),
        (ERROR_ID, 7),
        (TERM_INFO_ID, 20),
        (TERM_INPUT_ID, 7),
        (TERMINAL_ID, 10),
        (GFX_ID, 4),
        (MARGINS_ID, 11),
        (PALETTE_ID, 3),
        (MACROS_ID, 6),
        (BOARD_ID, 7),
        (SESSION_ID, 14),
        (USER_ID, 54),
        (MSG_ID, 16),
        (HTTP_ID, 7),
        (HTTP_REQUEST_ID, 8),
        (HTTP_RESPONSE_ID, 10),
        (REGEX_ID, 10),
        (REGEX_MATCH_ID, 13),
    ] {
        let actual = runtime.get_type_from_id(id as u8).unwrap();
        let expected = metadata.get_type_from_id(id as u8).unwrap();
        assert_eq!(actual.id_table.len(), count, "type {id}");
        assert_eq!(actual.member_id_lookup, expected.member_id_lookup);
        assert_eq!(actual.fields, expected.fields);
        assert_eq!(actual.field_ranks, expected.field_ranks);
        assert_eq!(actual.statics, expected.statics);
        assert_eq!(actual.instance_provider, expected.instance_provider);
        for (name, function) in &actual.functions {
            let expected = &expected.functions[name];
            assert_eq!(function.parameters, expected.parameters);
            assert_eq!(function.parameter_names, expected.parameter_names);
            assert_eq!(function.required, expected.required);
            assert_eq!(function.return_type, expected.return_type);
            assert_eq!(function.return_rank, expected.return_rank);
        }
    }
}

#[test]
fn runtime_factories_produce_dispatchable_objects() {
    let runtime = icy_board_registry();
    let metadata = UserTypeRegistry::icy_board_registry();
    for (&id, members) in &runtime.types {
        assert!(metadata.types[&id].static_receiver.is_none());
        assert!(metadata.types[&id].empty_value.is_none());
        let has_static = [SURFACE_ID, AUDIO_ID, ERROR_ID, HTTP_ID, HTTP_REQUEST_ID, HTTP_RESPONSE_ID, REGEX_ID].contains(&(id as usize));
        let has_empty = [CONFERENCE_ID, MESSAGE_AREA_ID, FILE_DIRECTORY_ID, DOOR_ID, USER_ID].contains(&(id as usize));
        assert_eq!(members.static_receiver.is_some(), has_static, "type {id}");
        assert_eq!(members.empty_value.is_some(), has_empty, "type {id}");
        for factory in [members.static_receiver, members.empty_value].into_iter().flatten() {
            let value = factory();
            assert_eq!(value.get_type(), VariableType::UserData(id));
            let GenericVariableData::UserData(object) = value.generic_data else {
                panic!("factory for {id} returned no object")
            };
            assert!(runtime_object(object.as_ref(), id).is_ok(), "type {id}");
        }
    }
}

#[test]
fn bridge_rejects_foreign_payloads_without_panicking() {
    let value = icy_board_ppl::compiler::user_data::user_data_value("not a runtime object", SURFACE_ID);
    let GenericVariableData::UserData(object) = value.generic_data else {
        unreachable!()
    };
    let error = runtime_object(object.as_ref(), SURFACE_ID as u8).err().unwrap();
    assert!(matches!(error.downcast_ref::<crate::vm::VMError>(), Some(crate::vm::VMError::NoObjectFound(id)) if *id == SURFACE_ID as u8));
}

#[test]
fn resource_handles_survive_the_runtime_wrapper() {
    use crate::icy_board::state::{ppl_audio::PplAudio, ppl_surface::PplSurface};
    for (value, id, handle) in [
        (PplSurface::value(42), SURFACE_ID, 42),
        (PplAudio::value(3), AUDIO_ID, 3),
        (PplAudio::invalid(), AUDIO_ID, -1),
    ] {
        assert_eq!(unsafe { value.data.int_value }, handle);
        let GenericVariableData::UserData(object) = value.generic_data else {
            unreachable!()
        };
        assert!(runtime_object(object.as_ref(), id as u8).is_ok());
    }
}
