use std::collections::HashMap;

use crate::executable::{EntryType, GenericVariableData, TableEntry, VarHeader, VariableTable, VariableType};

use super::Constant;

type NameTableLookup = HashMap<unicase::Ascii<String>, usize>;

#[derive(Default)]
pub struct LookupVariabeleTable {
    pub variable_table: VariableTable,
    variable_lookup: NameTableLookup,

    local_variable_lookup: Option<unicase::Ascii<String>>,
    local_lookups: HashMap<unicase::Ascii<String>, NameTableLookup>,

    const_lookup_table: HashMap<(VariableType, u64), usize>,
    string_lookup_table: HashMap<String, usize>,
}

impl LookupVariabeleTable {
    pub fn push(&mut self, mut entry: TableEntry) -> usize {
        let id = self.variable_table.len() + 1;
        entry.header.id = id;
        let name = unicase::Ascii::new(entry.name.clone());
        if let Some(local) = &self.local_variable_lookup {
            self.local_lookups.get_mut(local).unwrap().insert(name, entry.header.id);
        } else {
            self.variable_lookup.insert(name, entry.header.id);
        }
        self.variable_table.push(entry);
        id
    }

    pub fn len(&self) -> usize {
        self.variable_table.len()
    }

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.variable_table.is_empty()
    }

    pub fn lookup_variable_index(&self, identifier: &unicase::Ascii<String>) -> Option<usize> {
        if let Some(local) = &self.local_variable_lookup
            && let Some(index) = self.local_lookups.get(local).unwrap().get(identifier)
        {
            return Some(*index);
        }
        self.variable_lookup.get(identifier).copied()
    }

    pub fn has_variable(&self, identifier: &unicase::Ascii<String>) -> bool {
        self.lookup_variable_index(identifier).is_some()
    }

    pub(crate) fn start_compile_function_body(&mut self, identifier: &unicase::Ascii<String>) {
        self.local_variable_lookup = Some(identifier.clone());
    }

    pub(crate) fn end_compile_function_body(&mut self) {
        self.local_variable_lookup = None;
    }

    pub fn lookup_variable(&self, identifier: &unicase::Ascii<String>) -> Option<&TableEntry> {
        if let Some(local) = self.lookup_variable_index(identifier) {
            self.variable_table.try_get_entry(local)
        } else {
            None
        }
    }

    pub fn lookup_constant(&mut self, constant: &Constant) -> usize {
        let value = constant.get_value();

        if let GenericVariableData::String(value) = &value.generic_data {
            if let Some(id) = self.string_lookup_table.get(value.as_str()) {
                return *id;
            }
        } else {
            unsafe {
                let key = (constant.get_var_type(), value.data.u64_value);
                if let Some(id) = self.const_lookup_table.get(&key) {
                    return *id;
                }
            }
        }
        log::error!("Constant not found {constant:?}");
        0
    }

    pub(super) fn start_define_function_body(&mut self, identifier: unicase::Ascii<String>) {
        self.local_variable_lookup = Some(identifier.clone());
        self.local_lookups.insert(identifier, NameTableLookup::new());
    }

    pub(super) fn add_constant(&mut self, constant: &Constant) {
        let value = constant.get_value();
        if let GenericVariableData::String(value) = &value.generic_data {
            if self.string_lookup_table.contains_key(value.as_str()) {
                return;
            }
        } else {
            unsafe {
                let key = (constant.get_var_type(), value.data.u64_value);
                if self.const_lookup_table.contains_key(&key) {
                    return;
                }
            }
        }

        let header = VarHeader {
            id: 0,
            variable_type: constant.get_var_type(),
            dim: 0,
            vector_size: 0,
            matrix_size: 0,
            cube_size: 0,
            flags: 0,
        };

        let const_num = self.string_lookup_table.len() + self.const_lookup_table.len() + 1;
        let entry = TableEntry::new(format!("CONST_{}", const_num + 1), header, value.clone(), EntryType::Constant);
        let id = self.push(entry);
        if let GenericVariableData::String(value) = value.generic_data {
            self.string_lookup_table.insert(std::sync::Arc::unwrap_or_clone(value), id);
        } else {
            unsafe {
                let key = (constant.get_var_type(), value.data.u64_value);
                self.const_lookup_table.insert(key, id);
            }
        }
    }
}
