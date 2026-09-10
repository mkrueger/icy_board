use std::sync::Arc;

use crate::{
    Res,
    executable::{GenericVariableData, TableEntry, VariableType, VariableValue, create_record_value, variable_table::VARIABLE_FLAG_DYNAMIC_ARRAY},
    parser::UserTypeRegistry,
};

use super::{VMError, VirtualMachine};

/// Bind only uninitialized host leaves; existing objects retain their identity and state.
pub(crate) fn bind_host_defaults(value: &mut VariableValue, registry: &UserTypeRegistry) -> Res<()> {
    match &mut value.generic_data {
        GenericVariableData::None => {
            if let VariableType::UserData(id) = value.vtype
                && let Some(members) = registry.get_type_from_id(id)
            {
                let factory = members.empty_value.ok_or(VMError::NoObjectFound(id))?;
                let empty = factory();
                if empty.vtype != value.vtype || !matches!(empty.generic_data, GenericVariableData::UserData(_)) {
                    return Err(VMError::NoObjectFound(id).into());
                }
                *value = empty;
            }
        }
        GenericVariableData::Record(values) | GenericVariableData::Dim1(values) => {
            for value in Arc::make_mut(values) {
                bind_host_defaults(value, registry)?;
            }
        }
        GenericVariableData::Dim2(values) => {
            for value in Arc::make_mut(values).iter_mut().flatten() {
                bind_host_defaults(value, registry)?;
            }
        }
        GenericVariableData::Dim3(values) => {
            for value in Arc::make_mut(values).iter_mut().flatten().flatten() {
                bind_host_defaults(value, registry)?;
            }
        }
        _ => {}
    }
    Ok(())
}

impl VirtualMachine<'_> {
    pub(crate) fn initialize_host_defaults(&mut self) -> Res<()> {
        for id in 1..=self.variable_table.get_entries().len() {
            bind_host_defaults(self.variable_table.get_value_mut(id), self.type_registry)?;
        }
        Ok(())
    }

    fn unbound_type_default(&self, vtype: VariableType) -> Res<VariableValue> {
        let VariableType::UserData(id) = vtype else {
            return Ok(vtype.create_empty_value());
        };
        if !self.variable_table.enums.contains_key(&id)
            && let Some(definition) = self.type_registry.get_enum_from_id(id)
        {
            let default = *definition.domain.first().ok_or(VMError::InternalVMError)?;
            return Ok(VariableValue::new_enum(vtype, default, default));
        }
        create_record_value(id, &self.user_types, &self.variable_table.enums).ok_or_else(|| VMError::InternalVMError.into())
    }

    /// Rebuild records from declarations, not a previous value's resized fields.
    pub(crate) fn type_default(&self, vtype: VariableType) -> Res<VariableValue> {
        let mut value = self.unbound_type_default(vtype)?;
        bind_host_defaults(&mut value, self.type_registry)?;
        Ok(value)
    }

    /// Missing modern host/record elements need the same defaults as declarations.
    pub(crate) fn read_array_element(&self, array: &VariableValue, first: usize, second: usize, third: usize) -> Res<VariableValue> {
        if self.variable_table.get_version() < 400 || !matches!(array.vtype, VariableType::UserData(_)) {
            return Ok(self.variable_table.array_value(array, first, second, third));
        }
        let element = match &array.generic_data {
            GenericVariableData::Dim1(values) => values.get(first),
            GenericVariableData::Dim2(values) => values.get(first).and_then(|row| row.get(second)),
            GenericVariableData::Dim3(values) => values.get(first).and_then(|plane| plane.get(second)).and_then(|row| row.get(third)),
            _ => None,
        };
        match element {
            Some(value) => Ok(value.clone()),
            None => self.type_default(array.vtype),
        }
    }

    /// Bind after allocating so distinct new array elements get distinct host defaults.
    pub(crate) fn array_default(&self, vtype: VariableType, rank: u8, bounds: [usize; 3]) -> Res<VariableValue> {
        let element = self.unbound_type_default(vtype)?;
        let mut value = VariableValue {
            vtype,
            data: element.data,
            generic_data: GenericVariableData::create_array(element, rank, bounds[0], bounds[1], bounds[2])
                .ok_or(crate::executable::VMError::GenericDataNotSet)?,
        };
        bind_host_defaults(&mut value, self.type_registry)?;
        Ok(value)
    }

    pub(crate) fn local_default(&self, entry: &TableEntry) -> Res<VariableValue> {
        if self.variable_table.get_version() < 400 {
            return Ok(entry.value.emptied());
        }
        if entry.header.flags & VARIABLE_FLAG_DYNAMIC_ARRAY != 0 {
            return Ok(VariableValue {
                vtype: entry.value.vtype,
                data: entry.value.data,
                generic_data: entry.header.create_generic_data().ok_or(VMError::InternalVMError)?,
            });
        }
        self.reset_value(&entry.value)
    }

    fn reset_value(&self, value: &VariableValue) -> Res<VariableValue> {
        let generic_data = match &value.generic_data {
            GenericVariableData::Dim1(values) => {
                GenericVariableData::Dim1(Arc::new(values.iter().map(|value| self.reset_value(value)).collect::<Res<Vec<_>>>()?))
            }
            GenericVariableData::Dim2(values) => GenericVariableData::Dim2(Arc::new(
                values
                    .iter()
                    .map(|row| row.iter().map(|value| self.reset_value(value)).collect::<Res<Vec<_>>>())
                    .collect::<Res<Vec<_>>>()?,
            )),
            GenericVariableData::Dim3(values) => GenericVariableData::Dim3(Arc::new(
                values
                    .iter()
                    .map(|plane| {
                        plane
                            .iter()
                            .map(|row| row.iter().map(|value| self.reset_value(value)).collect::<Res<Vec<_>>>())
                            .collect::<Res<Vec<_>>>()
                    })
                    .collect::<Res<Vec<_>>>()?,
            )),
            _ => return self.type_default(value.vtype),
        };
        Ok(VariableValue {
            vtype: value.vtype,
            data: value.data,
            generic_data,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        compiler::user_data::runtime_object,
        executable::{PPEExpr, RecordField, VariableData, variable_table::VARIABLE_FLAG_STATIC},
        icy_board::{IcyBoard, bbs::BBS, state::IcyBoardState},
        parser::{self, board_catalog::TYPES},
        vm::{DiskIO, expressions::predefined_functions::array_value_at},
    };
    use icy_net::{ConnectionType, channel::ChannelConnection};

    async fn state() -> IcyBoardState {
        let bbs = Arc::new(tokio::sync::Mutex::new(BBS::new(1)));
        let node = bbs.lock().await.create_new_node(ConnectionType::Channel).await;
        let nodes = bbs.lock().await.open_connections.clone();
        let (_peer, connection) = ChannelConnection::create_pair();
        IcyBoardState::new(bbs, Arc::new(tokio::sync::Mutex::new(IcyBoard::new())), nodes, node, Box::new(connection)).await
    }

    fn fields(value: &VariableValue) -> &[VariableValue] {
        let GenericVariableData::Record(fields) = &value.generic_data else {
            panic!("expected record: {value:?}")
        };
        fields
    }

    fn length(value: &VariableValue) -> usize {
        match &value.generic_data {
            GenericVariableData::Dim1(values) => values.len(),
            GenericVariableData::Dim2(values) => values.len(),
            GenericVariableData::Dim3(values) => values.len(),
            _ => panic!("expected array: {value:?}"),
        }
    }

    fn object(value: &VariableValue) -> &Arc<dyn std::any::Any + Send + Sync> {
        let GenericVariableData::UserData(object) = &value.generic_data else {
            panic!("expected host: {value:?}")
        };
        object
    }

    fn push(vm: &mut VirtualMachine<'_>, value: VariableValue, flags: u8) -> usize {
        let mut entry = TableEntry::default();
        entry.header.id = vm.variable_table.get_entries().len() + 1;
        entry.header.variable_type = value.vtype;
        entry.header.dim = value.get_dimensions();
        entry.header.flags = flags;
        entry.value = value;
        let id = entry.header.id;
        vm.variable_table.push(entry);
        id
    }

    #[test]
    fn s1_all_catalog_placeholders_bind_without_replacing_live_objects() {
        let registry = parser::icy_board_registry();
        assert_eq!(24, TYPES.len());
        for &(id, name, _) in TYPES {
            for rank in 1..=3 {
                let placeholder = VariableType::UserData(id as u32).create_empty_value();
                let mut array = VariableValue {
                    vtype: placeholder.vtype,
                    data: VariableData::default(),
                    generic_data: GenericVariableData::create_array(placeholder, rank, 1, 1, 1).unwrap(),
                };
                bind_host_defaults(&mut array, &registry).unwrap();
                let first = array.get_array_value(0, 0, 0);
                let second = array.get_array_value(1, 0, 0);
                assert_eq!(VariableType::UserData(id as u32), first.vtype, "{name}");
                assert!(runtime_object(object(&first).as_ref(), id as u32).is_ok(), "{name}");
                assert!(!Arc::ptr_eq(object(&first), object(&second)), "{name} rank {rank}");
                let original = array.clone();
                bind_host_defaults(&mut array, &registry).unwrap();
                assert!(Arc::ptr_eq(object(&first), object(&array.get_array_value(0, 0, 0))), "{name}");
                assert!(Arc::ptr_eq(object(&first), object(&original.get_array_value(0, 0, 0))), "{name}");
            }
        }
    }

    #[tokio::test]
    async fn s1_record_defaults_rebuild_nested_shapes_and_enum_defaults() {
        let mut state = state().await;
        let registry = parser::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        let mut vm = VirtualMachine::new("defaults.ppe".into(), &registry, &mut io, &mut state);
        vm.variable_table.set_version(400);
        vm.variable_table.enums.insert(102, vec![7, 9]);
        for rank in 1..=3 {
            vm.user_types = vec![
                vec![
                    RecordField::scalar(VariableType::UserData(parser::AUDIO_ID as u32)),
                    RecordField {
                        dim: rank,
                        is_dynamic: true,
                        ..RecordField::scalar(VariableType::Integer)
                    },
                    RecordField {
                        dim: rank,
                        vector_size: 1,
                        matrix_size: u16::from(rank >= 2),
                        cube_size: u16::from(rank >= 3),
                        ..RecordField::scalar(VariableType::UserData(102))
                    },
                ],
                vec![RecordField::scalar(VariableType::UserData(100))],
            ];
            let mut previous = vm.type_default(VariableType::UserData(101)).unwrap();
            let GenericVariableData::Record(outer) = &mut previous.generic_data else {
                unreachable!()
            };
            let GenericVariableData::Record(inner) = &mut Arc::make_mut(outer)[0].generic_data else {
                unreachable!()
            };
            Arc::make_mut(inner)[1].redim(rank, 3, 3, 3);
            let id = push(&mut vm, previous, 0);
            let fresh = vm.local_default(vm.variable_table.get_var_entry(id)).unwrap();
            let inner = fields(&fields(&fresh)[0]);
            assert_eq!(0, length(&inner[1]));
            assert_eq!(rank, inner[1].get_dimensions());
            assert_eq!(2, length(&inner[2]));
            assert_eq!(rank, inner[2].get_dimensions());
            assert_eq!(7, inner[2].get_array_value(1, usize::from(rank >= 2), usize::from(rank >= 3)).as_int());
            assert_eq!(inner[0], vm.type_default(inner[0].vtype).unwrap());
            assert_eq!(4, length(&fields(&fields(vm.variable_table.get_value(id))[0])[1]));
        }
        let contact = vm.type_default(VariableType::UserData(parser::CONTACT_ID as u32)).unwrap();
        assert_eq!(2, fields(&contact).len());
        assert!(
            fields(&contact)
                .iter()
                .all(|field| field.vtype == VariableType::UnboundedString && field.as_string().is_empty())
        );
        vm.user_types = vec![vec![RecordField::scalar(VariableType::UserData(100))]];
        assert!(vm.type_default(VariableType::UserData(100)).is_err());
    }

    #[tokio::test]
    async fn s1_array_defaults_are_fresh_and_at_uses_the_executable_layout() {
        let mut state = state().await;
        let registry = parser::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        let mut vm = VirtualMachine::new("defaults.ppe".into(), &registry, &mut io, &mut state);
        vm.variable_table.set_version(400);
        vm.user_types = vec![vec![RecordField {
            dim: 2,
            vector_size: 1,
            ..RecordField::scalar(VariableType::UserData(parser::HTTP_REQUEST_ID as u32))
        }]];
        let array = vm.array_default(VariableType::UserData(100), 1, [1, 0, 0]).unwrap();
        let first = array.get_array_value(0, 0, 0);
        let second = array.get_array_value(1, 0, 0);
        assert!(!Arc::ptr_eq(
            object(&fields(&first)[0].get_array_value(0, 0, 0)),
            object(&fields(&second)[0].get_array_value(0, 0, 0)),
        ));
        let array_id = push(&mut vm, array, 0);
        let index_id = push(&mut vm, VariableValue::new_int(-1), 0);
        let missing = array_value_at(&mut vm, &[PPEExpr::Value(array_id), PPEExpr::Value(index_id)]).await.unwrap();
        assert_eq!(2, fields(&missing)[0].get_dimensions());
        assert_eq!(2, length(&fields(&missing)[0]));
        assert_eq!(0, fields(&missing)[0].get_matrix_size());
        assert!(runtime_object(object(&fields(&missing)[0].get_array_value(0, 0, 0)).as_ref(), parser::HTTP_REQUEST_ID as u32).is_ok());
    }

    #[tokio::test]
    async fn s1_review_array_reads_preserve_existing_hosts_and_create_fresh_missing_values() {
        let mut state = state().await;
        let registry = parser::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        let mut vm = VirtualMachine::new("read-defaults.ppe".into(), &registry, &mut io, &mut state);
        vm.variable_table.set_version(400);
        for &(id, name, _) in TYPES {
            let vtype = VariableType::UserData(id as u32);
            for rank in 1..=3 {
                let array = vm.array_default(vtype, rank, [0; 3]).unwrap();
                let stored = array.get_array_value(0, 0, 0);
                let read = vm.read_array_element(&array, 0, 0, 0).unwrap();
                assert!(Arc::ptr_eq(object(&stored), object(&read)), "{name} rank {rank}");
                let first_missing = vm.read_array_element(&array, 1, 1, 1).unwrap();
                let second_missing = vm.read_array_element(&array, 1, 1, 1).unwrap();
                assert_eq!(first_missing.vtype, vtype);
                assert!(runtime_object(object(&first_missing).as_ref(), id as u32).is_ok());
                assert!(!Arc::ptr_eq(object(&stored), object(&first_missing)), "{name} rank {rank}");
                assert!(!Arc::ptr_eq(object(&first_missing), object(&second_missing)), "{name} rank {rank}");
            }
        }
    }

    #[tokio::test]
    async fn s1_review_array_reads_keep_enum_defaults_and_legacy_fallbacks() {
        let mut state = state().await;
        let registry = parser::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        let mut vm = VirtualMachine::new("read-defaults.ppe".into(), &registry, &mut io, &mut state);
        vm.variable_table.enums.insert(100, vec![7, 9]);
        for rank in 1..=3 {
            for version in [340, 400] {
                vm.variable_table.set_version(version);
                let vtype = VariableType::UserData(100);
                let array = vm.array_default(vtype, rank, [0; 3]).unwrap();
                let value = vm.read_array_element(&array, usize::MAX, usize::MAX, usize::MAX).unwrap();
                assert_eq!(value, VariableValue::new_enum(vtype, 7, 7));
                let array = vm.array_default(VariableType::Integer, rank, [0; 3]).unwrap();
                assert_eq!(vm.read_array_element(&array, 1, 1, 1).unwrap(), vm.variable_table.array_value(&array, 1, 1, 1));
            }
            vm.variable_table.set_version(340);
            let array = vm.array_default(VariableType::UserData(parser::SURFACE_ID as u32), rank, [0; 3]).unwrap();
            let missing = vm.read_array_element(&array, 1, 1, 1).unwrap();
            assert!(matches!(missing.generic_data, GenericVariableData::None));
        }
    }

    #[tokio::test]
    async fn s1_call_reset_preserves_static_slots_and_legacy_parameter_tails() {
        let mut state = state().await;
        let registry = parser::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        let mut vm = VirtualMachine::new("defaults.ppe".into(), &registry, &mut io, &mut state);
        vm.variable_table.set_version(400);
        push(
            &mut vm,
            VariableValue::new_vector(VariableType::Integer, vec![VariableValue::new_int(4), VariableValue::new_int(9)]),
            0,
        );
        let dynamic = push(
            &mut vm,
            VariableValue::new_vector(VariableType::Integer, vec![VariableValue::new_int(8)]),
            VARIABLE_FLAG_DYNAMIC_ARRAY,
        );
        let static_id = push(&mut vm, VariableValue::new_int(42), VARIABLE_FLAG_STATIC);
        let fixed = push(
            &mut vm,
            VariableValue::new_vector(VariableType::Integer, vec![VariableValue::new_int(6), VariableValue::new_int(7)]),
            0,
        );
        let audio = push(&mut vm, crate::icy_board::state::ppl_audio::PplAudio::value(3), 0);
        vm.prepare_call_with_values(4, 1, 1, vec![VariableValue::new_int(2)]).unwrap();
        assert_eq!(2, vm.variable_table.get_value(1).get_array_value(0, 0, 0).as_int());
        assert_eq!(9, vm.variable_table.get_value(1).get_array_value(1, 0, 0).as_int());
        assert_eq!(0, length(vm.variable_table.get_value(dynamic)));
        assert_eq!(42, vm.variable_table.get_value(static_id).as_int());
        assert_eq!(2, length(vm.variable_table.get_value(fixed)));
        assert_eq!(0, vm.variable_table.get_value(fixed).get_array_value(1, 0, 0).as_int());
        assert_eq!(*vm.variable_table.get_value(audio), crate::icy_board::state::ppl_audio::PplAudio::invalid());
        assert_eq!(4, vm.call_local_value_stack.len());
        assert_eq!(4, vm.call_local_value_stack[0].as_int());
    }

    #[tokio::test]
    async fn s1_catalog_default_properties_are_typed_and_do_not_read_the_live_session() {
        use crate::icy_board::state::ppl_error::{ERR_INVALID, ERR_KIND_FILE, PplError};

        let mut state = state().await;
        state.session.user_name = "LIVE SYSOP".into();
        state.session.cur_security = 255;
        state.session.is_sysop = true;
        let registry = parser::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        let mut vm = VirtualMachine::new("defaults.ppe".into(), &registry, &mut io, &mut state);
        vm.variable_table.enums = registry.enums().into_iter().map(|definition| (definition.id, definition.domain)).collect();
        let sentinel = PplError::new(ERR_KIND_FILE, ERR_INVALID, "earlier failure");
        vm.set_error(sentinel.clone());
        for &(id, type_name, _) in TYPES {
            let value = vm.type_default(VariableType::UserData(id as u32)).unwrap();
            let receiver = runtime_object(object(&value).as_ref(), id as u32).unwrap();
            let members = registry.get_type_from_id(id as u32).unwrap();
            for (name, &vtype) in &members.fields {
                let property = receiver.get_property_value(&vm, name).unwrap();
                let property = vm.variable_table.checked_enum_value(vtype, property).unwrap();
                assert_eq!(vtype, property.vtype, "{type_name}.{name}");
                assert_eq!(
                    members.field_ranks.get(name).copied().unwrap_or_default(),
                    property.get_dimensions(),
                    "{type_name}.{name}"
                );
                if id == parser::SESSION_ID && property.get_dimensions() == 0 {
                    match vtype {
                        VariableType::UnboundedString => assert!(property.as_string().is_empty(), "{name}"),
                        VariableType::Boolean => assert!(!property.as_bool(), "{name}"),
                        VariableType::Integer => assert_eq!(0, property.as_int(), "{name}"),
                        _ => {}
                    }
                }
            }
            assert_eq!(sentinel, vm.last_error, "{type_name}");
        }
        assert!(vm.board_value.is_none());
    }

    #[tokio::test]
    async fn s1_inert_facades_fail_operations_without_live_effects() {
        use crate::icy_board::state::ppl_error::{ERR_INVALID, ERR_KIND_GFX, ERR_KIND_NET, ERR_KIND_TERM, ERR_KIND_USER};

        let mut state = state().await;
        let registry = parser::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        let mut vm = VirtualMachine::new("defaults.ppe".into(), &registry, &mut io, &mut state);
        for (id, kind) in [
            (parser::TERM_INPUT_ID, ERR_KIND_TERM),
            (parser::TERMINAL_ID, ERR_KIND_TERM),
            (parser::GFX_ID, ERR_KIND_GFX),
            (parser::MARGINS_ID, ERR_KIND_TERM),
            (parser::PALETTE_ID, ERR_KIND_TERM),
            (parser::MACROS_ID, ERR_KIND_TERM),
            (parser::SESSION_ID, ERR_KIND_USER),
            (parser::HTTP_ID, ERR_KIND_NET),
        ] {
            let value = vm.type_default(VariableType::UserData(id as u32)).unwrap();
            let receiver = runtime_object(object(&value).as_ref(), id as u32).unwrap();
            for (name, function) in &registry.get_type_from_id(id as u32).unwrap().functions {
                vm.clear_error();
                let result = receiver.call_function(&mut vm, name, &[]).await.unwrap();
                assert_eq!(function.return_type, result.vtype, "{id}.{name}");
                assert_eq!(function.return_rank, result.get_dimensions(), "{id}.{name}");
                assert_eq!(ERR_INVALID, vm.last_error.code, "{id}.{name}");
                assert!(vm.error_pending, "{id}.{name}");
                let kind = if id == parser::TERMINAL_ID && ["SetFont", "LoadFont"].contains(&name.as_str()) {
                    crate::icy_board::state::ppl_error::ERR_KIND_FONT
                } else {
                    kind
                };
                assert_eq!(kind, vm.last_error.kind, "{id}.{name}");
                if result.vtype == VariableType::Boolean {
                    assert!(!result.as_bool(), "{id}.{name}");
                }
            }
        }
        assert!(vm.board_value.is_none());
        assert!(vm.icy_board_state.ppl_graphics.is_none());
        assert!(!vm.icy_board_state.ppl_terminal.is_recording());
        assert_eq!(0, vm.icy_board_state.ppl_terminal.take_update_depth());
    }

    #[tokio::test]
    async fn s1_invalid_native_objects_do_not_grant_access_or_open_message_files() {
        use crate::icy_board::state::ppl_error::{ERR_INVALID, ERR_KIND_MSG};

        let mut state = state().await;
        state.session.cur_security = 255;
        state.session.is_sysop = true;
        let registry = parser::icy_board_registry();
        let mut io = DiskIO::new(".", None);
        let mut vm = VirtualMachine::new("defaults.ppe".into(), &registry, &mut io, &mut state);
        for id in [parser::CONFERENCE_ID, parser::MESSAGE_AREA_ID, parser::FILE_DIRECTORY_ID, parser::DOOR_ID] {
            let value = vm.type_default(VariableType::UserData(id as u32)).unwrap();
            let receiver = runtime_object(object(&value).as_ref(), id as u32).unwrap();
            for (name, function) in &registry.get_type_from_id(id as u32).unwrap().functions {
                let result = receiver.call_function(&mut vm, name, &[]).await.unwrap();
                if function.return_type == VariableType::Boolean {
                    assert!(!result.as_bool(), "{id}.{name}");
                } else {
                    assert_eq!(ERR_KIND_MSG, vm.last_error.kind, "{id}.{name}");
                    assert_eq!(ERR_INVALID, vm.last_error.code, "{id}.{name}");
                }
            }
        }
        assert!(vm.message_base.is_none());
    }
}
