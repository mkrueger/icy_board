use std::{
    fmt::{self, Display},
    io::stdout,
};

use codepages::tables::{CP437_TO_UNICODE, UNICODE_TO_CP437};
use crossterm::{
    execute,
    style::{Attribute, Color, Print, SetAttribute, SetForegroundColor},
};

use crate::{
    Res,
    crypt::{decrypt_chunks, encrypt_chunks},
};

use super::{ExecutableError, GenericVariableData, LAST_PPE_RUNTIME, PPEExpr, PPEScript, VariableData, VariableNameGenerator, VariableType, VariableValue};

pub const VARIABLE_FLAG_STATIC: u8 = 0x01;
pub const VARIABLE_FLAG_DYNAMIC_ARRAY: u8 = 0x02;
/// Runtime-400 whole-array parameter. Unmarked parameter arrays keep PCBoard's
/// element-zero calling convention, including on a runtime-400 target.
pub const VARIABLE_FLAG_ARRAY_PARAMETER: u8 = 0x04;
pub(crate) const MAX_DESERIALIZED_ARRAY_ELEMENTS: usize = 1_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RecordField {
    pub variable_type: VariableType,
    pub dim: u8,
    /// In-memory only; dynamic fields have rank 1..=3 and zero stored bounds.
    pub is_dynamic: bool,
    pub vector_size: usize,
    pub matrix_size: usize,
    pub cube_size: usize,
}

impl RecordField {
    pub fn scalar(variable_type: VariableType) -> Self {
        Self {
            variable_type,
            dim: 0,
            is_dynamic: false,
            vector_size: 0,
            matrix_size: 0,
            cube_size: 0,
        }
    }

    pub fn element_count(self) -> Option<usize> {
        if self.is_dynamic {
            return ((1..=3).contains(&self.dim) && self.vector_size == 0 && self.matrix_size == 0 && self.cube_size == 0).then_some(0);
        }
        if self.dim > 3 || (self.dim < 3 && self.cube_size != 0) || (self.dim < 2 && self.matrix_size != 0) || (self.dim == 0 && self.vector_size != 0) {
            return None;
        }
        let bounds = [self.vector_size, self.matrix_size, self.cube_size];
        bounds[..self.dim as usize]
            .iter()
            .try_fold(1usize, |count, bound| count.checked_mul(*bound + 1))
            .filter(|count| *count <= super::variable_value::MAX_ARRAY_SIZE)
    }

    pub(crate) fn has_record_io_scalar_type(self) -> bool {
        matches!(
            self.variable_type,
            VariableType::Boolean
                | VariableType::Unsigned
                | VariableType::Date
                | VariableType::EDate
                | VariableType::Integer
                | VariableType::Money
                | VariableType::Float
                | VariableType::String
                | VariableType::Time
                | VariableType::Byte
                | VariableType::Word
                | VariableType::SByte
                | VariableType::SWord
                | VariableType::BigStr
                | VariableType::UnboundedString
                | VariableType::Double
                | VariableType::DDate
                | VariableType::MessageAreaID
                | VariableType::Long
                | VariableType::ULong
        )
    }
}

#[derive(Clone, Default, Debug, PartialEq)]
pub struct VarHeader {
    pub id: usize,
    pub dim: u8,
    pub vector_size: usize,
    pub matrix_size: usize,
    pub cube_size: usize,
    pub variable_type: VariableType,
    pub flags: u8,
}

impl Display for VarHeader {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.dim > 0 {
            write!(
                f,
                "[id:{}, variable_type:{}, flags:{}, dim:{}[{},{},{})]",
                self.id, self.variable_type, self.flags, self.dim, self.vector_size, self.matrix_size, self.cube_size
            )
        } else {
            write!(f, "[id:{}, variable_type:{}, flags:{}]", self.id, self.variable_type, self.flags)
        }
    }
}

impl VarHeader {
    pub(crate) fn allocated_elements(&self) -> Option<usize> {
        if self.flags & VARIABLE_FLAG_DYNAMIC_ARRAY != 0 || self.dim == 0 {
            return Some(0);
        }
        let bounds = [self.vector_size, self.matrix_size, self.cube_size];
        bounds[..self.dim as usize]
            .iter()
            .try_fold(1usize, |count, bound| count.checked_mul(bound.checked_add(1)?))
    }

    /// .
    ///
    /// # Errors
    ///
    /// Panics if .
    pub fn from_bytes(cur_block: &[u8]) -> Res<VarHeader> {
        if cur_block.len() < 11 {
            return Err(Box::new(ExecutableError::BufferTooShort(cur_block.len())));
        }
        let mut dim = cur_block[2];
        if dim > 3 {
            log::warn!("Invalid dimension: {dim}, setting to 3");
            dim = 3;
        }

        Ok(Self {
            id: u16::from_le_bytes(cur_block[0..2].try_into()?) as usize,
            dim,
            vector_size: u16::from_le_bytes(cur_block[3..5].try_into()?) as usize,
            matrix_size: u16::from_le_bytes(cur_block[5..7].try_into()?) as usize,
            cube_size: u16::from_le_bytes(cur_block[7..9].try_into()?) as usize,
            variable_type: VariableType::from_byte(cur_block[9]),
            flags: cur_block[10],
        })
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buffer = Vec::new();
        buffer.extend(u16::to_le_bytes(self.id as u16));
        assert!(self.dim <= 3, "Invalid dimension: {}", self.dim);
        buffer.push(self.dim);
        buffer.extend(u16::to_le_bytes(self.vector_size as u16));
        buffer.extend(u16::to_le_bytes(self.matrix_size as u16));
        buffer.extend(u16::to_le_bytes(self.cube_size as u16));

        buffer.push(self.variable_type.into());
        buffer.push(self.flags);
        buffer
    }

    /// Returns the create generic data of this [`VarHeader`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn create_generic_data(&self) -> Option<GenericVariableData> {
        if self.flags & VARIABLE_FLAG_DYNAMIC_ARRAY != 0 {
            return match self.dim {
                1 => Some(GenericVariableData::Dim1(std::sync::Arc::new(Vec::new()))),
                2 => Some(GenericVariableData::Dim2(std::sync::Arc::new(Vec::new()))),
                3 => Some(GenericVariableData::Dim3(std::sync::Arc::new(Vec::new()))),
                _ => None,
            };
        }
        match self.dim {
            0 => Some(GenericVariableData::None),
            1..=3 => GenericVariableData::create_array(
                self.variable_type.create_empty_value(),
                self.dim,
                self.vector_size,
                self.matrix_size,
                self.cube_size,
            ),
            _ => panic!("Invalid dimension: {}", self.dim),
        }
    }
}

/// Initializes value fields recursively; the engine replaces host placeholders.
pub fn create_record_value(type_id: u32, user_types: &[Vec<RecordField>], enums: &std::collections::BTreeMap<u32, Vec<i32>>) -> Option<VariableValue> {
    create_record_value_inner(type_id, user_types, enums, &mut Vec::new())
}

fn create_record_value_inner(
    type_id: u32,
    user_types: &[Vec<RecordField>],
    enums: &std::collections::BTreeMap<u32, Vec<i32>>,
    visiting: &mut Vec<u32>,
) -> Option<VariableValue> {
    if let Some(values) = enums.get(&type_id) {
        let default = *values.first()?;
        return Some(VariableValue::new_enum(VariableType::UserData(type_id), default, default));
    }
    if type_id as usize != crate::parser::CONTACT_ID && crate::parser::board_catalog::TYPES.iter().any(|&(id, _, _)| id == type_id as usize) {
        return Some(VariableType::UserData(type_id).create_empty_value());
    }
    if visiting.contains(&type_id) {
        return None;
    }
    visiting.push(type_id);
    let built_in_fields = match type_id as usize {
        crate::parser::CONTACT_ID => Some(vec![
            RecordField::scalar(VariableType::UnboundedString),
            RecordField::scalar(VariableType::UnboundedString),
        ]),
        crate::parser::MSG_HEADER_ID => Some(vec![
            RecordField::scalar(VariableType::UnboundedString),
            RecordField::scalar(VariableType::UnboundedString),
            RecordField::scalar(VariableType::UnboundedString),
            RecordField::scalar(VariableType::Boolean),
        ]),
        _ => None,
    };
    let fields = if let Some(fields) = built_in_fields.as_ref() {
        fields
    } else {
        user_types.get((type_id as usize).checked_sub(crate::parser::FIRST_USER_TYPE_ID)?)?
    };
    let mut values = Vec::with_capacity(fields.len());
    for field in fields {
        field.element_count()?;
        let value = match field.variable_type {
            VariableType::UserData(id) => create_record_value_inner(id, user_types, enums, visiting)?,
            _ => field.variable_type.create_empty_value(),
        };
        let value = if field.dim == 0 {
            value
        } else {
            let generic_data = if field.is_dynamic {
                VarHeader {
                    dim: field.dim,
                    flags: VARIABLE_FLAG_DYNAMIC_ARRAY,
                    ..Default::default()
                }
                .create_generic_data()?
            } else {
                GenericVariableData::create_array(
                    value,
                    field.dim,
                    field.vector_size as usize,
                    field.matrix_size as usize,
                    field.cube_size as usize,
                )?
            };
            VariableValue {
                vtype: field.variable_type,
                data: enums
                    .get(&u32::from(field.variable_type))
                    .and_then(|values| values.first())
                    .map_or_else(VariableData::default, |value| VariableData::from_int(*value)),
                generic_data,
            }
        };
        values.push(value);
    }
    visiting.pop();
    Some(VariableValue {
        vtype: VariableType::UserData(type_id),
        data: crate::executable::VariableData::default(),
        generic_data: GenericVariableData::Record(std::sync::Arc::new(values)),
    })
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct FunctionValue {
    pub parameters: u32,
    pub local_variables: u32,
    pub start_offset: u32,
    pub first_var_id: i32,
    pub return_var: i32,
}

impl fmt::Debug for FunctionValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "parameters:{} locals:{} offset:{:04X}h first:{:04X}h return:{:04X}h",
            self.parameters, self.local_variables, self.start_offset, self.first_var_id, self.return_var
        )
    }
}

#[repr(C)]
#[derive(Clone, Copy, Default)]
pub struct ProcedureValue {
    pub parameters: u32,
    pub local_variables: u32,
    pub start_offset: u32,
    pub first_var_id: i32,
    pub pass_flags: u32,
}

#[derive(Clone, Copy, Debug)]
pub struct MsgAreaIdValue {
    pub conference: i32,
    pub area: i32,
}

impl ProcedureValue {
    pub fn to_data(self) -> VariableData {
        let mut res = VariableData::default();
        res.procedure_value = self;
        res
    }
}

impl fmt::Debug for ProcedureValue {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(
            f,
            "parameters:{} locals:{} offset:{:04X}h first:{:04X}h pass:{:b}b",
            self.parameters, self.local_variables, self.start_offset, self.first_var_id, self.pass_flags
        )
    }
}

impl FunctionValue {
    /// .
    ///
    /// # Errors
    ///
    /// Panics if .
    pub fn from_bytes(cur_buf: &[u8]) -> Res<FunctionValue> {
        if cur_buf.len() < 8 {
            return Err(Box::new(ExecutableError::BufferTooShort(cur_buf.len())));
        }
        Ok(Self {
            parameters: u32::from(cur_buf[0]),
            local_variables: u32::from(cur_buf[1]),
            start_offset: u32::from(u16::from_le_bytes((cur_buf[2..=3]).try_into()?)),
            first_var_id: i32::from(i16::from_le_bytes((cur_buf[4..=5]).try_into()?)),
            return_var: i32::from(i16::from_le_bytes((cur_buf[6..=7]).try_into()?)),
        })
    }

    pub fn append(&self, buffer: &mut Vec<u8>) -> Result<(), ExecutableError> {
        let invalid = || ExecutableError::Format400("routine cannot be represented by a legacy PPE".into());
        buffer.push(u8::try_from(self.parameters).map_err(|_| invalid())?);
        buffer.push(u8::try_from(self.local_variables).map_err(|_| invalid())?);
        buffer.extend(u16::try_from(self.start_offset).map_err(|_| invalid())?.to_le_bytes());
        buffer.extend(i16::try_from(self.first_var_id).map_err(|_| invalid())?.to_le_bytes());
        buffer.extend((self.return_var as u16).to_le_bytes());
        Ok(())
    }

    pub fn to_data(self) -> VariableData {
        let mut res = VariableData::default();
        res.function_value = self;
        res
    }
}

#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq, Default)]
pub enum EntryType {
    #[default]
    Constant,
    UserVariable,
    Variable,
    LocalVariable,
    FunctionResult,
    Parameter,
    Function,
    Procedure,
}
impl EntryType {
    pub fn use_name(self) -> bool {
        self != EntryType::Constant
    }
}

#[derive(Clone, Default, Debug)]
pub struct TableEntry {
    pub header: VarHeader,
    pub name: String,
    pub entry_type: EntryType,
    pub value: VariableValue,
    pub function_id: usize,
}

impl TableEntry {
    pub fn new(name: impl Into<String>, header: VarHeader, variable: VariableValue, entry_type: EntryType) -> Self {
        Self {
            header,
            name: name.into(),
            value: variable,
            function_id: 0,
            entry_type,
        }
    }

    pub fn get_name(&self) -> &String {
        &self.name
    }

    pub fn set_name(&mut self, name: String) {
        self.name = name;
    }

    pub fn get_type(&self) -> EntryType {
        self.entry_type
    }

    pub fn set_type(&mut self, entry_type: EntryType) {
        self.entry_type = entry_type;
    }

    pub fn report_variable_usage(&mut self) {
        if self.entry_type == EntryType::Constant {
            self.entry_type = EntryType::Variable;
        }
    }

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn to_buffer(&self, version: u16) -> Result<Vec<u8>, ExecutableError> {
        let mut buffer = self.header.to_bytes();
        encrypt_chunks(&mut buffer, version, false);

        let b = buffer.len();
        if self.header.variable_type == VariableType::Procedure || self.header.variable_type == VariableType::Function {
            if version < 340 {
                buffer.push(0);
                buffer.push(0);
            }
            buffer.push(self.header.variable_type.into());
            buffer.push(0);
            unsafe {
                self.value.data.function_value.append(&mut buffer)?;
            }
            encrypt_chunks(&mut buffer[b..], version, false);
        } else if self.header.variable_type == VariableType::String {
            if self.header.dim == 0 {
                let GenericVariableData::String(s) = &self.value.generic_data else {
                    return Err(ExecutableError::StringTypeInvalid(self.value.vtype));
                };
                let mut string_buffer: Vec<u8> = Vec::new();
                if version >= 400 {
                    string_buffer.extend_from_slice(s.as_bytes());
                } else {
                    for c in s.chars() {
                        if let Some(b) = UNICODE_TO_CP437.get(&c) {
                            string_buffer.push(*b);
                        } else {
                            string_buffer.push(c as u8);
                        }
                    }
                }
                string_buffer.push(0);
                if string_buffer.len() > u16::MAX as usize {
                    return Err(ExecutableError::StringConstantTooLong(string_buffer.len() - 1));
                }

                buffer.extend_from_slice(&u16::to_le_bytes(string_buffer.len() as u16));
                encrypt_chunks(&mut string_buffer, version, false);
                buffer.extend(string_buffer);
            } else {
                buffer.extend_from_slice(&[0, 0]);
            }
        } else {
            if version < 340 {
                // VTABLE - get's ignored by PCBoard - pure garbage
                buffer.push(0);
                buffer.push(0);
            }

            // variable type
            buffer.push(self.header.variable_type.into());
            buffer.push(0);

            if version <= 100 {
                buffer.extend_from_slice(&u32::to_le_bytes(self.value.get_u64_value() as u32));
            } else {
                buffer.extend_from_slice(&u64::to_le_bytes(self.value.get_u64_value()));
                encrypt_chunks(&mut buffer[b..], version, false);
            }
        }
        Ok(buffer)
    }
}

#[derive(Default, Clone)]
pub struct VariableTable {
    version: u16,
    entries: Vec<TableEntry>,
    has_user_vars: bool,
    /// Known integer values, in declaration order (the first value is the default).
    pub enums: std::collections::BTreeMap<u32, Vec<i32>>,
    /// Rebuilt from layouts by `fill_in_records`; never serialized. Check before record I/O.
    pub record_io_unsupported_types: std::collections::BTreeSet<u32>,
    pub routine_modes: std::collections::BTreeMap<usize, Vec<bool>>,
    pub host_catalog: Option<super::imports400::HostCatalog>,
}

impl VariableTable {
    pub fn is_var_parameter(&self, routine: usize, parameter: usize) -> bool {
        let routine = if let Some(entry) = self.try_get_entry(routine)
            && entry.entry_type == EntryType::Parameter
            && entry.header.variable_type == VariableType::Procedure
        {
            let assigned = unsafe { entry.value.data.procedure_value };
            self.get_entries()
                .iter()
                .find(|candidate| {
                    candidate.entry_type == EntryType::Procedure
                        && unsafe {
                            candidate.value.data.procedure_value.start_offset == assigned.start_offset
                                && candidate.value.data.procedure_value.first_var_id == assigned.first_var_id
                        }
                })
                .map_or(routine, |candidate| candidate.header.id)
        } else {
            routine
        };
        if let Some(modes) = self.routine_modes.get(&routine) {
            return modes.get(parameter).copied().unwrap_or(false);
        }
        self.try_get_entry(routine).is_some_and(|entry| {
            entry.header.variable_type == VariableType::Procedure
                && 1u32
                    .checked_shl(parameter as u32)
                    .is_some_and(|mask| unsafe { entry.value.data.procedure_value.pass_flags } & mask != 0)
        })
    }

    pub fn parameter_modes(&self, routine: usize, count: usize) -> Vec<bool> {
        (0..count).map(|parameter| self.is_var_parameter(routine, parameter)).collect()
    }
    /// Legacy array reads outside the bounds yield an empty value. For an
    /// enum that value must be its first member, never an untyped numeric zero.
    pub fn array_value(&self, array: &VariableValue, first: usize, second: usize, third: usize) -> VariableValue {
        if let VariableType::UserData(id) = array.vtype
            && let Some(default) = self.enums.get(&id).and_then(|members| members.first())
        {
            let value = match &array.generic_data {
                GenericVariableData::Dim1(values) => values.get(first),
                GenericVariableData::Dim2(values) => values.get(first).and_then(|row| row.get(second)),
                GenericVariableData::Dim3(values) => values.get(first).and_then(|plane| plane.get(second)).and_then(|row| row.get(third)),
                _ => None,
            };
            return value.cloned().unwrap_or_else(|| VariableValue::new_enum(array.vtype, *default, *default));
        }
        array.get_array_value(first, second, third)
    }

    /// Check the nominal type before publishing any part of a value.
    pub fn checked_enum_value(&self, expected: VariableType, value: VariableValue) -> Res<VariableValue> {
        let VariableType::UserData(id) = expected else {
            return Ok(value);
        };
        let Some(members) = self.enums.get(&id) else {
            return Ok(value);
        };
        let default = *members.first().ok_or(ExecutableError::InvalidEnumDefinition(id))?;
        if value.vtype != expected && value.vtype != VariableType::Integer {
            return Err(super::VMError::InvalidEnumValue(id, value.as_string()).into());
        }
        let generic_data = match value.generic_data {
            GenericVariableData::Dim1(values) => GenericVariableData::Dim1(std::sync::Arc::new(
                values
                    .iter()
                    .cloned()
                    .map(|value| self.checked_enum_value(expected, value))
                    .collect::<Res<Vec<_>>>()?,
            )),
            GenericVariableData::Dim2(values) => GenericVariableData::Dim2(std::sync::Arc::new(
                values
                    .iter()
                    .map(|row| {
                        row.iter()
                            .cloned()
                            .map(|value| self.checked_enum_value(expected, value))
                            .collect::<Res<Vec<_>>>()
                    })
                    .collect::<Res<Vec<_>>>()?,
            )),
            GenericVariableData::Dim3(values) => GenericVariableData::Dim3(std::sync::Arc::new(
                values
                    .iter()
                    .map(|plane| {
                        plane
                            .iter()
                            .map(|row| {
                                row.iter()
                                    .cloned()
                                    .map(|value| self.checked_enum_value(expected, value))
                                    .collect::<Res<Vec<_>>>()
                            })
                            .collect::<Res<Vec<_>>>()
                    })
                    .collect::<Res<Vec<_>>>()?,
            )),
            GenericVariableData::None | GenericVariableData::Enum(_) => {
                let number = unsafe { value.data.int_value };
                return Ok(VariableValue::new_enum(expected, number, default));
            }
            _ => return Err(super::VMError::InvalidEnumValue(id, "non-integer value".to_string()).into()),
        };
        Ok(VariableValue {
            vtype: expected,
            data: VariableData::from_int(default),
            generic_data,
        })
    }

    pub fn is_enum(&self, vtype: VariableType) -> bool {
        matches!(vtype, VariableType::UserData(id) if self.enums.contains_key(&id))
    }

    pub(crate) fn remap_user_types(&mut self, remap: &std::collections::HashMap<u32, u32>) {
        for entry in &mut self.entries {
            if let VariableType::UserData(type_id) = entry.header.variable_type
                && let Some(new_id) = remap.get(&type_id)
            {
                entry.header.variable_type = VariableType::UserData(*new_id);
            }
            entry.value.remap_user_types(remap);
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn has_user_vars(&self) -> bool {
        self.has_user_vars
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// .
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn deserialize(version: u16, buf: &mut [u8]) -> Res<(usize, Self)> {
        let mut i = 0;
        let Some(max_var_bytes) = buf.get(i..i + 2) else {
            return Err(Box::new(ExecutableError::BufferTooShort(buf.len())));
        };
        let max_var = u16::from_le_bytes(max_var_bytes.try_into()?) as usize;
        i += 2;
        let mut allocated_elements = 0usize;

        let mut result = vec![TableEntry::default(); max_var];
        if max_var == 0 {
            return Ok((
                i,
                VariableTable {
                    version,
                    entries: result,
                    has_user_vars: false,
                    enums: Default::default(),
                    record_io_unsupported_types: Default::default(),
                    routine_modes: Default::default(),
                    host_catalog: None,
                },
            ));
        }
        let mut var_count = max_var as i32 - 1;
        while var_count >= 0 {
            let Some(header_end) = i.checked_add(11) else {
                return Err(Box::new(ExecutableError::BufferTooShort(buf.len())));
            };
            let Some(cur_block) = buf.get_mut(i..header_end) else {
                return Err(Box::new(ExecutableError::BufferTooShort(buf.len())));
            };
            decrypt_chunks(cur_block, version, false);
            i += 11;
            let header = VarHeader::from_bytes(&buf[i - 11..i])?;
            // Dynamic storage is a 4.00 feature. Preserve legacy header bits
            // on disk without interpreting them as a new array-storage flag.
            let mut storage_header = header.clone();
            if version < 400 {
                storage_header.flags &= !VARIABLE_FLAG_DYNAMIC_ARRAY;
            }
            let elements = storage_header
                .allocated_elements()
                .ok_or(ExecutableError::ArrayAllocationTooLarge(usize::MAX, MAX_DESERIALIZED_ARRAY_ELEMENTS))?;
            let total_elements = allocated_elements
                .checked_add(elements)
                .ok_or(ExecutableError::ArrayAllocationTooLarge(usize::MAX, MAX_DESERIALIZED_ARRAY_ELEMENTS))?;
            if total_elements > MAX_DESERIALIZED_ARRAY_ELEMENTS {
                return Err(Box::new(ExecutableError::ArrayAllocationTooLarge(
                    total_elements,
                    MAX_DESERIALIZED_ARRAY_ELEMENTS,
                )));
            }
            allocated_elements = total_elements;

            if header.id > max_var {
                log::warn!("Variable count exceeds maximum: {} ({})", header.id, max_var);
            }
            if header.id != var_count as usize + 1 {
                log::warn!("Variable id mismatch: {} != {}", header.id, var_count as usize + 1);
            }

            let variable;
            let entry_type;
            match header.variable_type {
                VariableType::String => {
                    let Some(length_bytes) = buf.get(i..i + 2) else {
                        return Err(Box::new(ExecutableError::BufferTooShort(buf.len())));
                    };
                    let string_length = u16::from_le_bytes(length_bytes.try_into()?) as usize;
                    i += 2;
                    let Some(string_end) = i.checked_add(string_length) else {
                        return Err(Box::new(ExecutableError::BufferTooShort(buf.len())));
                    };
                    let Some(string_bytes) = buf.get_mut(i..string_end) else {
                        return Err(Box::new(ExecutableError::BufferTooShort(buf.len())));
                    };
                    decrypt_chunks(string_bytes, version, false);
                    let generic_data = if header.dim > 0 {
                        storage_header.create_generic_data()
                    } else {
                        // The stored length counts the terminating NUL, an empty constant has none.
                        let text_end = if string_length > 0 { string_end - 1 } else { i };
                        let str = if version >= 400 {
                            if string_length > 0 && buf[text_end] != 0 {
                                return Err(Box::new(ExecutableError::InvalidStringTerminator(header.id)));
                            }
                            std::str::from_utf8(&buf[i..text_end])
                                .map_err(|error| ExecutableError::InvalidStringEncoding {
                                    variable_id: header.id,
                                    valid_up_to: error.valid_up_to(),
                                })?
                                .to_string()
                        } else {
                            buf[i..text_end].iter().map(|byte| CP437_TO_UNICODE[*byte as usize]).collect()
                        };
                        Some(GenericVariableData::String(std::sync::Arc::new(str)))
                    };
                    variable = VariableValue {
                        vtype: VariableType::String,
                        generic_data: generic_data.unwrap_or(GenericVariableData::None),
                        ..Default::default()
                    };
                    i = string_end;
                    entry_type = EntryType::Constant;
                }

                VariableType::Function | VariableType::Procedure => {
                    if version <= 100 {
                        return Err(Box::new(ExecutableError::FunctionsNotSupported(version)));
                    }
                    let block_size = if version < 340 { 12 } else { 10 };
                    let Some(block_end) = i.checked_add(block_size) else {
                        return Err(Box::new(ExecutableError::BufferTooShort(buf.len())));
                    };
                    let Some(block) = buf.get_mut(i..block_end) else {
                        return Err(Box::new(ExecutableError::BufferTooShort(buf.len())));
                    };
                    decrypt_chunks(block, version, false);
                    if version < 340 {
                        i += 2; // SKIP VTABLE - seems ot get stored by accident.
                    }

                    let cur_buf = &buf[i..(i + 10)];
                    let vtype = VariableType::from_byte(cur_buf[0]);
                    if vtype != header.variable_type {
                        return Err(Box::new(ExecutableError::FunctionHeaderTypeMismatch(vtype, header.variable_type)));
                    }
                    let function_value = FunctionValue::from_bytes(&cur_buf[2..])?;
                    i += 2; // type

                    variable = VariableValue {
                        vtype,
                        data: VariableData { function_value },
                        ..Default::default()
                    };

                    entry_type = if vtype == VariableType::Function {
                        EntryType::Function
                    } else {
                        EntryType::Procedure
                    };
                    i += 8;
                }

                _ => {
                    if version <= 100 {
                        if i.checked_add(8).is_none_or(|end| end > buf.len()) {
                            return Err(Box::new(ExecutableError::BufferTooShort(buf.len())));
                        }
                        i += 2; // SKIP VTABLE - seems to get stored by accident.
                        let vtype: VariableType = VariableType::from_byte(buf[i]);
                        if vtype != header.variable_type {
                            log::error!(
                                "Encountered anomaly in variable table: {} variable type and variable value {} are not matching.",
                                header.variable_type,
                                vtype
                            );
                            log::error!("File is potentially damaged.");
                        }

                        // check variable type
                        let vtype = VariableType::from_byte(buf[i]);
                        if vtype != header.variable_type {
                            log::error!(
                                "Encountered anomaly in variable table: {} variable type and variable value {} are not matching.",
                                header.variable_type,
                                vtype
                            );
                            log::error!("File is potentially damaged.");
                        }
                        i += 2;

                        let mut data: VariableData = VariableData::default();
                        data.int_value = i32::from_le_bytes((buf[i..i + 4]).try_into()?);
                        variable = VariableValue {
                            vtype,
                            data,
                            generic_data: storage_header.create_generic_data().unwrap_or(GenericVariableData::None),
                        };
                        i += 4;
                    } else {
                        let block_size = if version < 340 { 12 } else { 10 };
                        let Some(block_end) = i.checked_add(block_size) else {
                            return Err(Box::new(ExecutableError::BufferTooShort(buf.len())));
                        };
                        let Some(block) = buf.get_mut(i..block_end) else {
                            return Err(Box::new(ExecutableError::BufferTooShort(buf.len())));
                        };
                        decrypt_chunks(block, version, false);
                        if version < 340 {
                            i += 2; // SKIP VTABLE - seems to get stored by accident.
                        }

                        // check variable type
                        let vtype = VariableType::from_byte(buf[i]);
                        if vtype != header.variable_type {
                            log::error!(
                                "Encountered anomaly in variable table: {} variable type and variable value {} are not matching.",
                                header.variable_type,
                                vtype
                            );
                            log::error!("File is potentially damaged.");
                        }
                        i += 2;

                        let mut data = VariableData::default();
                        data.u64_value = u64::from_le_bytes((buf[i..i + 8]).try_into()?);

                        variable = VariableValue {
                            vtype,
                            data,
                            generic_data: storage_header.create_generic_data().unwrap_or(GenericVariableData::None),
                        };
                        i += 8;
                    }

                    entry_type = EntryType::Constant;
                }
            }
            result[var_count as usize] = TableEntry::new("", header, variable, entry_type);
            var_count -= 1;
        }

        for k in (0..result.len()).rev() {
            let cur = result[k].clone();
            match cur.header.variable_type {
                VariableType::Function => unsafe {
                    let function = cur.value.data.function_value;
                    // A routine with no start offset owns nothing, so its ids are never read.
                    if function.start_offset > 0 {
                        if function.first_var_id < 0 || function.return_var < 0 {
                            return Err(Box::new(ExecutableError::InvalidVariableIndexInTable(usize::MAX, result.len())));
                        }
                        let ret = (function.return_var as usize).saturating_sub(1);
                        let owned_end = (function.first_var_id as usize)
                            .checked_add(function.parameters as usize)
                            .and_then(|end| end.checked_add(function.local_variables as usize))
                            .ok_or(ExecutableError::InvalidVariableIndexInTable(usize::MAX, result.len()))?;
                        let last = (function.local_variables as usize)
                            .checked_add(ret)
                            .ok_or(ExecutableError::InvalidVariableIndexInTable(usize::MAX, result.len()))?;
                        if owned_end > result.len() || last > result.len() {
                            return Err(Box::new(ExecutableError::InvalidVariableIndexInTable(owned_end.max(last), result.len())));
                        }
                        for (j, i) in (cur.value.data.function_value.first_var_id as usize..last).enumerate() {
                            let fvar = &mut result[i];
                            if i == ret {
                                fvar.set_type(EntryType::FunctionResult);
                            } else if j < cur.value.data.function_value.parameters as usize {
                                fvar.set_type(EntryType::Parameter);
                            }
                        }
                    }
                },
                VariableType::Procedure => unsafe {
                    let procedure = cur.value.data.procedure_value;
                    let mut j = 0;
                    if procedure.start_offset > 0 {
                        if procedure.first_var_id < 0 {
                            return Err(Box::new(ExecutableError::InvalidVariableIndexInTable(usize::MAX, result.len())));
                        }
                        let last = (procedure.first_var_id as usize)
                            .checked_add(procedure.parameters as usize)
                            .and_then(|end| end.checked_add(procedure.local_variables as usize))
                            .ok_or(ExecutableError::InvalidVariableIndexInTable(usize::MAX, result.len()))?;
                        if last > result.len() {
                            return Err(Box::new(ExecutableError::InvalidVariableIndexInTable(last, result.len())));
                        }
                        (cur.value.data.procedure_value.first_var_id as usize..last).for_each(|i| {
                            let fvar = &mut result[i];
                            if j < cur.value.data.procedure_value.parameters as usize {
                                fvar.set_type(EntryType::Parameter);
                            }
                            j += 1;
                        });
                    }
                },
                _ => {}
            }
        }

        let mut table = VariableTable {
            version,
            entries: result,
            has_user_vars: false,
            enums: Default::default(),
            record_io_unsupported_types: Default::default(),
            routine_modes: Default::default(),
            host_catalog: None,
        };
        table.analyze_locals();
        table.generate_names();
        Ok((i, table))
    }

    pub fn generate_names(&mut self) {
        let user_vars_version = self.scan_user_variables_version();
        self.has_user_vars = user_vars_version > 0;
        let mut name_generator = VariableNameGenerator::new(self.version, user_vars_version);
        for res in &mut self.entries {
            let (name, is_user_variable) = name_generator.get_next_name(res);
            if is_user_variable {
                res.set_type(EntryType::UserVariable);
            }

            res.set_name(name);
        }
        let mut par = 1;
        let mut vars = 1;
        let mut loc = 1;

        for i in 0..self.entries.len() {
            let var_type = self.entries[i].header.variable_type;
            if var_type == VariableType::Function {
                let id = unsafe { self.entries[i].value.data.function_value.return_var as usize };
                let name = self.entries[i].get_name().clone();
                if let Some(entry) = self.try_get_entry_mut(id) {
                    entry.set_name(name);
                }
            }
            if var_type == VariableType::Function || var_type == VariableType::Procedure {
                let first_var = unsafe { self.entries[i].value.data.procedure_value.first_var_id as usize };
                if unsafe { self.entries[i].value.data.procedure_value.start_offset } == 0 {
                    continue;
                }
                let last = unsafe {
                    self.entries[i].value.data.procedure_value.local_variables as usize
                        + self.entries[i].value.data.procedure_value.parameters as usize
                        + first_var
                };

                (first_var..last).for_each(|i| {
                    if self.entries[i].get_type() == EntryType::Parameter {
                        self.entries[i].set_name(format!("PAR{par:03}"));
                        par += 1;
                    } else if self.entries[i].get_type() == EntryType::Variable {
                        self.entries[i].set_name(format!("VAR{vars:03}"));
                        vars += 1;
                    } else if self.entries[i].get_type() == EntryType::LocalVariable {
                        self.entries[i].set_name(format!("LOC{loc:03}"));
                        loc += 1;
                    }
                });
            }
        }
    }

    pub fn analyze_usage(&mut self, script: &PPEScript) {
        for stmt in &script.statements {
            self.analyze_statement(&stmt.command);
        }
    }
    fn analyze_statement(&mut self, stmt: &super::PPECommand) {
        match stmt {
            super::PPECommand::ProcedureCall(id, args) => {
                for (i, arg) in args.iter().enumerate() {
                    if self.is_var_parameter(*id, i) {
                        self.report_usage(arg);
                    }
                }
            }
            super::PPECommand::PredefinedCall(id, args) => match id.sig {
                super::StatementSignature::Invalid => {}
                super::StatementSignature::ArgumentsWithVariable(var_arg, _) | super::StatementSignature::VariableArguments(var_arg, _, _) => {
                    if var_arg > 0 {
                        self.report_usage(&args[var_arg - 1]);
                    }
                }
                super::StatementSignature::SpecialCaseDcreate => {
                    self.report_usage(&args[3]);
                }
                super::StatementSignature::SpecialCaseDlockg | super::StatementSignature::SpecialCaseSort => {
                    self.report_usage(&args[1]);
                }
                super::StatementSignature::SpecialCaseVarSeg => {
                    self.report_usage(&args[0]);
                    self.report_usage(&args[1]);
                }
                super::StatementSignature::SpecialCasePop => {
                    for arg in args {
                        self.report_usage(arg);
                    }
                }
            },
            super::PPECommand::Let(variable, _) => {
                self.report_usage(variable);
            }
            super::PPECommand::ForEach(variable, collection, _) => {
                self.report_usage(&super::PPEExpr::Value(*variable));
                self.report_usage(collection);
            }
            _ => {}
        }
    }

    fn report_usage(&mut self, variable: &PPEExpr) {
        // A member assignment names the record only at the base of the expression.
        let mut variable = variable;
        while let PPEExpr::Member(base, _) | PPEExpr::IndexedMember(base, _, _) = variable {
            variable = base;
        }
        if let Some(id) = variable.get_id()
            && id < self.entries.len() + 1
            && id > 0
        {
            self.get_var_entry_mut(id).report_variable_usage();
        }
    }

    pub fn push(&mut self, entry: TableEntry) {
        self.entries.push(entry);
    }

    pub fn set_value(&mut self, id: usize, value: VariableValue) {
        let val = value.convert_to(self.entries[id - 1].value.vtype);
        self.get_var_entry_mut(id).value = val;
    }

    pub fn get_value(&self, id: usize) -> &VariableValue {
        &self.get_var_entry(id).value
    }

    pub fn get_value_mut(&mut self, id: usize) -> &mut VariableValue {
        &mut self.get_var_entry_mut(id).value
    }

    pub fn try_get_value(&self, id: usize) -> Option<&VariableValue> {
        if id == 0 || id > self.entries.len() {
            return None;
        }
        Some(self.get_value(id))
    }

    pub fn get_var_entry(&self, id: usize) -> &TableEntry {
        assert!(
            id > 0 && id <= self.entries.len(),
            "Invalid variable id: {} #entries: {}",
            id,
            self.entries.len()
        );
        &self.entries[id - 1]
    }

    pub fn get_var_entry_mut(&mut self, id: usize) -> &mut TableEntry {
        assert!(
            id > 0 && id <= self.entries.len(),
            "Invalid variable id: {} #entries: {}",
            id,
            self.entries.len()
        );
        &mut self.entries[id - 1]
    }

    pub fn try_get_entry(&self, id: usize) -> Option<&TableEntry> {
        if id == 0 || id > self.entries.len() {
            return None;
        }
        Some(self.get_var_entry(id))
    }

    pub fn try_get_entry_mut(&mut self, id: usize) -> Option<&mut TableEntry> {
        if id == 0 || id > self.entries.len() {
            return None;
        }
        Some(self.get_var_entry_mut(id))
    }

    pub fn scan_user_variables_version(&self) -> u16 {
        for (i, u_var) in USER_VARIABLES.iter().enumerate() {
            if i >= self.entries.len()
                || self.entries[i].header.variable_type != u_var.value.vtype
                || self.entries[i].header.dim != u_var.value.get_dimensions()
                || self.entries[i].header.vector_size != u_var.value.get_vector_size()
            {
                // workaround for a bug in 3.40 beta where U_BIRTHDATE was a string instead of a date.
                if i < self.entries.len() && u_var.name == "U_BIRTHDATE" && self.entries[i].header.variable_type == VariableType::String {
                    continue;
                }
                let res = if u_var.runtime_version > 340 {
                    340
                } else if u_var.runtime_version > 300 {
                    300
                } else if u_var.runtime_version > 100 {
                    100
                } else {
                    0
                };
                return res;
            }
        }
        LAST_PPE_RUNTIME
    }

    pub fn print_variable_table(&self) {
        println!();
        execute!(
            stdout(),
            Print("Variable Table ".to_string()),
            SetAttribute(Attribute::Bold),
            Print(format!("{}", self.len())),
            SetAttribute(Attribute::Reset),
            Print(" variables\n\n".to_string())
        )
        .unwrap();

        println!("   # Type         Flags Role           Name        Value");
        println!("---------------------------------------------------------------------------------------");
        for var in self.entries.iter().rev() {
            let ts = if var.header.dim > 0 {
                format!("{}({})", var.header.variable_type, var.header.dim)
            } else {
                var.header.variable_type.to_string()
            };
            execute!(
                stdout(),
                SetForegroundColor(Color::Green),
                Print(format!("{:04X} ", var.header.id)),
                SetAttribute(Attribute::Reset),
            )
            .unwrap();

            execute!(
                stdout(),
                SetForegroundColor(Color::Yellow),
                Print(format!("{ts:<13}")),
                SetAttribute(Attribute::Reset),
            )
            .unwrap();

            let ts = format!("{:?}", var.get_type());

            execute!(
                stdout(),
                SetAttribute(Attribute::Bold),
                Print(format!("{}", var.header.flags)),
                SetAttribute(Attribute::Reset),
            )
            .unwrap();

            print!("     {ts:<15}");
            execute!(
                stdout(),
                SetForegroundColor(Color::Magenta),
                Print(format!("{:<12}", var.get_name())),
                SetAttribute(Attribute::Reset),
            )
            .unwrap();

            if var.header.variable_type == VariableType::Function {
                unsafe {
                    execute!(
                        stdout(),
                        SetAttribute(Attribute::Bold),
                        Print(format!("{:?}", var.value.data.function_value)),
                        SetAttribute(Attribute::Reset)
                    )
                    .unwrap();
                }
            } else if var.header.variable_type == VariableType::Procedure {
                unsafe {
                    execute!(
                        stdout(),
                        SetAttribute(Attribute::Bold),
                        Print(format!("{:?}", var.value.data.procedure_value)),
                        SetAttribute(Attribute::Reset)
                    )
                    .unwrap();
                }
            } else if var.header.dim > 0 {
                let d = match var.header.dim {
                    1 => format!("{}", var.header.vector_size),
                    2 => format!("{}, {}", var.header.vector_size, var.header.matrix_size),
                    _ => format!("{}, {}, {}", var.header.vector_size, var.header.matrix_size, var.header.cube_size),
                };
                execute!(
                    stdout(),
                    Print("[".to_string()),
                    SetAttribute(Attribute::Bold),
                    Print(d),
                    SetAttribute(Attribute::Reset),
                    Print("]".to_string()),
                )
                .unwrap();
            } else if matches!(
                var.header.variable_type,
                VariableType::String | VariableType::BigStr | VariableType::UnboundedString
            ) {
                execute!(
                    stdout(),
                    SetAttribute(Attribute::Bold),
                    Print(format!("\"{}\"", var.value)),
                    SetAttribute(Attribute::Reset)
                )
                .unwrap();
            } else {
                execute!(
                    stdout(),
                    SetAttribute(Attribute::Bold),
                    Print(format!("{}", var.value)),
                    SetAttribute(Attribute::Reset)
                )
                .unwrap();
            }
            println!();
        }
    }

    /// .
    ///
    /// # Errors
    ///
    /// This function will return an error if .
    pub fn serialize(&self, buffer: &mut Vec<u8>) -> Result<(), ExecutableError> {
        if self.entries.len() > u16::MAX as usize {
            return Err(ExecutableError::TooManyDeclarations(self.entries.len()));
        }
        let max_var = u16::to_le_bytes(self.entries.len() as u16);
        buffer.extend_from_slice(&max_var);

        for d in self.entries.iter().rev() {
            let var_data = d.to_buffer(self.version)?;
            buffer.extend(var_data);
        }
        Ok(())
    }

    pub fn get_version(&self) -> u16 {
        self.version
    }
    pub fn set_version(&mut self, version: u16) {
        self.version = version;
    }

    pub fn get_entries(&self) -> &[TableEntry] {
        &self.entries
    }

    /// Gives every variable of a program declared type the fields its record has.
    /// The layout is not part of a variable's own entry, so it is filled in once the
    /// type table has been read.
    pub fn fill_in_records(&mut self, user_types: &[Vec<RecordField>]) {
        self.record_io_unsupported_types.clear();
        for (index, fields) in user_types.iter().enumerate() {
            let type_id = (crate::parser::FIRST_USER_TYPE_ID + index) as u32;
            let unsupported = fields.iter().any(|field| {
                if field.is_dynamic || field.element_count().is_none() {
                    return true;
                }
                match field.variable_type {
                    VariableType::UserData(id) if self.enums.contains_key(&id) => false,
                    VariableType::UserData(id) if crate::parser::is_user_declared_type(id) => id >= type_id || self.record_io_unsupported_types.contains(&id),
                    _ => !field.has_record_io_scalar_type(),
                }
            });
            if unsupported {
                self.record_io_unsupported_types.insert(type_id);
            }
        }
        for entry in &mut self.entries {
            let VariableType::UserData(type_id) = entry.header.variable_type else {
                continue;
            };
            if !crate::parser::is_user_declared_type(type_id) && !matches!(type_id as usize, crate::parser::CONTACT_ID | crate::parser::MSG_HEADER_ID) {
                continue;
            }
            let Some(value) = create_record_value(type_id, user_types, &self.enums) else {
                continue;
            };
            let array_data = self
                .enums
                .get(&type_id)
                .and_then(|values| values.first())
                .map_or_else(VariableData::default, |value| VariableData::from_int(*value));
            if entry.header.dim == 0 {
                entry.value = value;
            } else if self.version >= 400 && entry.header.flags & VARIABLE_FLAG_DYNAMIC_ARRAY != 0 {
                // Record layout initialization must not turn an empty dynamic
                // array into the classic one-element array with upper bound 0.
                entry.value.generic_data = entry.header.create_generic_data().unwrap_or_default();
                entry.value.data = array_data;
            } else if let Some(generic_data) = GenericVariableData::create_array(
                value,
                entry.header.dim,
                entry.header.vector_size,
                entry.header.matrix_size,
                entry.header.cube_size,
            ) {
                entry.value = VariableValue {
                    vtype: entry.header.variable_type,
                    data: array_data,
                    generic_data,
                };
            }
        }
    }

    pub(crate) fn analyze_locals(&mut self) {
        for t in &self.entries.clone() {
            if t.header.variable_type == VariableType::Function {
                unsafe {
                    // A routine with no start offset owns nothing, its variables stay global.
                    if t.value.data.function_value.start_offset == 0 {
                        continue;
                    }
                    if let Some(result) = self.try_get_entry_mut(t.value.data.function_value.return_var as usize) {
                        result.set_type(EntryType::FunctionResult);
                    }
                    let start = t.value.data.function_value.first_var_id as usize + t.value.data.function_value.parameters as usize + 1;
                    for i in 0..t.value.data.function_value.local_variables {
                        let idx = start + i as usize;
                        if idx == t.value.data.function_value.return_var as usize {
                            continue;
                        }

                        let var = self.get_var_entry_mut(idx);
                        if var.header.flags != 0 {
                            continue;
                        }
                        var.set_type(EntryType::LocalVariable);
                    }
                }
            } else if t.header.variable_type == VariableType::Procedure {
                unsafe {
                    if t.value.data.procedure_value.start_offset == 0 {
                        continue;
                    }
                    let start = t.value.data.procedure_value.first_var_id as usize + t.value.data.procedure_value.parameters as usize + 1;
                    for i in 0..t.value.data.procedure_value.local_variables {
                        let var = self.get_var_entry_mut(start + i as usize);
                        if var.header.flags != 0 {
                            continue;
                        }
                        var.set_type(EntryType::LocalVariable);
                    }
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn string_constant(length: usize) -> TableEntry {
        TableEntry::new(
            "text".to_string(),
            VarHeader {
                id: 1,
                variable_type: VariableType::String,
                ..Default::default()
            },
            VariableValue::new_string("x".repeat(length)),
            EntryType::Constant,
        )
    }

    #[test]
    fn string_constant_length_reserves_space_for_the_terminating_nul() {
        assert!(string_constant(u16::MAX as usize - 1).to_buffer(400).is_ok());
        assert!(matches!(
            string_constant(u16::MAX as usize).to_buffer(400),
            Err(ExecutableError::StringConstantTooLong(length)) if length == u16::MAX as usize
        ));
    }

    fn encoded_string(text: &str, version: u16) -> Vec<u8> {
        let mut entry = string_constant(0);
        entry.value = VariableValue::new_string(text.to_string());
        let mut buffer = 1u16.to_le_bytes().to_vec();
        buffer.extend(entry.to_buffer(version).unwrap());
        buffer
    }

    #[test]
    fn s5_literal_encoding_is_selected_by_runtime() {
        for version in [100, 300, 340, 400] {
            let text = if version >= 400 {
                "\u{20ac}\u{754c}e\u{301}\u{1f600}\0end"
            } else {
                "\u{e9}\u{2591}\u{2502}"
            };
            let mut buffer = encoded_string(text, version);
            let mut payload = buffer[15..].to_vec();
            decrypt_chunks(&mut payload, version, false);
            let expected = if version >= 400 { text.as_bytes().to_vec() } else { vec![0x82, 0xb0, 0xb3] };
            assert_eq!(&payload[..payload.len() - 1], expected, "runtime {version}");
            assert_eq!(payload.last(), Some(&0));
            let (consumed, table) = VariableTable::deserialize(version, &mut buffer).unwrap();
            assert_eq!(consumed, buffer.len());
            assert_eq!(table.get_value(1).as_string(), text);
        }
    }

    #[test]
    fn s5_literal_utf8_byte_limits() {
        for byte_length in [65533, 65534, 65535] {
            let mut entry = string_constant(0);
            entry.value = VariableValue::new_string(format!("{}{}", "\u{1f600}".repeat(byte_length / 4), "x".repeat(byte_length % 4)));
            let result = entry.to_buffer(400);
            if byte_length < 65535 {
                let mut buffer = 1u16.to_le_bytes().to_vec();
                buffer.extend(result.unwrap());
                let (_, table) = VariableTable::deserialize(400, &mut buffer).unwrap();
                assert_eq!(table.get_value(1).as_string(), entry.value.as_string());
            } else {
                assert_eq!(result.unwrap_err(), ExecutableError::StringConstantTooLong(byte_length));
            }
        }
    }

    #[test]
    fn s5_literal_loader_rejects_malformed_utf8_and_frames() {
        for payload in [
            vec![0xff],
            vec![0xc0, 0x80],
            vec![0xed, 0xa0, 0x80],
            vec![0xf4, 0x90, 0x80, 0x80],
            vec![0xe2, 0x82],
        ] {
            let mut buffer = encoded_string(&"x".repeat(payload.len()), 400);
            buffer[15..15 + payload.len()].copy_from_slice(&payload);
            let error = VariableTable::deserialize(400, &mut buffer).err().unwrap();
            assert_eq!(
                error.downcast_ref::<ExecutableError>(),
                Some(&ExecutableError::InvalidStringEncoding {
                    variable_id: 1,
                    valid_up_to: 0
                })
            );
        }
        let mut buffer = encoded_string("valid", 400);
        *buffer.last_mut().unwrap() = b'x';
        let error = VariableTable::deserialize(400, &mut buffer).err().unwrap();
        assert_eq!(error.downcast_ref::<ExecutableError>(), Some(&ExecutableError::InvalidStringTerminator(1)));
        let buffer = encoded_string("\u{20ac}", 400);
        for end in 0..buffer.len() {
            assert!(VariableTable::deserialize(400, &mut buffer[..end].to_vec()).is_err(), "truncated at {end}");
        }
        for text in ["", "ASCII", "\0"] {
            let (_, table) = VariableTable::deserialize(400, &mut encoded_string(text, 400)).unwrap();
            assert_eq!(table.get_value(1).as_string(), text);
        }
    }
}

pub struct UserVariable {
    pub name: &'static str,
    pub runtime_version: u16,
    pub value: VariableValue,
}

pub static USER_VARIABLES: std::sync::LazyLock<[UserVariable; 29]> = std::sync::LazyLock::new(|| {
    [
        UserVariable {
            name: "U_EXPERT",
            runtime_version: 100,
            value: VariableValue::new_bool(false),
        },
        UserVariable {
            name: "U_FSE",
            runtime_version: 100,
            value: VariableValue::new_bool(false),
        },
        UserVariable {
            name: "U_FSEP",
            runtime_version: 100,
            value: VariableValue::new_bool(false),
        },
        UserVariable {
            name: "U_CLS",
            runtime_version: 100,
            value: VariableValue::new_bool(false),
        },
        UserVariable {
            name: "U_EXPDATE",
            runtime_version: 100,
            value: VariableValue::new(VariableType::Date, VariableData::default()),
        },
        UserVariable {
            name: "U_SEC",
            runtime_version: 100,
            value: VariableValue::new_int(0),
        },
        UserVariable {
            name: "U_PAGELEN",
            runtime_version: 100,
            value: VariableValue::new_int(0),
        },
        UserVariable {
            name: "U_EXPSEC",
            runtime_version: 100,
            value: VariableValue::new_int(0),
        },
        UserVariable {
            name: "U_CITY",
            runtime_version: 100,
            value: VariableValue::new_string(String::new()),
        },
        UserVariable {
            name: "U_BDPHONE",
            runtime_version: 100,
            value: VariableValue::new_string(String::new()),
        },
        UserVariable {
            name: "U_HVPHONE",
            runtime_version: 100,
            value: VariableValue::new_string(String::new()),
        },
        UserVariable {
            name: "U_TRANS",
            runtime_version: 100,
            value: VariableValue::new_string(String::new()),
        },
        UserVariable {
            name: "U_CMNT1",
            runtime_version: 100,
            value: VariableValue::new_string(String::new()),
        },
        UserVariable {
            name: "U_CMNT2",
            runtime_version: 100,
            value: VariableValue::new_string(String::new()),
        },
        UserVariable {
            name: "U_PWD",
            runtime_version: 100,
            value: VariableValue::new_string(String::new()),
        },
        UserVariable {
            name: "U_SCROLL",
            runtime_version: 100,
            value: VariableValue::new_bool(false),
        },
        UserVariable {
            name: "U_LONGHDR",
            runtime_version: 100,
            value: VariableValue::new_bool(false),
        },
        UserVariable {
            name: "U_DEF79",
            runtime_version: 100,
            value: VariableValue::new_bool(false),
        },
        UserVariable {
            name: "U_ALIAS",
            runtime_version: 100,
            value: VariableValue::new_string(String::new()),
        },
        UserVariable {
            name: "U_VER",
            runtime_version: 100,
            value: VariableValue::new_string(String::new()),
        },
        UserVariable {
            name: "U_ADDR",
            runtime_version: 100,
            value: VariableValue::new_vector(VariableType::String, vec![VariableValue::new_string(String::new()); 5 + 1]),
        },
        UserVariable {
            name: "U_NOTES",
            runtime_version: 100,
            value: VariableValue::new_vector(VariableType::String, vec![VariableValue::new_string(String::new()); 4 + 1]),
        },
        UserVariable {
            name: "U_PWDEXP",
            runtime_version: 100,
            value: VariableValue::new(VariableType::Date, VariableData::default()),
        },
        UserVariable {
            name: "U_ACCOUNT",
            runtime_version: 300,
            value: VariableValue::new_vector(VariableType::Integer, vec![VariableValue::new_int(0); 16 + 1]),
        },
        UserVariable {
            name: "U_SHORTDESC",
            runtime_version: 340,
            value: VariableValue::new_bool(false),
        },
        UserVariable {
            name: "U_GENDER",
            runtime_version: 340,
            value: VariableValue::new_string(String::new()),
        },
        UserVariable {
            name: "U_BIRTHDATE",
            runtime_version: 340,
            value: VariableValue::new_string(String::new()),
        },
        UserVariable {
            name: "U_EMAIL",
            runtime_version: 340,
            value: VariableValue::new_string(String::new()),
        },
        UserVariable {
            name: "U_WEB",
            runtime_version: 340,
            value: VariableValue::new_string(String::new()),
        },
    ]
});
