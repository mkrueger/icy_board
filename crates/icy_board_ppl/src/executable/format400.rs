use sha2::{Digest, Sha256};
use std::collections::HashSet;

use crate::parser::{FIRST_USER_TYPE_ID, MAX_TYPE_FIELDS, MAX_USER_TYPES, UserTypeRegistry};

use super::code400::{self, MAX_ITEMS, Reader, Result, blob, word};
use super::container::{Compression, Container, ContainerError, LoadLimits, REQUIRED, Section};
use super::imports400::HostCatalog;
use super::{
    EntryType, Executable, FunctionValue, GenericVariableData, PPECommand, PPEExpr, PPEScript, ProcedureValue, RecordField, TableEntry, VarHeader,
    VariableData, VariableTable, VariableType,
};

fn invalid(message: &'static str) -> ContainerError {
    ContainerError::Invalid(message)
}

/// The sections that make up the executable program itself.
const REQUIRED_SECTIONS: [[u8; 4]; 6] = [*b"TYPE", *b"CONS", *b"VARS", *b"ROUT", *b"IMPT", *b"CODE"];

fn variable_type(id: u32) -> Result<VariableType> {
    match id {
        0..=24 => Ok(VariableType::from(id as u8)),
        30..=0x7fff_ffff => Ok(VariableType::UserData(id)),
        _ => Err(invalid("type reference")),
    }
}

fn write_constant(output: &mut Vec<u8>, entry: &TableEntry) -> Result<()> {
    word(output, u32::from(entry.header.variable_type) as usize)?;
    match &entry.value.generic_data {
        GenericVariableData::String(text) => {
            word(output, 2)?;
            blob(output, text.as_bytes())?;
        }
        GenericVariableData::Bytes(bytes) => {
            word(output, 3)?;
            blob(output, bytes)?;
        }
        GenericVariableData::None | GenericVariableData::Enum(_) => {
            let value = &entry.value;
            let payload = match value.vtype {
                VariableType::Boolean => u64::from(value.as_bool()),
                VariableType::Unsigned => value.as_unsigned(),
                VariableType::Long => value.as_long() as u64,
                VariableType::ULong => value.as_ulong(),
                VariableType::Float => u64::from(value.as_float().to_bits()),
                VariableType::Double => value.as_double().to_bits(),
                VariableType::Byte | VariableType::SByte => u64::from(value.as_byte()),
                VariableType::Word | VariableType::SWord => u64::from(value.as_word()),
                VariableType::Date
                | VariableType::EDate
                | VariableType::DDate
                | VariableType::Integer
                | VariableType::Money
                | VariableType::Time
                | VariableType::UserData(_) => u64::from(value.as_int() as u32),
                VariableType::MessageAreaID => {
                    let (conference, area) = value.as_msg_id();
                    u64::from(conference as u32) | (u64::from(area as u32) << 32)
                }
                _ => return Err(invalid("scalar constant type")),
            };
            word(output, 1)?;
            blob(output, &payload.to_le_bytes())?;
        }
        _ => return Err(invalid("live object or aggregate constant")),
    }
    Ok(())
}

pub(super) fn encode(executable: &Executable, compression: Compression, debug_names: bool, limits: &LoadLimits) -> Result<Vec<u8>> {
    if executable.user_types.len() > MAX_USER_TYPES {
        return Err(ContainerError::Limit("record count"));
    }
    if executable.user_types.iter().any(|fields| fields.is_empty() || fields.len() > MAX_TYPE_FIELDS) {
        return Err(ContainerError::Limit("record fields"));
    }
    let mut script = PPEScript::from_ppe_file(executable).map_err(|_| invalid("invalid legacy code"))?;
    for statement in &mut script.statements {
        statement.command.normalize_control();
    }
    let catalog = executable
        .variable_table
        .host_catalog
        .clone()
        .unwrap_or_else(|| HostCatalog::from_registry(&UserTypeRegistry::icy_board_registry()));
    let (used, _) = catalog.rewrite(
        &mut script,
        &executable.variable_table,
        &executable.user_types,
        &Default::default(),
        &Default::default(),
    )?;
    let (code, addresses) = code400::encode(&script)?;
    let mut types = Vec::new();
    for (index, fields) in executable.user_types.iter().enumerate() {
        word(&mut types, 1)?;
        word(&mut types, FIRST_USER_TYPE_ID + index)?;
        word(&mut types, fields.len())?;
        for field in fields {
            word(&mut types, u32::from(field.variable_type) as usize)?;
            word(&mut types, field.dim as usize)?;
            word(&mut types, usize::from(field.is_dynamic))?;
            for bound in [field.vector_size, field.matrix_size, field.cube_size] {
                word(&mut types, bound as usize)?;
            }
        }
    }
    for (&id, values) in &executable.variable_table.enums {
        word(&mut types, 2)?;
        word(&mut types, id as usize)?;
        word(&mut types, values.len())?;
        for value in values {
            types.extend_from_slice(&value.to_le_bytes());
        }
    }
    let mut constants = Vec::new();
    let mut variables = Vec::new();
    let mut routines = Vec::new();
    let mut debug = Vec::new();
    let mut constant_count = 0;
    let mut routine_count = 0;
    for (index, entry) in executable.variable_table.get_entries().iter().enumerate() {
        if entry.header.id != index + 1 {
            return Err(invalid("variable ordering"));
        }
        let header = &entry.header;
        let routine = matches!(header.variable_type, VariableType::Function | VariableType::Procedure);
        word(&mut variables, u32::from(header.variable_type) as usize)?;
        word(&mut variables, header.dim as usize)?;
        word(&mut variables, header.flags as usize)?;
        for bound in [header.vector_size, header.matrix_size, header.cube_size] {
            word(&mut variables, bound)?;
        }
        word(&mut variables, entry.entry_type as usize)?;
        word(&mut variables, entry.function_id)?;
        let host_default = matches!(header.variable_type, VariableType::UserData(id) if !executable.variable_table.enums.contains_key(&id));
        let constant = header.dim == 0
            && !routine
            && !host_default
            && !matches!(
                entry.value.generic_data,
                GenericVariableData::Record(_) | GenericVariableData::UserData(_) | GenericVariableData::Table(_) | GenericVariableData::Password(_)
            );
        if constant {
            write_constant(&mut constants, entry)?;
            constant_count += 1;
            word(&mut variables, constant_count)?;
        } else {
            word(&mut variables, 0)?;
        }
        if routine {
            routine_count += 1;
            let (parameters, locals, start, first, result, _flags) = unsafe {
                if header.variable_type == VariableType::Function {
                    let function = entry.value.data.function_value;
                    (
                        function.parameters,
                        function.local_variables,
                        function.start_offset,
                        function.first_var_id,
                        function.return_var as usize,
                        0,
                    )
                } else {
                    let procedure = entry.value.data.procedure_value;
                    (
                        procedure.parameters,
                        procedure.local_variables,
                        procedure.start_offset,
                        procedure.first_var_id,
                        0,
                        procedure.pass_flags,
                    )
                }
            };
            word(&mut routines, header.id)?;
            word(&mut routines, parameters as usize)?;
            word(&mut routines, locals as usize)?;
            word(
                &mut routines,
                if entry.entry_type == EntryType::Parameter {
                    u32::MAX as usize
                } else {
                    *addresses.get(&(start as usize)).ok_or(invalid("routine entry address"))? as usize
                },
            )?;
            word(&mut routines, first as usize)?;
            word(&mut routines, result)?;
            for parameter in 0..parameters {
                word(
                    &mut routines,
                    usize::from(executable.variable_table.is_var_parameter(header.id, parameter as usize)),
                )?;
            }
        }
        blob(&mut debug, entry.name.as_bytes())?;
    }
    let names = executable.debug_info.clone().unwrap_or_default();
    // Structure always matches the program; only the names themselves are optional.
    word(&mut debug, executable.user_types.len())?;
    for (index, fields) in executable.user_types.iter().enumerate() {
        let record = names.records.get(index);
        blob(&mut debug, record.map_or("", |(name, _)| name.as_str()).as_bytes())?;
        word(&mut debug, fields.len())?;
        for field in 0..fields.len() {
            let name = record.and_then(|(_, names)| names.get(field)).map_or("", String::as_str);
            blob(&mut debug, name.as_bytes())?;
        }
    }
    word(&mut debug, executable.variable_table.enums.len())?;
    for (id, values) in &executable.variable_table.enums {
        let definition = names.enums.get(id);
        word(&mut debug, *id as usize)?;
        blob(&mut debug, definition.map_or("", |(name, _)| name.as_str()).as_bytes())?;
        word(&mut debug, values.len())?;
        for member in 0..values.len() {
            let name = definition.and_then(|(_, names)| names.get(member)).map_or("", String::as_str);
            blob(&mut debug, name.as_bytes())?;
        }
    }
    let mut debug = Section::new(*b"DBUG", executable.variable_table.len() as u32, debug);
    debug.flags = 0;
    let mut container = Container {
        runtime: 400,
        entry_routine: 0,
        sections: vec![
            Section::new(*b"TYPE", (executable.user_types.len() + executable.variable_table.enums.len()) as u32, types),
            Section::new(*b"CONS", constant_count as u32, constants),
            Section::new(*b"VARS", executable.variable_table.len() as u32, variables),
            Section::new(*b"ROUT", routine_count, routines),
            catalog.encode(&used)?,
            code,
        ],
    };
    let mut identity = Section::new(*b"IDEN", 1, content_identity(&container).to_vec());
    identity.flags = 0;
    container.sections.push(identity);
    for section in &executable.extra_sections {
        if REQUIRED_SECTIONS.contains(&section.kind) || section.kind == *b"IDEN" || section.kind == *b"DBUG" {
            return Err(invalid("preserved section kind"));
        }
        container.sections.push(section.clone());
    }
    if debug_names {
        container.sections.push(debug);
    }
    let bytes = container.encode(compression, limits)?;
    decode(&bytes, limits)?;
    Ok(bytes)
}

fn content_identity(container: &Container) -> [u8; 32] {
    let mut digest = Sha256::new();
    digest.update(container.runtime.to_le_bytes());
    digest.update(container.entry_routine.to_le_bytes());
    // The identity covers the program. Optional sections stay outside it so that
    // stripping debug data, repacking or adding a future section keeps it valid.
    let mut sections: Vec<_> = container.sections.iter().filter(|section| REQUIRED_SECTIONS.contains(&section.kind)).collect();
    sections.sort_by_key(|section| section.kind);
    for section in sections {
        digest.update(section.kind);
        digest.update(section.schema.to_le_bytes());
        digest.update(section.flags.to_le_bytes());
        digest.update(section.entries.to_le_bytes());
        digest.update((section.data.len() as u64).to_le_bytes());
        digest.update(&section.data);
    }
    digest.finalize().into()
}

pub(super) fn decode(bytes: &[u8], limits: &LoadLimits) -> Result<Executable> {
    decode_with_registry(bytes, limits, &UserTypeRegistry::icy_board_registry())
}

pub(super) fn decode_with_registry(bytes: &[u8], limits: &LoadLimits, registry: &UserTypeRegistry) -> Result<Executable> {
    let container = Container::decode(bytes, limits)?;
    if let Some(identity) = container.sections.iter().find(|section| section.kind == *b"IDEN") {
        if identity.entries != 1 || identity.data.as_slice() != content_identity(&container) {
            return Err(invalid("content identity mismatch"));
        }
    }
    if container.entry_routine != 0 {
        return Err(invalid("entry routine"));
    }
    let section = |kind: &[u8; 4]| {
        container
            .sections
            .iter()
            .find(|section| &section.kind == kind && section.flags == REQUIRED)
            .ok_or(invalid("missing required section"))
    };
    for item in &container.sections {
        if item.flags & REQUIRED != 0 && !REQUIRED_SECTIONS.contains(&item.kind) {
            return Err(invalid("unsupported required section semantics"));
        }
        if item.entries as usize > MAX_ITEMS {
            return Err(ContainerError::Limit("section entries"));
        }
    }
    let extra_sections: Vec<_> = container
        .sections
        .iter()
        .filter(|section| !REQUIRED_SECTIONS.contains(&section.kind) && section.kind != *b"IDEN" && section.kind != *b"DBUG")
        .cloned()
        .collect();
    let mut table = VariableTable::default();
    table.set_version(400);
    let imports = HostCatalog::decode(section(b"IMPT")?)?;
    let current_catalog = HostCatalog::from_registry(registry);
    let (mut type_remap, member_remap) = imports.bind(&current_catalog)?;
    let mut user_types = Vec::new();
    let type_section = section(b"TYPE")?;
    let mut input = Reader::new(&type_section.data);
    let mut type_ids = HashSet::new();
    for _ in 0..type_section.entries {
        let kind = input.word()?;
        let id = input.word()?;
        if !type_ids.insert(id) {
            return Err(invalid("duplicate type"));
        }
        match kind {
            1 => {
                if id as usize != FIRST_USER_TYPE_ID + user_types.len() || user_types.len() >= MAX_USER_TYPES {
                    return Err(invalid("record id"));
                }
                let count = input.count(24)?;
                if count == 0 || count > MAX_TYPE_FIELDS {
                    return Err(ContainerError::Limit("record fields"));
                }
                let mut fields = Vec::new();
                for _ in 0..count {
                    let variable_type = variable_type(input.word()?)?;
                    let dim = u8::try_from(input.word()?).map_err(|_| invalid("field rank"))?;
                    let flags = input.word()?;
                    if flags > 1 {
                        return Err(invalid("field flags"));
                    }
                    let field = RecordField {
                        variable_type,
                        dim,
                        is_dynamic: flags == 1,
                        vector_size: input.word()? as usize,
                        matrix_size: input.word()? as usize,
                        cube_size: input.word()? as usize,
                    };
                    if field.element_count().is_none() {
                        return Err(invalid("field shape"));
                    }
                    fields.push(field);
                }
                user_types.push(fields);
            }
            2 => {
                let count = input.count(4)?;
                if count == 0 {
                    return Err(invalid("enum default"));
                }
                let values = (0..count).map(|_| input.word().map(|value| value as i32)).collect::<Result<Vec<_>>>()?;
                table.enums.insert(id, values);
            }
            _ => return Err(invalid("type kind")),
        }
    }
    input.finish()?;
    let known_type = |typ: VariableType| match typ {
        VariableType::UserData(id) => type_ids.contains(&id) || imports.types.contains_key(&id),
        _ => true,
    };
    let mut footprints = Vec::new();
    let mut depths = Vec::new();
    for (index, fields) in user_types.iter().enumerate() {
        let mut footprint = 1usize;
        let mut depth = 1usize;
        for field in fields {
            if !known_type(field.variable_type) {
                return Err(invalid("field type"));
            }
            let mut elements = field.element_count().ok_or(invalid("field shape"))?;
            if let VariableType::UserData(id) = field.variable_type {
                if id as usize >= FIRST_USER_TYPE_ID && !table.enums.contains_key(&id) && !imports.types.contains_key(&id) {
                    let previous = id as usize - FIRST_USER_TYPE_ID;
                    if previous >= index {
                        return Err(invalid("recursive or forward record"));
                    }
                    elements = elements.checked_mul(footprints[previous]).ok_or(ContainerError::Limit("record allocation"))?;
                    depth = depth.max(depths[previous] + 1);
                }
            }
            footprint = footprint.checked_add(elements).ok_or(ContainerError::Limit("record allocation"))?;
        }
        if footprint > MAX_ITEMS || depth > code400::MAX_DEPTH {
            return Err(ContainerError::Limit("record allocation or depth"));
        }
        footprints.push(footprint);
        depths.push(depth);
    }
    let constant_section = section(b"CONS")?;
    let mut input = Reader::new(&constant_section.data);
    let mut constants = Vec::new();
    for _ in 0..constant_section.entries {
        let typ = variable_type(input.word()?)?;
        let tag = input.word()?;
        let payload = input.blob()?;
        if !known_type(typ) {
            return Err(invalid("constant type"));
        }
        let mut value = typ.create_empty_value();
        match tag {
            1 if payload.len() == 8
                && (!matches!(
                    typ,
                    VariableType::UserData(_)
                        | VariableType::Function
                        | VariableType::Procedure
                        | VariableType::Table
                        | VariableType::Password
                        | VariableType::String
                        | VariableType::BigStr
                        | VariableType::UnboundedString
                        | VariableType::Bytes
                ) || matches!(typ, VariableType::UserData(id) if table.enums.contains_key(&id))) =>
            {
                value.data = VariableData::default();
                value.data.u64_value = u64::from_le_bytes(payload.try_into().unwrap());
            }
            2 if matches!(typ, VariableType::String | VariableType::BigStr | VariableType::UnboundedString) => {
                value.generic_data = GenericVariableData::String(std::sync::Arc::new(
                    std::str::from_utf8(payload).map_err(|_| invalid("invalid UTF-8 constant"))?.to_string(),
                ));
            }
            3 if typ == VariableType::Bytes => value.generic_data = GenericVariableData::Bytes(payload.to_vec()),
            _ => return Err(invalid("constant representation")),
        }
        constants.push(value);
    }
    input.finish()?;
    let variable_section = section(b"VARS")?;
    let mut input = Reader::new(&variable_section.data);
    let mut allocation = 0usize;
    for index in 0..variable_section.entries as usize {
        let typ = variable_type(input.word()?)?;
        if !known_type(typ) {
            return Err(invalid("variable type"));
        }
        let dim = input.word()?;
        let flags = input.word()?;
        if dim > 3 || flags & !7 != 0 || (flags & 2 != 0 && dim == 0) {
            return Err(invalid("variable rank or flags"));
        }
        let header = VarHeader {
            id: index + 1,
            variable_type: typ,
            dim: dim as u8,
            flags: flags as u8,
            vector_size: input.word()? as usize,
            matrix_size: input.word()? as usize,
            cube_size: input.word()? as usize,
        };
        let entry_type = match input.word()? {
            0 => EntryType::Constant,
            1 => EntryType::UserVariable,
            2 => EntryType::Variable,
            3 => EntryType::LocalVariable,
            4 => EntryType::FunctionResult,
            5 => EntryType::Parameter,
            6 => EntryType::Function,
            7 => EntryType::Procedure,
            _ => return Err(invalid("variable storage kind")),
        };
        let function_id = input.word()? as usize;
        let constant = input.word()? as usize;
        let mut value = if constant == 0 {
            typ.create_empty_value()
        } else {
            let value = constants.get(constant - 1).ok_or(invalid("constant reference"))?;
            if value.vtype != typ || dim != 0 {
                return Err(invalid("constant assignment"));
            }
            value.clone()
        };
        let elements = header.allocated_elements().ok_or(ContainerError::Limit("array allocation"))?.max(1);
        let footprint = match typ {
            VariableType::UserData(id) if id as usize >= FIRST_USER_TYPE_ID && !table.enums.contains_key(&id) && !imports.types.contains_key(&id) => {
                *footprints.get(id as usize - FIRST_USER_TYPE_ID).ok_or(invalid("record reference"))?
            }
            _ => 1,
        };
        allocation = allocation
            .checked_add(elements.checked_mul(footprint).ok_or(ContainerError::Limit("array allocation"))?)
            .ok_or(ContainerError::Limit("array allocation"))?;
        if allocation > MAX_ITEMS {
            return Err(ContainerError::Limit("total value allocation"));
        }
        if dim != 0 && !matches!(typ, VariableType::Function | VariableType::Procedure) {
            value.generic_data = header.create_generic_data().ok_or(invalid("array shape"))?;
        }
        let mut entry = TableEntry::new(String::new(), header, value, entry_type);
        entry.function_id = function_id;
        table.push(entry);
    }
    input.finish()?;
    let mut script = code400::decode(section(b"CODE")?)?;
    let routine_section = section(b"ROUT")?;
    let mut input = Reader::new(&routine_section.data);
    let mut routine_ids = HashSet::new();
    for _ in 0..routine_section.entries {
        let id = input.word()? as usize;
        if !routine_ids.insert(id) {
            return Err(invalid("duplicate routine"));
        }
        let parameters = input.word()?;
        let locals = input.word()?;
        let start = input.word()?;
        let first = input.word()?;
        let result = input.word()?;
        let callback = start == u32::MAX;
        if (!callback && (start as usize >= script.statements.len() || first as usize + parameters as usize + locals as usize > table.len()))
            || result as usize > table.len()
        {
            return Err(invalid("routine references"));
        }
        if parameters > 4096 || locals > 65_536 {
            return Err(ContainerError::Limit("routine frame"));
        }
        let mut flags = 0u32;
        let mut modes = Vec::new();
        for parameter in 0..parameters {
            let mode = input.word()?;
            if mode > 1 {
                return Err(invalid("parameter mode"));
            }
            modes.push(mode == 1);
            if mode == 1 {
                flags |= 1u32.checked_shl(parameter).unwrap_or(0);
            }
        }
        table.routine_modes.insert(id, modes);
        let entry = table.try_get_entry_mut(id).ok_or(invalid("routine variable"))?;
        if callback && (entry.entry_type != EntryType::Parameter || first != 0 || locals != 0 || result != 0) {
            return Err(invalid("callback descriptor"));
        }
        let start = if callback { 0 } else { start * 2 };
        if entry.header.variable_type == VariableType::Function {
            if (!callback && result == 0) || flags != 0 {
                return Err(invalid("function signature"));
            }
            entry.value.data = FunctionValue {
                parameters: parameters.try_into().map_err(|_| ContainerError::Limit("parameters"))?,
                local_variables: locals.try_into().map_err(|_| ContainerError::Limit("locals"))?,
                start_offset: start,
                first_var_id: first.try_into().map_err(|_| ContainerError::Limit("routine frame"))?,
                return_var: result.try_into().map_err(|_| ContainerError::Limit("result id"))?,
            }
            .to_data();
        } else if entry.header.variable_type == VariableType::Procedure && result == 0 {
            entry.value.data = ProcedureValue {
                parameters: parameters.try_into().map_err(|_| ContainerError::Limit("parameters"))?,
                local_variables: locals.try_into().map_err(|_| ContainerError::Limit("locals"))?,
                start_offset: start,
                first_var_id: first.try_into().map_err(|_| ContainerError::Limit("routine frame"))?,
                pass_flags: flags,
            }
            .to_data();
        } else {
            return Err(invalid("routine kind"));
        }
    }
    input.finish()?;
    for entry in table.get_entries() {
        if matches!(entry.header.variable_type, VariableType::Function | VariableType::Procedure) && !routine_ids.contains(&entry.header.id) {
            return Err(invalid("missing routine"));
        }
    }
    validate_code(&script, &table, &user_types)?;
    table.generate_names();
    let mut debug_info = None;
    if let Some(debug) = container.sections.iter().find(|section| section.kind == *b"DBUG") {
        if debug.entries as usize != table.len() {
            return Err(invalid("debug variable count"));
        }
        let mut input = Reader::new(&debug.data);
        for id in 1..=table.len() {
            table.get_var_entry_mut(id).name = input.text()?;
        }
        let mut names = super::DebugInfo::default();
        let records = input.count(8)?;
        if records != user_types.len() {
            return Err(invalid("debug record count"));
        }
        for fields in &user_types {
            let name = input.text()?;
            let count = input.count(4)?;
            if count != fields.len() {
                return Err(invalid("debug field count"));
            }
            names.records.push((name, (0..count).map(|_| input.text()).collect::<Result<Vec<_>>>()?));
        }
        let enums = input.count(12)?;
        if enums != table.enums.len() {
            return Err(invalid("debug enum count"));
        }
        for _ in 0..enums {
            let id = input.word()?;
            let name = input.text()?;
            let count = input.count(4)?;
            if table.enums.get(&id).is_none_or(|values| values.len() != count) {
                return Err(invalid("debug enum members"));
            }
            names.enums.insert(id, (name, (0..count).map(|_| input.text()).collect::<Result<Vec<_>>>()?));
        }
        input.finish()?;
        debug_info = Some(names);
    }
    let mut next_enum = crate::parser::EVENT_KIND_ENUM_ID - crate::parser::BUILTIN_ENUM_COUNT as u32;
    for &id in table.enums.keys() {
        if !imports.types.contains_key(&id) && current_catalog.types.contains_key(&id) {
            while type_ids.contains(&next_enum) || current_catalog.types.contains_key(&next_enum) || type_remap.values().any(|&mapped| mapped == next_enum) {
                next_enum -= 1;
            }
            if next_enum as usize <= FIRST_USER_TYPE_ID + user_types.len() {
                return Err(ContainerError::Limit("enum ids"));
            }
            type_remap.insert(id, next_enum);
            next_enum -= 1;
        }
    }
    let (_, constants) = imports.rewrite(&mut script, &table, &user_types, &type_remap, &member_remap)?;
    table.remap_user_types(&type_remap);
    table.enums = table
        .enums
        .into_iter()
        .map(|(id, values)| (type_remap.get(&id).copied().unwrap_or(id), values))
        .collect();
    if let Some(names) = &mut debug_info {
        names.enums = std::mem::take(&mut names.enums)
            .into_iter()
            .map(|(id, entry)| (type_remap.get(&id).copied().unwrap_or(id), entry))
            .collect();
    }
    for fields in &mut user_types {
        for field in fields {
            if let VariableType::UserData(id) = &mut field.variable_type {
                *id = type_remap.get(id).copied().unwrap_or(*id);
            }
        }
    }
    for statement in &mut script.statements {
        statement.command.remap_user_types(&type_remap);
    }
    let enum_values: Vec<_> = table
        .get_entries()
        .iter()
        .filter(|entry| entry.header.dim == 0 && table.is_enum(entry.header.variable_type))
        .map(|entry| (entry.header.id, entry.value.data))
        .collect();
    table.fill_in_records(&user_types);
    for (id, data) in enum_values {
        table.get_value_mut(id).data = data;
    }
    for constant in constants {
        table.push(constant);
    }
    table.host_catalog = Some(current_catalog);
    Ok(Executable {
        runtime: 400,
        variable_table: table,
        user_types,
        script_buffer: Vec::new(),
        in_memory_script: Some(script),
        extra_sections,
        debug_info,
    })
}

fn validate_code(script: &PPEScript, table: &VariableTable, types: &[Vec<RecordField>]) -> Result<()> {
    fn target(value: &PPEExpr, table: &VariableTable) -> Result<()> {
        match value {
            PPEExpr::Value(id) | PPEExpr::Dim(id, _) if table.try_get_entry(*id).is_some_and(|entry| entry.entry_type != EntryType::Constant) => Ok(()),
            PPEExpr::Member(_, _) | PPEExpr::IndexedMember(_, _, _) => Ok(()),
            _ => Err(invalid("assignment target")),
        }
    }
    fn expr(value: &PPEExpr, table: &VariableTable, types: &[Vec<RecordField>]) -> Result<()> {
        match value {
            PPEExpr::Invalid => return Err(invalid("invalid expression")),
            PPEExpr::Value(id) => {
                table.try_get_entry(*id).ok_or(invalid("variable reference"))?;
            }
            PPEExpr::RoutineReference(id) => {
                if table
                    .try_get_entry(*id)
                    .is_none_or(|entry| !matches!(entry.header.variable_type, VariableType::Function | VariableType::Procedure))
                {
                    return Err(invalid("routine reference"));
                }
            }
            PPEExpr::RecordLiteral(id, fields) => {
                let layout = types
                    .get((*id as usize).checked_sub(FIRST_USER_TYPE_ID).ok_or(invalid("record literal type"))?)
                    .ok_or(invalid("record literal type"))?;
                let mut seen = HashSet::new();
                for (field, value) in fields {
                    if *field >= layout.len() || !seen.insert(*field) {
                        return Err(invalid("record literal field"));
                    }
                    expr(value, table, types)?;
                }
            }
            PPEExpr::Dim(id, args) | PPEExpr::FunctionCall(id, args) => {
                let entry = table.try_get_entry(*id).ok_or(invalid("call or array reference"))?;
                if matches!(value, PPEExpr::FunctionCall(..)) && entry.header.variable_type != VariableType::Function {
                    return Err(invalid("function reference"));
                }
                if matches!(value, PPEExpr::FunctionCall(..)) && args.len() != unsafe { entry.value.data.function_value.parameters } as usize {
                    return Err(invalid("function argument count"));
                }
                if matches!(value, PPEExpr::Dim(..)) && (args.len() > 3 || args.len() != entry.header.dim as usize) {
                    return Err(invalid("array rank"));
                }
                for arg in args {
                    expr(arg, table, types)?;
                }
            }
            PPEExpr::Member(base, _) | PPEExpr::UnaryExpression(_, base) => expr(base, table, types)?,
            PPEExpr::BinaryExpression(_, left, right) => {
                expr(left, table, types)?;
                expr(right, table, types)?;
            }
            PPEExpr::IndexedMember(base, _, args) | PPEExpr::MemberFunctionCall(base, args, _) => {
                expr(base, table, types)?;
                for arg in args {
                    expr(arg, table, types)?;
                }
            }
            PPEExpr::PredefinedFunctionCall(_, args) => {
                for arg in args {
                    expr(arg, table, types)?;
                }
            }
        }
        Ok(())
    }
    for statement in &script.statements {
        match &statement.command {
            PPECommand::IfNot(value, _) | PPECommand::MemberCall(value) => expr(value, table, types)?,
            PPECommand::Let(left, right) => {
                expr(left, table, types)?;
                expr(right, table, types)?;
                target(left, table)?;
            }
            PPECommand::ProcedureCall(id, args) => {
                if table
                    .try_get_entry(*id)
                    .is_none_or(|entry| entry.header.variable_type != VariableType::Procedure)
                {
                    return Err(invalid("procedure reference"));
                }
                if args.len() != unsafe { table.get_var_entry(*id).value.data.procedure_value.parameters } as usize {
                    return Err(invalid("procedure argument count"));
                }
                for (index, arg) in args.iter().enumerate() {
                    expr(arg, table, types)?;
                    if table.is_var_parameter(*id, index) {
                        target(arg, table)?;
                    }
                }
            }
            PPECommand::PredefinedCall(_, args) => {
                for arg in args {
                    expr(arg, table, types)?;
                }
            }
            PPECommand::ForEach(id, value, _) => {
                table.try_get_entry(*id).ok_or(invalid("iteration variable"))?;
                expr(value, table, types)?;
            }
            PPECommand::OnError(super::OnErrorTarget::Procedure(id)) => {
                if table
                    .try_get_entry(*id)
                    .is_none_or(|entry| entry.header.variable_type != VariableType::Procedure)
                {
                    return Err(invalid("handler procedure"));
                }
            }
            _ => {}
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        ast::BinOp,
        executable::{PPEStatement, VariableValue},
    };

    #[test]
    fn invalid_references_and_assignment_targets_are_rejected() {
        for command in [
            PPECommand::Let(Box::new(PPEExpr::Value(2)), Box::new(PPEExpr::Value(1))),
            PPECommand::Let(Box::new(PPEExpr::Value(1)), Box::new(PPEExpr::Value(1))),
            PPECommand::MemberCall(Box::new(PPEExpr::RoutineReference(1))),
            PPECommand::MemberCall(Box::new(PPEExpr::Dim(1, vec![PPEExpr::Value(1)]))),
        ] {
            let mut executable = Executable::default();
            executable.variable_table.push(TableEntry::new(
                "constant",
                VarHeader {
                    id: 1,
                    variable_type: VariableType::Integer,
                    ..Default::default()
                },
                VariableValue::new_int(3),
                EntryType::Constant,
            ));
            executable.in_memory_script = Some(PPEScript {
                statements: vec![PPEStatement { span: 0..1, command }],
                ..Default::default()
            });
            assert!(executable.to_buffer().is_err());
        }
    }

    #[test]
    fn a_future_optional_section_loads_and_survives_a_rewrite() {
        let mut executable = Executable::default();
        executable.variable_table.push(TableEntry::new(
            "value",
            VarHeader {
                id: 1,
                variable_type: VariableType::Integer,
                ..Default::default()
            },
            VariableValue::new_int(1),
            EntryType::Constant,
        ));
        executable.in_memory_script = Some(PPEScript {
            statements: vec![PPEStatement {
                span: 0..1,
                command: PPECommand::End,
            }],
            ..Default::default()
        });

        let limits = LoadLimits::default();
        let mut container = Container::decode(&executable.to_buffer().unwrap(), &limits).unwrap();
        let mut future = Section::new(*b"FUTR", 3, b"written by a newer compiler".to_vec());
        future.flags = 0;
        container.sections.push(future.clone());
        let bytes = container.encode(Compression::None, &limits).unwrap();

        // The identity covers the program, so adding an optional section keeps it valid.
        let loaded = Executable::from_buffer(&mut bytes.clone(), false).unwrap();
        assert_eq!(loaded.extra_sections, vec![future]);
        let rewritten = Container::decode(&loaded.to_buffer().unwrap(), &limits).unwrap();
        assert!(rewritten.sections.iter().any(|section| section.kind == *b"FUTR"));

        let mut required = Container::decode(&bytes, &limits).unwrap();
        required.sections.last_mut().unwrap().flags = REQUIRED;
        let bytes = required.encode(Compression::None, &limits).unwrap();
        assert!(Executable::from_buffer(&mut bytes.clone(), false).is_err());
    }

    #[test]
    fn the_code_budget_is_measured_in_stored_bytes() {
        let mut executable = Executable::default();
        executable.variable_table.push(TableEntry::new(
            "value",
            VarHeader {
                id: 1,
                variable_type: VariableType::Integer,
                ..Default::default()
            },
            VariableValue::new_int(1),
            EntryType::Constant,
        ));
        let statements = (0..64)
            .map(|index| PPEStatement {
                span: index..index + 1,
                command: PPECommand::Goto((index + 1) * 2),
            })
            .chain(std::iter::once(PPEStatement {
                span: 64..65,
                command: PPECommand::End,
            }))
            .collect();
        executable.in_memory_script = Some(PPEScript {
            statements,
            ..Default::default()
        });

        let script = executable.in_memory_script.clone().unwrap();
        let bytes = executable.to_buffer().unwrap();
        let container = Container::decode(&bytes, &LoadLimits::default()).unwrap();
        let stored = container.sections.iter().find(|section| section.kind == *b"CODE").unwrap().data.len();

        assert_eq!(super::super::code400::encoded_size(&script).unwrap(), stored);
    }

    #[test]
    fn executable_roundtrip_both_compression_modes() {
        let mut executable = Executable::default();
        executable.variable_table.push(TableEntry::new(
            "text",
            VarHeader {
                id: 1,
                variable_type: VariableType::String,
                ..Default::default()
            },
            VariableValue::new_string("\u{20ac}\0\u{1f680}".repeat(20_000)),
            EntryType::Constant,
        ));
        executable.in_memory_script = Some(PPEScript {
            statements: vec![
                PPEStatement {
                    span: 0..1,
                    command: PPECommand::IfNot(
                        Box::new(PPEExpr::BinaryExpression(
                            BinOp::ShortAnd,
                            Box::new(PPEExpr::Value(1)),
                            Box::new(PPEExpr::Value(1)),
                        )),
                        2,
                    ),
                },
                PPEStatement {
                    span: 1..2,
                    command: PPECommand::End,
                },
            ],
            ..Default::default()
        });
        for compression in [Compression::None, Compression::Zstd] {
            let mut bytes = executable.to_buffer_with_compression(compression).unwrap();
            assert!(bytes.starts_with(super::super::container::MAGIC));
            let loaded = Executable::from_buffer(&mut bytes, false).unwrap();
            assert_eq!(
                executable.variable_table.get_value(1).as_string(),
                loaded.variable_table.get_value(1).as_string()
            );
            assert_eq!(
                executable.in_memory_script.as_ref().unwrap().statements[0].command,
                loaded.in_memory_script.unwrap().statements[0].command
            );
        }
    }
}
