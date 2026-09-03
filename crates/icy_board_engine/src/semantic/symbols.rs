use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::Arc,
};

use crate::{
    ast::{Constant, FunctionDeclarationAstNode, ParameterSpecifier, ProcedureDeclarationAstNode},
    executable::{EntryType, FuncOpCode, GenericVariableData, OpCode, TableEntry, VarHeader, VariableType},
    parser::lexer::Spanned,
};

#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceType {
    PredefinedFunc(FuncOpCode),
    PredefinedProc(OpCode),
    Label(usize),
    Variable(usize),
    Constant(usize),
    Function(usize),
    Procedure(usize),
}

pub(super) fn parameter_lists_match(expected: &[ParameterSpecifier], actual: &[ParameterSpecifier]) -> bool {
    expected.len() == actual.len()
        && expected
            .iter()
            .zip(actual)
            .all(|(expected, actual)| parameter_signature_matches(expected, actual))
}

fn parameter_signature_matches(expected: &ParameterSpecifier, actual: &ParameterSpecifier) -> bool {
    match (expected, actual) {
        (ParameterSpecifier::Variable(expected), ParameterSpecifier::Variable(actual)) => {
            if expected.get_variable_type() != actual.get_variable_type() || expected.is_var() != actual.is_var() {
                return false;
            }
            match (expected.get_variable(), actual.get_variable()) {
                (Some(expected), Some(actual)) => {
                    expected.get_dimensions().len() == actual.get_dimensions().len()
                        && expected
                            .get_dimensions()
                            .iter()
                            .zip(actual.get_dimensions())
                            .all(|(expected, actual)| expected.get_dimension() == actual.get_dimension())
                }
                (None, None) => true,
                _ => false,
            }
        }
        (ParameterSpecifier::Function(expected), ParameterSpecifier::Function(actual)) => {
            expected.get_return_type() == actual.get_return_type() && parameter_lists_match(expected.get_parameters(), actual.get_parameters())
        }
        (ParameterSpecifier::Procedure(expected), ParameterSpecifier::Procedure(actual)) => {
            parameter_lists_match(expected.get_parameters(), actual.get_parameters())
        }
        _ => false,
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum SemanticInfo {
    PredefinedFunc(FuncOpCode),
    MemberFunctionCall(usize),
    MemberSetterCall(usize),
    IndexedRecordField(usize),
    ArrayMemberFunc(FuncOpCode, Vec<i32>),
    ScalarMemberFunc(FuncOpCode, &'static [i32]),
    ArrayValueAt,
    ScalarStaticFunc(FuncOpCode),
    ArrayMemberProc(OpCode),
    PredefFunctionGroup(Vec<usize>),
    FunctionReference(usize),
    VariableReference(usize),
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct References {
    pub variable_type: VariableType,
    pub variable_table_index: usize,
    pub header: Option<VarHeader>,
    pub declaration: Option<(Arc<PathBuf>, Spanned<String>)>,
    pub implementation: Option<(Arc<PathBuf>, Spanned<String>)>,
    pub return_types: Vec<(Arc<PathBuf>, Spanned<String>)>,
    pub usages: Vec<(Arc<PathBuf>, Spanned<String>)>,
}

impl References {
    pub fn contains_pos(&self, path: &PathBuf, offset: usize) -> bool {
        for (reference_path, range) in &self.usages {
            if reference_path.as_ref() == path && range.span.contains(&offset) {
                return true;
            }
        }
        for (reference_path, range) in &self.return_types {
            if reference_path.as_ref() == path && range.span.contains(&offset) {
                return true;
            }
        }
        if let Some((reference_path, declaration)) = &self.implementation {
            if reference_path.as_ref() != path {
                return false;
            }
            if declaration.span.contains(&offset) {
                return true;
            }
        }
        if let Some((reference_path, declaration)) = &self.declaration {
            reference_path.as_ref() == path && declaration.span.contains(&offset)
        } else {
            false
        }
    }

    pub(super) fn create_table_entry(&self) -> TableEntry {
        self.create_table_entry_as(self.variable_type)
    }

    pub(super) fn create_table_entry_as(&self, storage_type: VariableType) -> TableEntry {
        let Some(header) = &self.header else {
            panic!("Header not set for {self:?}");
        };
        let mut header = header.clone();
        header.variable_type = storage_type;
        if let Some((_, declaration)) = self.declaration.as_ref() {
            TableEntry::new(declaration.token.clone(), header, storage_type.create_empty_value(), EntryType::Variable)
        } else if let Some((_, usage)) = self.usages.first() {
            TableEntry::new(usage.token.clone(), header, storage_type.create_empty_value(), EntryType::Variable)
        } else {
            panic!("Can't find declaration for {self:?}");
        }
    }
}

#[derive(Clone)]
pub enum FunctionDeclaration {
    Function(FunctionDeclarationAstNode),
    Procedure(ProcedureDeclarationAstNode),
}

#[derive(Clone)]
pub struct FunctionContainer {
    pub name: unicase::Ascii<String>,
    pub parameter_index: Option<usize>,
    pub id: usize,
    pub functions: FunctionDeclaration,
    pub lookup: VariableLookups,
    pub parameters: core::ops::Range<usize>,
    pub local_variables: core::ops::Range<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ModuleSymbolKind {
    Function,
    Procedure,
    Type,
    Enum,
    Variable,
    Constant,
}

#[derive(Debug, Clone)]
pub struct ModuleExport {
    pub name: String,
    pub kind: ModuleSymbolKind,
}

#[derive(Default, Clone)]
pub struct VariableLookups {
    pub variable_lookup: HashMap<unicase::Ascii<String>, usize>,
    pub(super) constants: Vec<Constant>,
    pub const_lookup_table: HashSet<(VariableType, u64)>,
    pub string_lookup_table: HashSet<String>,
}

impl VariableLookups {
    pub fn add_constant(&mut self, constant: &Constant) {
        let value = constant.get_value();
        if let GenericVariableData::String(string) = &value.generic_data {
            if self.string_lookup_table.insert(string.as_ref().clone()) {
                self.constants.push(constant.clone());
            }
        } else {
            unsafe {
                let key = (constant.get_var_type(), value.data.u64_value);
                if self.const_lookup_table.insert(key) {
                    self.constants.push(constant.clone());
                }
            }
        }
    }
}
