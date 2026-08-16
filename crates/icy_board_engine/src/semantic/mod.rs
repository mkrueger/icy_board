use core::panic;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    ast::{
        AstVisitor, CommentAstNode, ConstDeclarationStatement, Constant, ConstantExpression, EnumDeclarationAstNode, Expression, FunctionCallExpression,
        FunctionDeclarationAstNode, FunctionImplementation, GosubStatement, GotoStatement, IdentifierExpression, LabelStatement, LetStatement,
        ParameterSpecifier, PredefinedCallStatement, ProcedureCallStatement, ProcedureDeclarationAstNode, ProcedureImplementation, TypeDeclarationAstNode,
        VariableDeclarationStatement, VariableParameterSpecifier, const_value_with_members, walk_function_implementation, walk_indexer_expression,
        walk_predefined_call_statement, walk_procedure_call_statement, walk_procedure_implementation,
    },
    compiler::{CompilationErrorType, CompilationWarningType, user_data::UserDataMemberRegistry, workspace::Workspace},
    executable::{
        EntryType, FIRST_RECORD_LITERAL_RUNTIME, FIRST_ROUTINE_REFERENCE_RUNTIME, FIRST_TYPE_TABLE_RUNTIME, FUNCTION_DEFINITIONS, FuncOpCode,
        FunctionDefinition, FunctionValue, GenericVariableData, OpCode, ProcedureValue, TableEntry, USER_VARIABLES, VarHeader, VariableData, VariableTable,
        VariableType, VariableValue,
    },
    parser::{
        self, ErrorReporter, ParserErrorType, UserTypeRegistry,
        lexer::{Spanned, Token},
    },
};

#[cfg(test)]
mod find_references_tests;

#[derive(Debug, Clone, PartialEq)]
pub enum ReferenceType {
    PredefinedFunc(FuncOpCode),
    PredefinedProc(OpCode),
    Label(usize),
    Variable(usize),

    Function(usize),
    Procedure(usize),
}

fn parameter_lists_match(expected: &[ParameterSpecifier], actual: &[ParameterSpecifier]) -> bool {
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

    PredefFunctionGroup(Vec<usize>),

    /// id looks up into 'function_containers'
    FunctionReference(usize),

    /// id looks up into 'references'
    VariableReference(usize),
}

#[derive(Default, Debug, Clone, PartialEq)]
pub struct References {
    pub variable_type: VariableType,

    pub variable_table_index: usize,

    pub header: Option<VarHeader>,

    pub declaration: Option<(PathBuf, Spanned<String>)>,
    pub implementation: Option<(PathBuf, Spanned<String>)>,
    pub return_types: Vec<(PathBuf, Spanned<String>)>,

    pub usages: Vec<(PathBuf, Spanned<String>)>,
}

impl References {
    pub fn contains_pos(&self, path: &PathBuf, offset: usize) -> bool {
        for (p, r) in &self.usages {
            if p != path {
                continue;
            }
            if r.span.contains(&offset) {
                return true;
            }
        }

        for (p, r) in &self.return_types {
            if p != path {
                continue;
            }

            if r.span.contains(&offset) {
                return true;
            }
        }

        if let Some((p, decl)) = &self.implementation {
            if p != path {
                return false;
            }
            if decl.span.contains(&offset) {
                return true;
            }
        }
        if let Some((p, decl)) = &self.declaration {
            if p != path {
                return false;
            }
            decl.span.contains(&offset)
        } else {
            false
        }
    }

    fn create_table_entry(&self) -> TableEntry {
        self.create_table_entry_as(self.variable_type)
    }

    fn create_table_entry_as(&self, storage_type: VariableType) -> TableEntry {
        if let Some(header) = &self.header {
            let mut header = header.clone();
            header.variable_type = storage_type;
            if let Some((_, decl)) = self.declaration.as_ref() {
                TableEntry::new(decl.token.to_string(), header, storage_type.create_empty_value(), EntryType::Variable)
            } else if !self.usages.is_empty() {
                TableEntry::new(
                    self.usages.first().unwrap().1.token.to_string(),
                    header,
                    storage_type.create_empty_value(),
                    EntryType::Variable,
                )
            } else {
                panic!("Can't find declaration for {self:?}")
            }
        } else {
            panic!("Header not set for {self:?}")
        }
    }
}

type NameTableLookup = HashMap<unicase::Ascii<String>, usize>;

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

#[derive(Default, Clone)]
pub struct VariableLookups {
    pub variable_lookup: NameTableLookup,

    constants: Vec<Constant>,
    pub const_lookup_table: HashSet<(VariableType, u64)>,
    pub string_lookup_table: HashSet<String>,
}

impl VariableLookups {
    pub fn add_constant(&mut self, constant: &Constant) {
        let value = constant.get_value();
        if let GenericVariableData::String(str) = &value.generic_data {
            if self.string_lookup_table.insert(str.to_string()) {
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

pub struct SemanticVisitor {
    lang_version: u16,
    runtime: u16,
    pub type_registry: UserTypeRegistry,

    pub errors: Arc<Mutex<ErrorReporter>>,
    pub references: Vec<(ReferenceType, References)>,

    /// Maps member references -> user type IDs
    pub user_type_lookup: HashMap<usize, u8>,

    pub function_type_lookup: HashMap<u64, SemanticInfo>,

    pub require_user_variables: bool,
    allow_routine_reference: bool,
    allowed_routine_reference_spans: HashSet<usize>,

    // labels
    label_count: usize,
    label_lookup_table: NameTableLookup,

    // variables
    global_lookup: VariableLookups,

    local_variable_lookup: Option<VariableLookups>,

    /// Named constants never reach the variable table - the value takes the place of
    /// the name - so they are kept beside it.
    global_constants: HashMap<unicase::Ascii<String>, (VariableType, VariableValue)>,
    local_constants: Option<HashMap<unicase::Ascii<String>, (VariableType, VariableValue)>>,

    /// Where the FOR statements of the current file keep their count, which a
    /// desugared loop compares and steps itself.
    loop_counters: HashSet<usize>,

    // constants
    pub function_containers: Vec<FunctionContainer>,

    cur_func_impl: usize,
    cur_func_call: u64,

    last_lookup_index: usize,
}

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
    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
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

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn lookup_variable_index(&self, identifier: &unicase::Ascii<String>) -> Option<usize> {
        if let Some(local) = &self.local_variable_lookup {
            if let Some(c) = self.local_lookups.get(local).unwrap().get(identifier) {
                return Some(*c);
            }
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

        if let GenericVariableData::String(str) = &value.generic_data {
            if let Some(id) = self.string_lookup_table.get(str) {
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
        log::error!("Constant not found {:?}", constant);
        0
    }

    fn start_define_function_body(&mut self, identifer: unicase::Ascii<String>) {
        self.local_variable_lookup = Some(identifer.clone());
        self.local_lookups.insert(identifer, NameTableLookup::new());
    }

    fn add_constant(&mut self, constant: &Constant) {
        let value = constant.get_value();
        if let GenericVariableData::String(str) = &value.generic_data {
            if self.string_lookup_table.contains_key(str) {
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

        let header: VarHeader = VarHeader {
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
        if let GenericVariableData::String(str) = value.generic_data {
            self.string_lookup_table.insert(str, id);
        } else {
            unsafe {
                let key = (constant.get_var_type(), value.data.u64_value);
                self.const_lookup_table.insert(key, id);
            }
        }
    }
}

impl SemanticVisitor {
    fn storage_type(&self, source_type: VariableType) -> VariableType {
        if self.type_registry.is_enum_type(source_type) {
            VariableType::Integer
        } else {
            source_type
        }
    }

    fn source_type_name(&self, variable_type: VariableType) -> String {
        if let VariableType::UserData(id) = variable_type {
            if let Some(definition) = self.type_registry.get_enum_from_id(id) {
                return definition.name.to_string();
            }
        }
        variable_type.to_string()
    }

    pub fn set_loop_counters(&mut self, loop_counters: HashSet<usize>) {
        self.loop_counters = loop_counters;
    }

    /// True for the variable a desugared FOR counts with.
    fn counts_a_loop(&self, expr: &Expression) -> bool {
        matches!(expr, Expression::Identifier(identifier) if self.loop_counters.contains(&identifier.get_identifier_token().span.start))
    }

    /// The enum a constant belongs to, if it names one of its members or another
    /// constant of that type.
    fn declared_constant_type(&self, expr: &Expression) -> Option<VariableType> {
        match expr {
            Expression::Identifier(identifier) => self.lookup_constant(identifier.get_identifier()).map(|(variable_type, _)| *variable_type),
            Expression::MemberReference(member) => {
                let Expression::Identifier(base) = member.get_expression() else {
                    return None;
                };
                let definition = self.type_registry.get_enum(base.get_identifier())?;
                definition.value(member.get_identifier()).map(|_| VariableType::UserData(definition.id))
            }
            _ => None,
        }
    }
    pub fn is_routine_reference(&self, span_start: usize) -> bool {
        self.allowed_routine_reference_spans.contains(&span_start)
    }

    pub fn new(workspace: &Workspace, errors: Arc<Mutex<ErrorReporter>>, type_registry: UserTypeRegistry) -> Self {
        let mut result = Self {
            lang_version: workspace.language_version(),
            runtime: workspace.runtime(),
            errors,
            references: Vec::new(),
            type_registry,

            label_count: 0,
            label_lookup_table: HashMap::new(),
            user_type_lookup: HashMap::new(),
            function_type_lookup: HashMap::new(),

            global_lookup: VariableLookups::default(),
            local_variable_lookup: None,
            global_constants: HashMap::new(),
            local_constants: None,
            loop_counters: HashSet::new(),
            require_user_variables: false,
            allow_routine_reference: false,
            allowed_routine_reference_spans: HashSet::new(),
            cur_func_call: 0,
            cur_func_impl: 0,
            function_containers: Vec::new(),
            last_lookup_index: 0,
        };
        for user_var in USER_VARIABLES.iter() {
            if user_var.runtime_version <= workspace.runtime() {
                result.add_predefined_variable(user_var.name, &user_var.value);
            } else {
                break;
            }
        }
        result
    }

    /// Returns the generate variable table of this [`SemanticVisitor`].
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn generate_variable_table(&mut self) -> LookupVariabeleTable {
        let mut variable_table = LookupVariabeleTable::default();

        if self.require_user_variables {
            for user_var in USER_VARIABLES.iter() {
                if user_var.runtime_version <= self.lang_version {
                    let header = VarHeader {
                        id: 0,
                        variable_type: user_var.value.get_type(),
                        dim: user_var.value.get_dimensions(),
                        vector_size: user_var.value.get_vector_size(),
                        matrix_size: user_var.value.get_matrix_size(),
                        cube_size: user_var.value.get_cube_size(),
                        flags: 0,
                    };
                    let entry = TableEntry::new(user_var.name, header, user_var.value.clone(), EntryType::UserVariable);
                    variable_table.push(entry);
                } else {
                    break;
                }
            }
        }

        let mut variables: Vec<usize> = self.global_lookup.variable_lookup.values().map(|u| *u).collect();
        variables.sort();
        for i in variables {
            let storage_type = self.storage_type(self.references[i].1.variable_type);
            let (rt, r) = &mut self.references[i];
            if !matches!(rt, ReferenceType::Variable(_)) {
                continue;
            }
            if r.usages.is_empty() {
                continue;
            }

            // Skip user variables - they've already been added above
            // Check if this is a predefined user variable by checking if it has no declaration
            // but has usages (predefined variables have no declaration)
            if self.require_user_variables && r.declaration.is_none() {
                // This is a predefined variable that's being used
                // Find it in the already-added user variables and update the reference
                if let Some(name) = r.usages.first().map(|(_, s)| &s.token) {
                    if let Some(idx) = variable_table.lookup_variable_index(&unicase::Ascii::new(name.clone())) {
                        r.variable_table_index = idx;
                        continue;
                    }
                }
            }

            r.variable_table_index = variable_table.len() + 1;
            let entry = r.create_table_entry_as(storage_type);
            variable_table.push(entry);
        }

        for f in &self.function_containers.clone() {
            if f.parameter_index.is_some() {
                continue;
            }
            {
                let (_rt, r) = &mut self.references[f.id];
                if r.usages.is_empty() {
                    continue;
                }
                r.variable_table_index = variable_table.variable_table.len() + 1;
            }
            let mut locals = 0;
            for idx in f.local_variables.clone() {
                let (rt, _r) = &self.references[idx];
                if !matches!(rt, ReferenceType::Variable(_)) {
                    continue;
                }
                locals += 1;
            }
            let id = variable_table.variable_table.len() + 1;

            if let FunctionDeclaration::Function(func) = &f.functions {
                let header = VarHeader {
                    id: 0,
                    dim: 0,
                    vector_size: 0,
                    matrix_size: 0,
                    cube_size: 0,
                    variable_type: VariableType::Function,
                    flags: 0,
                };
                let function_value = FunctionValue {
                    parameters: f.parameters.len() as u8,
                    local_variables: locals + 1,
                    start_offset: 0,
                    first_var_id: id as i16,
                    return_var: id as i16 + locals as i16 + f.parameters.len() as i16 + 1,
                };
                variable_table.push(TableEntry::new(
                    f.name.to_string(),
                    header,
                    VariableValue {
                        vtype: VariableType::Function,
                        data: VariableData { function_value },
                        generic_data: GenericVariableData::None,
                    },
                    EntryType::Function,
                ));
                variable_table.start_define_function_body(func.get_identifier().clone());
            } else if let FunctionDeclaration::Procedure(proc) = &f.functions {
                let header = VarHeader {
                    id: 0,
                    dim: 0,
                    vector_size: 0,
                    matrix_size: 0,
                    cube_size: 0,
                    variable_type: VariableType::Procedure,
                    flags: 0,
                };
                let procedure_value = ProcedureValue {
                    parameters: f.parameters.len() as u8,
                    local_variables: locals,
                    start_offset: 0,
                    first_var_id: id as i16,
                    pass_flags: proc.get_pass_flags(),
                };
                variable_table.push(TableEntry::new(
                    f.name.to_string(),
                    header,
                    VariableValue {
                        vtype: VariableType::Procedure,
                        data: VariableData { procedure_value },
                        generic_data: GenericVariableData::None,
                    },
                    EntryType::Procedure,
                ));
                variable_table.start_define_function_body(proc.get_identifier().clone());
            };

            for idx in f.parameters.clone() {
                let storage_type = self.storage_type(self.references[idx].1.variable_type);
                let (rt, r) = &mut self.references[idx];
                if let ReferenceType::Function(func) = rt {
                    for f in &mut self.function_containers {
                        if f.id == *func {
                            f.parameter_index = Some(variable_table.len());
                            break;
                        }
                    }
                    r.variable_table_index = variable_table.len() + 1;
                    let mut new_entry = r.create_table_entry();
                    new_entry.entry_type = EntryType::Parameter;
                    let FunctionDeclaration::Function(signature) = &self.function_containers[*func].functions else {
                        unreachable!("function parameter has no function signature");
                    };
                    new_entry.value = VariableValue::new_function(FunctionValue {
                        parameters: signature.get_parameters().len() as u8,
                        ..FunctionValue::default()
                    });
                    variable_table.push(new_entry);
                    continue;
                }
                if let ReferenceType::Procedure(func) = rt {
                    for f in &mut self.function_containers {
                        if f.id == *func {
                            f.parameter_index = Some(variable_table.len());
                            break;
                        }
                    }
                    r.variable_table_index = variable_table.len() + 1;
                    let mut new_entry = r.create_table_entry();
                    new_entry.entry_type = EntryType::Parameter;
                    let FunctionDeclaration::Procedure(signature) = &self.function_containers[*func].functions else {
                        unreachable!("procedure parameter has no procedure signature");
                    };
                    new_entry.value = VariableValue::new_procedure(ProcedureValue {
                        parameters: signature.get_parameters().len() as u8,
                        pass_flags: signature.get_pass_flags(),
                        ..ProcedureValue::default()
                    });
                    variable_table.push(new_entry);
                    continue;
                }
                if !matches!(rt, ReferenceType::Variable(_)) {
                    continue;
                }
                let mut new_entry = r.create_table_entry_as(storage_type);
                new_entry.entry_type = EntryType::Parameter;
                variable_table.push(new_entry);
            }

            for idx in f.local_variables.clone() {
                let (rt, r) = &self.references[idx];
                if !matches!(rt, ReferenceType::Variable(_)) {
                    continue;
                }
                let mut new_entry = r.create_table_entry_as(self.storage_type(r.variable_type));
                new_entry.entry_type = EntryType::LocalVariable;
                variable_table.push(new_entry);
            }

            if let FunctionDeclaration::Function(f) = &f.functions {
                let return_type = f.get_return_type();
                let storage_type = self.storage_type(return_type);
                let header = VarHeader {
                    id,
                    dim: 0,
                    vector_size: 0,
                    matrix_size: 0,
                    cube_size: 0,
                    variable_type: storage_type,
                    flags: 0,
                };
                variable_table.push(TableEntry::new(
                    format!("{} result", f.get_identifier()),
                    header,
                    storage_type.create_empty_value(),
                    EntryType::Variable,
                ));
            }

            variable_table.end_compile_function_body();
        }

        for c in &self.global_lookup.constants {
            variable_table.add_constant(c);
        }
        for f in &self.function_containers {
            let (_rt, r) = &mut self.references[f.id];
            if r.usages.is_empty() {
                continue;
            }
            for c in &f.lookup.constants {
                variable_table.add_constant(c);
            }
        }
        variable_table
    }

    fn add_constant(&mut self, constant: &Constant) {
        if let Some(local_lookup) = &mut self.local_variable_lookup {
            local_lookup.add_constant(constant);
        } else {
            self.global_lookup.add_constant(constant);
        }
    }

    fn add_declaration(&mut self, variable_type: VariableType, identifier_token: &Spanned<parser::lexer::Token>) -> usize {
        let id = self.references.len();

        let reftype = match variable_type {
            VariableType::Function => ReferenceType::Function(self.function_containers.len()),
            VariableType::Procedure => ReferenceType::Procedure(self.function_containers.len()),
            _ => ReferenceType::Variable(id),
        };

        self.references.push((
            reftype,
            References {
                variable_type,
                variable_table_index: 0,
                implementation: None,
                header: None,
                return_types: vec![],
                declaration: Some((
                    self.errors.lock().unwrap().file_name().to_path_buf(),
                    Spanned::new(identifier_token.token.to_string(), identifier_token.span.clone()),
                )),
                usages: vec![],
            },
        ));
        return id;
    }

    fn add_reference(&mut self, reftype: ReferenceType, variable_type: VariableType, identifier_token: &Spanned<parser::lexer::Token>) {
        for (_i, r) in &mut self.references.iter_mut().enumerate() {
            if r.0 == reftype {
                r.1.usages.push((
                    self.errors.lock().unwrap().file_name().to_path_buf(),
                    Spanned::new(identifier_token.token.to_string(), identifier_token.span.clone()),
                ));
                return;
            }
        }
        self.references.push((
            reftype,
            References {
                declaration: None,
                implementation: None,
                header: None,
                return_types: vec![],

                variable_type,
                variable_table_index: 0,
                usages: vec![(
                    self.errors.lock().unwrap().file_name().to_path_buf(),
                    Spanned::new(identifier_token.token.to_string(), identifier_token.span.clone()),
                )],
            },
        ));
    }

    fn add_label_usage(&mut self, label_token: &Spanned<Token>) {
        let Token::Identifier(identifier) = &label_token.token else {
            log::error!("Invalid label token {:?}", label_token);
            return;
        };
        let idx = if let Some(idx) = self.label_lookup_table.get_mut(identifier) {
            *idx
        } else {
            self.label_count += 1;
            self.label_lookup_table.insert(identifier.clone(), self.label_count);
            self.label_count
        };

        self.add_reference(ReferenceType::Label(idx), VariableType::UserData(255), label_token);
    }

    fn set_label_declaration(&mut self, label_token: &Spanned<Token>) {
        let Token::Label(identifier) = &label_token.token else {
            log::error!("Invalid label token {:?}", label_token);
            return;
        };

        // begin is a pseudo label
        if *identifier == "~BEGIN~" {
            return;
        }

        let idx = if let Some(idx) = self.label_lookup_table.get_mut(identifier) {
            for r in &mut self.references {
                if r.0 == ReferenceType::Label(*idx) && r.1.declaration.is_some() {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(label_token.span.clone(), CompilationErrorType::LabelAlreadyDefined(identifier.to_string()));
                    return;
                }
            }
            *idx
        } else {
            self.label_count += 1;
            self.label_lookup_table.insert(identifier.clone(), self.label_count);
            self.label_count
        };
        let reftype = ReferenceType::Label(idx);
        let span = label_token.span.start + 1..label_token.span.end;

        for (_i, r) in &mut self.references.iter_mut().enumerate() {
            if r.0 == reftype {
                r.1.declaration = Some((
                    self.errors.lock().unwrap().file_name().to_path_buf(),
                    Spanned::new(label_token.token.to_string(), span),
                ));
                return;
            }
        }

        self.references.push((
            reftype,
            References {
                variable_type: VariableType::Integer,
                variable_table_index: 0,
                implementation: None,
                header: None,
                return_types: vec![],
                declaration: Some((
                    self.errors.lock().unwrap().file_name().to_path_buf(),
                    Spanned::new(label_token.token.to_string(), span),
                )),
                usages: vec![],
            },
        ));
    }

    fn start_parse_function_body(&mut self) {
        self.local_variable_lookup = Some(VariableLookups::default());
        self.local_constants = Some(HashMap::new());

        // TODO: clear the local label lookup on each new functions for future language versions?
        // self.label_lookup_table.clear();
    }

    fn end_parse_function_body(&mut self) -> Option<VariableLookups> {
        self.local_constants = None;
        self.local_variable_lookup.take()
    }

    fn has_variable_defined(&self, id: &unicase::Ascii<String>) -> bool {
        if let Some(local_lookup) = &self.local_variable_lookup {
            let local_name = self.local_constants.as_ref().is_some_and(|constants| constants.contains_key(id)) || local_lookup.variable_lookup.contains_key(id);
            let global_routine = self
                .global_lookup
                .variable_lookup
                .get(id)
                .is_some_and(|index| matches!(self.references[*index].0, ReferenceType::Function(_) | ReferenceType::Procedure(_)));
            return local_name || global_routine;
        }
        self.global_constants.contains_key(id) || self.global_lookup.variable_lookup.contains_key(id)
    }

    fn lookup_constant(&self, id: &unicase::Ascii<String>) -> Option<&(VariableType, VariableValue)> {
        if let Some(local) = &self.local_constants {
            if let Some(constant) = local.get(id) {
                return Some(constant);
            }
        }
        if self
            .local_variable_lookup
            .as_ref()
            .is_some_and(|lookup| lookup.variable_lookup.contains_key(id))
        {
            return None;
        }
        self.global_constants.get(id)
    }

    fn add_predefined_variable(&mut self, name: &str, val: &VariableValue) {
        if self.has_variable_defined(&unicase::Ascii::new(name.to_string())) {
            panic!("Variable {} already exists", name);
        }

        let val = val.clone();
        let id = self.references.len();
        let header = VarHeader {
            id,
            variable_type: val.get_type(),
            dim: val.get_dimensions(),
            vector_size: val.get_vector_size(),
            matrix_size: val.get_matrix_size(),
            cube_size: val.get_cube_size(),
            flags: 0,
        };
        self.references.push((
            ReferenceType::Variable(id),
            References {
                variable_type: val.get_type(),
                variable_table_index: 0,
                header: Some(header),
                declaration: None,
                implementation: None,
                return_types: vec![],
                usages: vec![],
            },
        ));
        self.global_lookup.variable_lookup.insert(unicase::Ascii::new(name.to_string()), id);
    }

    fn add_variable(
        &mut self,
        variable_type: VariableType,
        identifier: &Spanned<parser::lexer::Token>,
        dim: u8,
        vector_size: usize,
        matrix_size: usize,
        cube_size: usize,
    ) {
        let id = self.add_declaration(variable_type, identifier);

        let header = VarHeader {
            id,
            variable_type,
            dim,
            vector_size,
            matrix_size,
            cube_size,
            flags: 0,
        };
        self.references.last_mut().unwrap().1.header = Some(header);

        if self.has_variable_defined(&unicase::Ascii::new(identifier.token.to_string())) {
            panic!("Variable {} already exists", identifier.token.to_string());
        }

        if let Some(local_lookup) = &mut self.local_variable_lookup {
            local_lookup.variable_lookup.insert(unicase::Ascii::new(identifier.token.to_string()), id);
        } else {
            self.global_lookup.variable_lookup.insert(unicase::Ascii::new(identifier.token.to_string()), id);
        }
    }

    fn lookup_variable(&mut self, id: &unicase::Ascii<String>) -> Option<usize> {
        if let Some(local_lookup) = &self.local_variable_lookup {
            if let Some(idx) = local_lookup.variable_lookup.get(id) {
                self.last_lookup_index = *idx;
                return Some(*idx);
            }
        }

        if let Some(idx) = self.global_lookup.variable_lookup.get(id) {
            self.last_lookup_index = *idx;
            return Some(*idx);
        }
        None
    }

    fn is_whole_custom_type_array(&mut self, expression: &Expression) -> bool {
        match expression {
            Expression::Identifier(identifier) => {
                let Some(index) = self.lookup_variable(identifier.get_identifier()) else {
                    return false;
                };
                let reference = &self.references[index].1;
                matches!(reference.variable_type, VariableType::UserData(_)) && reference.header.as_ref().is_some_and(|header| header.dim > 0)
            }
            Expression::Parens(parens) => self.is_whole_custom_type_array(parens.get_expression()),
            _ => false,
        }
    }

    fn add_reference_to(&mut self, identifier: &Spanned<Token>, idx: usize) {
        self.references[idx].1.usages.push((
            self.errors.lock().unwrap().file_name().to_path_buf(),
            Spanned::new(identifier.token.to_string(), identifier.span.clone()),
        ));
    }

    fn add_parameters(&mut self, parameters: &[ParameterSpecifier]) {
        for (i, param) in parameters.iter().enumerate() {
            match param {
                ParameterSpecifier::Variable(param) => {
                    let id = self.add_declaration(param.get_variable_type(), param.get_variable().as_ref().unwrap().get_identifier_token());
                    self.references[id].1.header = Some(VarHeader {
                        id,
                        variable_type: param.get_variable_type(),
                        dim: 0,
                        vector_size: 0,
                        matrix_size: 0,
                        cube_size: 0,
                        flags: 0,
                    });

                    self.local_variable_lookup
                        .as_mut()
                        .unwrap()
                        .variable_lookup
                        .insert(unicase::Ascii::new(param.get_variable().as_ref().unwrap().get_identifier().to_string()), id);
                }
                ParameterSpecifier::Function(func) => {
                    let id = self.add_declaration(VariableType::Function, func.get_identifier_token());
                    self.references[id].1.header = Some(VarHeader {
                        id,
                        variable_type: VariableType::Function,
                        dim: func.get_parameters().len() as u8,
                        vector_size: 0,
                        matrix_size: 0,
                        cube_size: 0,
                        flags: 0,
                    });
                    self.local_variable_lookup
                        .as_mut()
                        .unwrap()
                        .variable_lookup
                        .insert(unicase::Ascii::new(func.get_identifier().to_string()), id);

                    self.references[id].1.implementation = Some((
                        self.errors.lock().unwrap().file_name().to_path_buf(),
                        Spanned::new(func.get_identifier().to_string(), func.get_identifier_token().span.clone()),
                    ));
                    self.function_containers.push(FunctionContainer {
                        name: func.get_identifier().clone(),
                        parameter_index: Some(i),
                        id,
                        functions: FunctionDeclaration::Function(FunctionDeclarationAstNode::empty(
                            func.get_identifier().clone(),
                            func.get_parameters().clone(),
                            func.get_return_type(),
                        )),
                        lookup: VariableLookups::default(),
                        parameters: 0..0,
                        local_variables: 0..0,
                    });
                }
                ParameterSpecifier::Procedure(func) => {
                    let id = self.add_declaration(VariableType::Procedure, func.get_identifier_token());
                    self.references[id].1.header = Some(VarHeader {
                        id,
                        variable_type: VariableType::Procedure,
                        dim: func.get_parameters().len() as u8,
                        vector_size: 0,
                        matrix_size: 0,
                        cube_size: 0,
                        flags: 0,
                    });
                    self.local_variable_lookup
                        .as_mut()
                        .unwrap()
                        .variable_lookup
                        .insert(unicase::Ascii::new(func.get_identifier().to_string()), id);

                    self.references[id].1.implementation = Some((
                        self.errors.lock().unwrap().file_name().to_path_buf(),
                        Spanned::new(func.get_identifier().to_string(), func.get_identifier_token().span.clone()),
                    ));
                    self.function_containers.push(FunctionContainer {
                        name: func.get_identifier().clone(),
                        parameter_index: Some(i),
                        id,
                        functions: FunctionDeclaration::Procedure(ProcedureDeclarationAstNode::empty(
                            func.get_identifier().clone(),
                            func.get_parameters().clone(),
                        )),
                        lookup: VariableLookups::default(),
                        parameters: 0..0,
                        local_variables: 0..0,
                    });
                }
            }
        }
    }

    fn check_argument_is_variable(&mut self, arg_num: usize, expr: &Expression) {
        // that the identifier/dim is in the vtable is checked in argument evaluation
        if let Expression::Identifier(_) = expr {
            return;
        }

        if let Expression::FunctionCall(a) = expr {
            if let Some(SemanticInfo::VariableReference(_)) = self.function_type_lookup.get(&a.id) {
                return;
            }
        }
        if let Expression::Indexer(_) = expr {
            return;
        }

        self.errors
            .lock()
            .unwrap()
            .report_error(expr.get_span().clone(), CompilationErrorType::VariableExpected(arg_num + 1));
    }

    /// Resolves a field of a record the program declared and remembers the type, so
    /// code generation can look the field up again by the member's source position.
    fn resolve_record_field(&mut self, type_id: u8, member: &unicase::Ascii<String>, span: &core::ops::Range<usize>) -> VariableType {
        let Some(definition) = self.type_registry.get_user_type_from_id(type_id) else {
            self.errors.lock().unwrap().report_error(span.clone(), CompilationErrorType::TypeNotFound);
            return VariableType::None;
        };
        let Some(index) = definition.field_index(member) else {
            self.errors.lock().unwrap().report_error(
                span.clone(),
                CompilationErrorType::RecordMemberNotFound(VariableType::UserData(type_id), member.to_string()),
            );
            return VariableType::None;
        };
        self.user_type_lookup.insert(span.start, type_id);
        definition.field_type(index).unwrap_or(VariableType::None)
    }

    fn check_arg_count(&mut self, arg_count_expected: usize, arg_count: usize, identifier_token: &Spanned<Token>) {
        if arg_count < arg_count_expected {
            self.errors.lock().unwrap().report_error(
                identifier_token.span.clone(),
                ParserErrorType::TooFewArguments(identifier_token.token.to_string(), arg_count, arg_count_expected as i8),
            );
        }
        if arg_count > arg_count_expected {
            self.errors.lock().unwrap().report_error(
                identifier_token.span.clone(),
                ParserErrorType::TooManyArguments(identifier_token.token.to_string(), arg_count, arg_count_expected as i8),
            );
        }
    }

    fn check_expr_arg_count(&self, arg_count_expected: usize, arg_count: usize, expr: &Expression) {
        if arg_count < arg_count_expected {
            self.errors.lock().unwrap().report_error(
                expr.get_span(),
                ParserErrorType::TooFewArguments(expr.to_string(), arg_count, arg_count_expected as i8),
            );
        }
        if arg_count > arg_count_expected {
            self.errors.lock().unwrap().report_error(
                expr.get_span(),
                ParserErrorType::TooManyArguments(expr.to_string(), arg_count, arg_count_expected as i8),
            );
        }
    }

    /// Registers the signature of a function the file implements, so a call that comes
    /// before it resolves. A name that is already declared keeps its declaration.
    fn predeclare_function(&mut self, function: &crate::ast::FunctionImplementation) {
        if self.has_variable_defined(function.get_identifier()) {
            return;
        }
        let id = self.add_declaration(VariableType::Function, function.get_identifier_token());
        self.global_lookup.variable_lookup.insert(function.get_identifier().clone(), id);
        self.function_containers.push(FunctionContainer {
            name: function.get_identifier().clone(),
            parameter_index: None,
            id,
            functions: FunctionDeclaration::Function(FunctionDeclarationAstNode::empty(
                function.get_identifier().clone(),
                function.get_parameters().clone(),
                function.get_return_type(),
            )),
            lookup: VariableLookups::default(),
            parameters: 0..0,
            local_variables: 0..0,
        });
    }

    fn predeclare_procedure(&mut self, procedure: &crate::ast::ProcedureImplementation) {
        if self.has_variable_defined(procedure.get_identifier()) {
            return;
        }
        let id = self.add_declaration(VariableType::Procedure, procedure.get_identifier_token());
        self.global_lookup.variable_lookup.insert(procedure.get_identifier().clone(), id);
        self.function_containers.push(FunctionContainer {
            name: procedure.get_identifier().clone(),
            parameter_index: None,
            id,
            functions: FunctionDeclaration::Procedure(ProcedureDeclarationAstNode::empty(
                procedure.get_identifier().clone(),
                procedure.get_parameters().clone(),
            )),
            lookup: VariableLookups::default(),
            parameters: 0..0,
            local_variables: 0..0,
        });
    }

    pub fn finish(&mut self) {
        for (rt, r) in &mut self.references.iter() {
            if matches!(rt, ReferenceType::Label(_)) {
                if r.declaration.is_none() {
                    if let Some((file, span)) = r.usages.first() {
                        self.errors.lock().unwrap().report_error_file(
                            file.clone(),
                            span.span.clone(),
                            CompilationErrorType::LabelNotFound(span.token.to_string()),
                        );
                    }
                } else if r.usages.is_empty() {
                    if let Some((file_name, declaration)) = &r.declaration {
                        if ":~BEGIN~" == declaration.token || declaration.token.starts_with(":*(") {
                            continue;
                        }
                        self.errors.lock().unwrap().report_warning_file(
                            file_name.clone(),
                            declaration.span.clone(),
                            CompilationWarningType::UnusedLabel(declaration.token.to_string()),
                        );
                    }
                }
                continue;
            }

            let Some((file, decl)) = &r.declaration else {
                continue;
            };

            if r.variable_type == VariableType::Function || r.variable_type == VariableType::Procedure {
                if r.implementation.is_none() {
                    self.errors.lock().unwrap().report_error_file(
                        file.clone(),
                        decl.span.clone(),
                        CompilationErrorType::MissingImplementation(decl.token.to_string()),
                    );
                }
                if r.usages.is_empty() {
                    self.errors.lock().unwrap().report_warning_file(
                        file.clone(),
                        decl.span.clone(),
                        CompilationErrorType::UnusedFunction(decl.token.to_string()),
                    );
                }
            } else if matches!(rt, ReferenceType::Variable(_)) && r.usages.is_empty() {
                self.errors
                    .lock()
                    .unwrap()
                    .report_warning_file(file.clone(), decl.span.clone(), CompilationErrorType::UnusedVariable(decl.token.to_string()));
            }
        }

        // search if any user variables are used.
        if !self.require_user_variables {
            for (_i, user_var) in USER_VARIABLES.iter().enumerate() {
                if user_var.runtime_version > self.lang_version {
                    continue;
                }
                for (_rype, r) in &self.references {
                    if !r.usages.is_empty() && r.usages[0].1.token == user_var.name {
                        self.require_user_variables = true;
                        break;
                    }
                }
            }
        }
    }

    fn check_arg_types(&mut self, call_parameters: &[ParameterSpecifier], arguments: &[Expression]) {
        for i in 0..call_parameters.len() {
            match &call_parameters[i] {
                ParameterSpecifier::Function(f) => {
                    let previous = self.allow_routine_reference;
                    self.allow_routine_reference = true;
                    let vt: VariableType = arguments[i].visit(self);
                    self.allow_routine_reference = previous;
                    if vt != VariableType::Function {
                        self.errors
                            .lock()
                            .unwrap()
                            .report_error(arguments[i].get_span().clone(), CompilationErrorType::FunctionExpected);
                    }

                    if vt == VariableType::Function {
                        let container = self.function_containers.iter().find(|container| container.id == self.last_lookup_index);
                        let matches = container.is_some_and(|container| match &container.functions {
                            FunctionDeclaration::Function(declaration) => {
                                f.get_return_type() == declaration.get_return_type() && parameter_lists_match(f.get_parameters(), declaration.get_parameters())
                            }
                            _ => false,
                        });
                        if !matches {
                            self.errors.lock().unwrap().report_error(
                                arguments[i].get_span().clone(),
                                CompilationErrorType::ParameterMismatch(arguments[i].to_string()),
                            );
                        }
                    }
                }
                ParameterSpecifier::Procedure(p) => {
                    let previous = self.allow_routine_reference;
                    self.allow_routine_reference = true;
                    let vt = arguments[i].visit(self);
                    self.allow_routine_reference = previous;
                    if vt != VariableType::Procedure {
                        self.errors
                            .lock()
                            .unwrap()
                            .report_error(arguments[i].get_span().clone(), CompilationErrorType::ProcedureExpected);
                    }
                    if vt == VariableType::Procedure {
                        let container = self.function_containers.iter().find(|container| container.id == self.last_lookup_index);
                        let matches = container.is_some_and(|container| match &container.functions {
                            FunctionDeclaration::Procedure(declaration) => parameter_lists_match(p.get_parameters(), declaration.get_parameters()),
                            _ => false,
                        });
                        if !matches {
                            self.errors.lock().unwrap().report_error(
                                arguments[i].get_span().clone(),
                                CompilationErrorType::ParameterMismatch(arguments[i].to_string()),
                            );
                        }
                    }
                }
                ParameterSpecifier::Variable(parameter) => {
                    let expected = parameter.get_variable_type();
                    let actual = arguments[i].visit(self);
                    if expected != actual && (matches!(expected, VariableType::UserData(_)) || matches!(actual, VariableType::UserData(_))) {
                        self.errors.lock().unwrap().report_error(
                            arguments[i].get_span(),
                            CompilationErrorType::ArgumentTypeMismatch(i + 1, self.source_type_name(expected), self.source_type_name(actual)),
                        );
                    }
                }
            }
        }
    }
}

impl AstVisitor<VariableType> for SemanticVisitor {
    fn visit_record_literal_expression(&mut self, record: &crate::ast::RecordLiteralExpression) -> VariableType {
        if self.runtime < FIRST_RECORD_LITERAL_RUNTIME {
            self.errors.lock().unwrap().report_error(
                record.get_type_token().span.clone(),
                CompilationErrorType::RecordLiteralNeedsRuntime(FIRST_RECORD_LITERAL_RUNTIME),
            );
        }
        let VariableType::UserData(type_id) = record.get_variable_type() else {
            return VariableType::None;
        };
        let Some(definition) = self.type_registry.get_user_type_from_id(type_id) else {
            return VariableType::None;
        };
        let mut seen = HashSet::new();
        for field in record.get_fields() {
            let name = field.get_identifier();
            if !seen.insert(name.clone()) {
                self.errors.lock().unwrap().report_error(
                    field.get_identifier_token().span.clone(),
                    CompilationErrorType::DuplicateRecordLiteralField(name.to_string()),
                );
                continue;
            }
            let Some(index) = definition.field_index(name) else {
                self.errors.lock().unwrap().report_error(
                    field.get_identifier_token().span.clone(),
                    CompilationErrorType::UnknownRecordLiteralField(record.get_variable_type(), name.to_string()),
                );
                field.get_value().visit(self);
                continue;
            };
            let expected = definition.field_type(index).unwrap_or(VariableType::None);
            let actual = field.get_value().visit(self);
            if expected != actual && (matches!(expected, VariableType::UserData(_)) || matches!(actual, VariableType::UserData(_))) {
                self.errors.lock().unwrap().report_error(
                    field.get_value().get_span(),
                    CompilationErrorType::RecordLiteralFieldTypeMismatch(name.to_string(), self.source_type_name(expected), self.source_type_name(actual)),
                );
            }
        }
        record.get_variable_type()
    }

    fn visit_binary_expression(&mut self, binary: &crate::ast::BinaryExpression) -> VariableType {
        let left = binary.get_left_expression().visit(self);
        let right = binary.get_right_expression().visit(self);
        let has_enum = self.type_registry.is_enum_type(left) || self.type_registry.is_enum_type(right);
        if has_enum && self.counts_a_loop(binary.get_left_expression()) {
            // A FOR writes its own comparison and step, so it may count over an enum.
            return match binary.get_op() {
                crate::ast::BinOp::Lower | crate::ast::BinOp::LowerEq | crate::ast::BinOp::Greater | crate::ast::BinOp::GreaterEq => {
                    if left != right {
                        self.errors.lock().unwrap().report_error(
                            binary.get_right_expression().get_span(),
                            CompilationErrorType::EnumComparisonTypeMismatch(self.source_type_name(left), self.source_type_name(right)),
                        );
                    }
                    VariableType::Boolean
                }
                _ => left,
            };
        }
        if has_enum && !matches!(binary.get_op(), crate::ast::BinOp::Eq | crate::ast::BinOp::NotEq) {
            self.errors.lock().unwrap().report_error(
                binary.get_op_token().span.clone(),
                CompilationErrorType::CustomTypeOperatorNotSupported(binary.get_op()),
            );
            return VariableType::None;
        }
        if has_enum && left != right {
            self.errors.lock().unwrap().report_error(
                binary.get_op_token().span.clone(),
                CompilationErrorType::EnumComparisonTypeMismatch(self.source_type_name(left), self.source_type_name(right)),
            );
            return VariableType::Boolean;
        }
        let has_custom_type = matches!(left, VariableType::UserData(_)) || matches!(right, VariableType::UserData(_));
        if has_custom_type && !matches!(binary.get_op(), crate::ast::BinOp::Eq | crate::ast::BinOp::NotEq) {
            self.errors.lock().unwrap().report_error(
                binary.get_op_token().span.clone(),
                CompilationErrorType::CustomTypeOperatorNotSupported(binary.get_op()),
            );
        }
        if has_custom_type && matches!(binary.get_op(), crate::ast::BinOp::Eq | crate::ast::BinOp::NotEq) {
            if self.is_whole_custom_type_array(binary.get_left_expression()) || self.is_whole_custom_type_array(binary.get_right_expression()) {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(binary.get_op_token().span.clone(), CompilationErrorType::CustomTypeArrayComparisonNotSupported);
            } else if left != right {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(binary.get_op_token().span.clone(), CompilationErrorType::ComparisonTypeMismatch(left, right));
            }
            VariableType::Boolean
        } else {
            VariableType::None
        }
    }

    fn visit_identifier_expression(&mut self, identifier: &IdentifierExpression) -> VariableType {
        if let Some((variable_type, _)) = self.lookup_constant(identifier.get_identifier()) {
            return *variable_type;
        }
        let predef = FunctionDefinition::get_function_definitions(identifier.get_identifier());
        if !predef.is_empty() {
            let def = &FUNCTION_DEFINITIONS[predef[0]];
            if self.cur_func_call > 0 {
                self.function_type_lookup.insert(self.cur_func_call, SemanticInfo::PredefFunctionGroup(predef));
            } else {
                self.errors.lock().unwrap().report_error(
                    identifier.get_identifier_token().span.clone(),
                    CompilationErrorType::FunctionUsedAsVariable(identifier.get_identifier().to_string()),
                );
            }
            return def.return_type;
        } else if let Some(idx) = self.lookup_variable(identifier.get_identifier()) {
            let (rt, r) = &mut self.references[idx];
            let identifier = identifier.get_identifier_token();
            if self.cur_func_call > 0 {
                if let ReferenceType::Function(func_idx) = rt {
                    self.function_type_lookup.insert(self.cur_func_call, SemanticInfo::FunctionReference(*func_idx));
                } else if let ReferenceType::Variable(func_idx) = rt {
                    self.function_type_lookup.insert(self.cur_func_call, SemanticInfo::VariableReference(*func_idx));
                }
            } else {
                match rt {
                    ReferenceType::Function(_) | ReferenceType::Procedure(_)
                        if !self.allow_routine_reference && !self.allowed_routine_reference_spans.contains(&identifier.span.start) =>
                    {
                        self.errors.lock().unwrap().report_error(
                            identifier.span.clone(),
                            CompilationErrorType::FunctionUsedAsVariable(identifier.token.to_string()),
                        );
                        return VariableType::None;
                    }
                    ReferenceType::Function(_) | ReferenceType::Procedure(_) if self.allow_routine_reference => {
                        if self.runtime < FIRST_ROUTINE_REFERENCE_RUNTIME {
                            self.errors.lock().unwrap().report_error(
                                identifier.span.clone(),
                                CompilationErrorType::RoutineReferenceNeedsRuntime(FIRST_ROUTINE_REFERENCE_RUNTIME),
                            );
                        }
                        self.allowed_routine_reference_spans.insert(identifier.span.start);
                    }
                    _ => {}
                }
            }
            r.usages.push((
                self.errors.lock().unwrap().file_name().to_path_buf(),
                Spanned::new(identifier.token.to_string(), identifier.span.clone()),
            ));
            r.variable_type
        } else {
            if self.lang_version < 350 || self.cur_func_call == 0 {
                self.errors.lock().unwrap().report_error(
                    identifier.get_identifier_token().span.clone(),
                    CompilationErrorType::VariableNotFound(identifier.get_identifier().to_string()),
                );
            }
            VariableType::None
        }
    }

    fn visit_member_reference_expression(&mut self, member_reference_expression: &crate::ast::MemberReferenceExpression) -> VariableType {
        if let Expression::Identifier(base) = member_reference_expression.get_expression() {
            if let Some(definition) = self.type_registry.get_enum(base.get_identifier()) {
                if let Some(value) = definition.value(member_reference_expression.get_identifier()) {
                    self.add_constant(&Constant::Integer(value, crate::ast::constant::NumberFormat::Default));
                    return VariableType::UserData(definition.id);
                }
                self.errors.lock().unwrap().report_error(
                    member_reference_expression.get_identifier_token().span.clone(),
                    CompilationErrorType::EnumMemberNotFound(definition.name.to_string(), member_reference_expression.get_identifier().to_string()),
                );
                return VariableType::None;
            }
        }
        let t = member_reference_expression.get_expression().visit(self);
        if let VariableType::UserData(d) = t {
            if crate::parser::is_user_declared_type(d) {
                return self.resolve_record_field(
                    d,
                    member_reference_expression.get_identifier(),
                    &member_reference_expression.get_identifier_token().span,
                );
            }
            if let Some(t) = self.type_registry.get_type_from_id(d) {
                for (name, t) in &t.fields {
                    if name == member_reference_expression.get_identifier() {
                        self.user_type_lookup.insert(member_reference_expression.get_identifier_token().span.start, d);
                        return *t;
                    }
                }
                for (name, (_args, t)) in &t.functions {
                    if name == member_reference_expression.get_identifier() {
                        self.user_type_lookup.insert(member_reference_expression.get_identifier_token().span.start, d);
                        return *t;
                    }
                }
                for (name, _args) in &t.procedures {
                    if name == member_reference_expression.get_identifier() {
                        self.user_type_lookup.insert(member_reference_expression.get_identifier_token().span.start, d);
                        return VariableType::None;
                    }
                }
                self.errors.lock().unwrap().report_error(
                    member_reference_expression.get_identifier_token().span.clone(),
                    CompilationErrorType::InvalidMemberReferenceExpression,
                );
            } else {
                self.errors.lock().unwrap().report_error(
                    member_reference_expression.get_expression().get_span().clone(),
                    CompilationErrorType::TypeNotFound,
                );
            }
        } else {
            self.errors.lock().unwrap().report_error(
                member_reference_expression.get_identifier_token().span.clone(),
                CompilationErrorType::InvalidMemberReferenceExpression,
            );
        }
        VariableType::None
    }

    fn visit_constant_expression(&mut self, constant: &ConstantExpression) -> VariableType {
        self.add_constant(constant.get_constant_value());
        match constant.get_constant_value() {
            Constant::Integer(_, _) => VariableType::Integer,
            Constant::String(_) => VariableType::String,
            Constant::Boolean(_) => VariableType::Boolean,
            Constant::Money(_) => VariableType::Money,
            Constant::Unsigned(_) => VariableType::Unsigned,
            Constant::Double(_) => VariableType::Double,
            Constant::Builtin(_) => VariableType::Integer,
        }
    }

    fn visit_comment(&mut self, _comment: &CommentAstNode) -> VariableType {
        // nothing yet
        VariableType::None
    }

    fn visit_enum_declaration(&mut self, _enum_decl: &EnumDeclarationAstNode) -> VariableType {
        VariableType::None
    }

    fn visit_predefined_call_statement(&mut self, call_stmt: &PredefinedCallStatement) -> VariableType {
        let def = call_stmt.get_func();
        walk_predefined_call_statement(self, call_stmt);

        match def.sig {
            crate::executable::StatementSignature::Invalid => panic!("Invalid signature"),
            crate::executable::StatementSignature::ArgumentsWithVariable(v, arg_count) => {
                self.check_arg_count(arg_count, call_stmt.get_arguments().len(), call_stmt.get_identifier_token());
                if v > 0 {
                    if let Some(arg) = call_stmt.get_arguments().get(v - 1) {
                        self.check_argument_is_variable(v - 1, &arg);
                    }
                }
            }
            crate::executable::StatementSignature::VariableArguments(_, min, max) => {
                if call_stmt.get_arguments().len() < min {
                    self.errors.lock().unwrap().report_error(
                        call_stmt.get_identifier_token().span.clone(),
                        CompilationErrorType::TooFewArguments(call_stmt.get_identifier().to_string(), min),
                    );
                }
                if max > 0 && call_stmt.get_arguments().len() > max {
                    self.errors.lock().unwrap().report_error(
                        call_stmt.get_identifier_token().span.clone(),
                        CompilationErrorType::TooManyArguments(call_stmt.get_identifier().to_string(), max),
                    );
                }
            }
            crate::executable::StatementSignature::SpecialCaseDlockg => {
                self.check_arg_count(3, call_stmt.get_arguments().len(), call_stmt.get_identifier_token());
                if call_stmt.get_arguments().len() >= 3 {
                    self.check_argument_is_variable(2, &call_stmt.get_arguments()[2]);
                }
            }
            crate::executable::StatementSignature::SpecialCaseDcreate => {
                self.check_arg_count(4, call_stmt.get_arguments().len(), call_stmt.get_identifier_token());
                if call_stmt.get_arguments().len() >= 4 {
                    self.check_argument_is_variable(3, &call_stmt.get_arguments()[3]);
                }
            }
            crate::executable::StatementSignature::SpecialCaseSort => {
                self.check_arg_count(2, call_stmt.get_arguments().len(), call_stmt.get_identifier_token());

                for i in 0..=1 {
                    if call_stmt.get_arguments().len() <= i {
                        break;
                    }
                    if let Expression::Identifier(a) = &call_stmt.get_arguments()[i] {
                        if let Some(idx) = self.lookup_variable(a.get_identifier()) {
                            let (_rt, r) = &mut self.references[idx];
                            if let Some(header) = &r.header {
                                if header.dim != 1 {
                                    self.errors.lock().unwrap().report_error(
                                        a.get_identifier_token().span.clone(),
                                        CompilationErrorType::SortArgumentDimensionError(header.dim),
                                    );
                                }
                            }
                        } else {
                            self.errors
                                .lock()
                                .unwrap()
                                .report_error(call_stmt.get_arguments()[i].get_span().clone(), CompilationErrorType::VariableExpected(i + 1));
                        }
                    } else {
                        self.errors
                            .lock()
                            .unwrap()
                            .report_error(call_stmt.get_arguments()[i].get_span().clone(), CompilationErrorType::VariableExpected(i + 1));
                    }
                }
            }
            crate::executable::StatementSignature::SpecialCaseVarSeg => {
                self.check_arg_count(2, call_stmt.get_arguments().len(), call_stmt.get_identifier_token());

                for (v, arg) in call_stmt.get_arguments().iter().enumerate() {
                    self.check_argument_is_variable(v, arg);
                }
            }
            crate::executable::StatementSignature::SpecialCasePop => {
                for (v, arg) in call_stmt.get_arguments().iter().enumerate() {
                    self.check_argument_is_variable(v, arg);
                }
            }
        }

        self.add_reference(
            ReferenceType::PredefinedProc(call_stmt.get_func().opcode),
            VariableType::Procedure,
            call_stmt.get_identifier_token(),
        );
        VariableType::None
    }

    fn visit_function_call_expression(&mut self, call: &FunctionCallExpression) -> VariableType {
        let mut res = VariableType::None;
        let is_ident = matches!(call.get_expression(), Expression::Identifier(_));
        self.cur_func_call = call.id;
        call.get_expression().visit(self);
        self.cur_func_call = 0;

        match self.function_type_lookup.get(&call.id).cloned() {
            Some(SemanticInfo::FunctionReference(idx)) => {
                let declaration = self.function_containers[idx].functions.clone();
                let arg_count = match &declaration {
                    FunctionDeclaration::Function(f) => {
                        res = f.get_return_type();
                        self.check_arg_types(f.get_parameters(), call.get_arguments());
                        f.get_parameters().len()
                    }
                    _ => {
                        self.errors.lock().unwrap().report_error(
                            call.get_expression().get_span(),
                            CompilationErrorType::FunctionNotFound(call.get_expression().to_string()),
                        );
                        0
                    }
                };
                self.check_expr_arg_count(arg_count, call.get_arguments().len(), call.get_expression());
            }
            Some(SemanticInfo::VariableReference(idx)) => {
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                if let Expression::MemberReference(member) = call.get_expression() {
                    if let Some(user_type) = self.user_type_lookup.get(&member.get_identifier_token().span.start) {
                        if let Some(registry) = self.type_registry.get_type_from_id(*user_type) {
                            for (name, (pars, t)) in &registry.functions {
                                if name == member.get_identifier() {
                                    self.check_expr_arg_count(pars.len(), call.get_arguments().len(), call.get_expression());
                                    if let Some(member) = registry.get_member_id(name) {
                                        self.function_type_lookup.insert(call.id, SemanticInfo::MemberFunctionCall(member));
                                        return *t;
                                    } else {
                                        self.errors.lock().unwrap().report_error(
                                            member.get_identifier_token().span.clone(),
                                            CompilationErrorType::FunctionNotFound(member.get_identifier().to_string()),
                                        );
                                        return res;
                                    }
                                }
                            }
                            self.errors.lock().unwrap().report_error(
                                member.get_identifier_token().span.clone(),
                                CompilationErrorType::FunctionNotFound(member.get_identifier().to_string()),
                            );
                            return res;
                        }
                    } else {
                        // error already reported.
                        return res;
                    }
                }

                let (rt, r) = &mut self.references[idx];

                let arg_count = if let ReferenceType::Variable(_func) = rt {
                    r.header.as_ref().unwrap().dim as usize
                } else {
                    0
                };
                res = r.variable_type;
                self.check_expr_arg_count(arg_count, call.get_arguments().len(), call.get_expression());
            }
            Some(SemanticInfo::PredefFunctionGroup(funcs)) => {
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                for func in &funcs {
                    let def = &FUNCTION_DEFINITIONS[*func];
                    if def.parameter_count() == call.get_arguments().len() {
                        if self.lang_version < def.version {
                            self.errors.lock().unwrap().report_error(
                                call.get_expression().get_span(),
                                ParserErrorType::FunctionVersionNotSupported(def.opcode, def.version, self.lang_version),
                            );
                            return res;
                        }
                        self.function_type_lookup.insert(call.id, SemanticInfo::PredefinedFunc(def.opcode));
                        if let Expression::Identifier(id) = call.get_expression() {
                            self.add_reference(ReferenceType::PredefinedFunc(def.opcode), VariableType::Function, id.get_identifier_token());
                        }
                        return def.return_type;
                    }
                }
                // report wrong argument count
                self.check_expr_arg_count(
                    FUNCTION_DEFINITIONS[funcs[0]].parameter_count(),
                    call.get_arguments().len(),
                    call.get_expression(),
                );
            }

            _ => {
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                if self.lang_version < 350 || !is_ident {
                    self.errors.lock().unwrap().report_error(
                        call.get_expression().get_span(),
                        CompilationErrorType::FunctionNotFound(call.get_expression().to_string()),
                    );
                } else if let Expression::Identifier(ident) = call.get_expression() {
                    let id = self.add_declaration(VariableType::Function, ident.get_identifier_token());
                    self.global_lookup.variable_lookup.insert(ident.get_identifier().clone(), id);
                    self.function_containers.push(FunctionContainer {
                        name: ident.get_identifier().clone(),
                        parameter_index: None,
                        id,
                        functions: FunctionDeclaration::Function(FunctionDeclarationAstNode::empty(
                            ident.get_identifier().clone(),
                            call.get_arguments()
                                .iter()
                                .map(|_a| ParameterSpecifier::Variable(VariableParameterSpecifier::empty(false, VariableType::None, None)))
                                .collect(),
                            VariableType::None,
                        )),
                        lookup: VariableLookups::default(),
                        parameters: 0..0,
                        local_variables: 0..0,
                    });
                } else {
                    panic!("Invalid function call expression");
                }
            }
        }
        res
    }

    fn visit_indexer_expression(&mut self, indexer: &crate::ast::IndexerExpression) -> VariableType {
        let mut found = false;
        let mut res = VariableType::None;
        let arg_count = if let Some(idx) = self.lookup_variable(indexer.get_identifier()) {
            let (rt, r) = &mut self.references[idx];
            if matches!(rt, ReferenceType::Function(_)) {
                self.errors.lock().unwrap().report_error(
                    indexer.get_identifier_token().span.clone(),
                    CompilationErrorType::IndexerCalledOnFunction(indexer.get_identifier().to_string()),
                );
                return VariableType::None;
            }
            found = true;
            res = r.variable_type;
            r.usages.push((
                self.errors.lock().unwrap().file_name().to_path_buf(),
                Spanned::new(indexer.get_identifier().to_string(), indexer.get_identifier_token().span.clone()),
            ));
            r.header.as_ref().unwrap().dim as usize
        } else {
            0
        };

        if found {
            self.check_arg_count(arg_count, indexer.get_arguments().len(), indexer.get_identifier_token());
        } else {
            self.errors.lock().unwrap().report_error(
                indexer.get_identifier_token().span.clone(),
                CompilationErrorType::FunctionNotFound(indexer.get_identifier().to_string()),
            );
        }
        walk_indexer_expression(self, indexer);
        res
    }

    fn visit_goto_statement(&mut self, goto: &GotoStatement) -> VariableType {
        self.add_label_usage(goto.get_label_token());
        VariableType::None
    }

    fn visit_gosub_statement(&mut self, gosub: &GosubStatement) -> VariableType {
        self.add_label_usage(gosub.get_label_token());
        VariableType::None
    }

    fn visit_label_statement(&mut self, label: &LabelStatement) -> VariableType {
        self.set_label_declaration(label.get_label_token());
        VariableType::None
    }

    fn visit_let_statement(&mut self, let_stmt: &LetStatement) -> VariableType {
        let mut target_type = VariableType::None;
        if self.lookup_constant(let_stmt.get_identifier()).is_some() {
            self.errors.lock().unwrap().report_error(
                let_stmt.get_identifier_token().span.clone(),
                CompilationErrorType::CannotAssignToConstant(let_stmt.get_identifier().to_string()),
            );
            return VariableType::None;
        }
        if let Some(idx) = self.lookup_variable(let_stmt.get_identifier()) {
            if self.references[idx].1.variable_type == VariableType::Procedure {
                self.errors
                    .lock()
                    .unwrap()
                    .report_warning(let_stmt.get_identifier_token().span.clone(), CompilationWarningType::CannotAssignToProcedure);
            } else if self.references[idx].1.variable_type == VariableType::Function {
                self.references[idx].1.return_types.push((
                    self.errors.lock().unwrap().file_name().to_path_buf(),
                    Spanned::new(let_stmt.get_identifier().to_string(), let_stmt.get_identifier_token().span.clone()),
                ));
                if let Some(container) = self.function_containers.iter().find(|container| container.id == idx) {
                    if let FunctionDeclaration::Function(function) = &container.functions {
                        target_type = function.get_return_type();
                    }
                }
            } else {
                target_type = self.references[idx].1.variable_type;
                if let Some(header) = &self.references[idx].1.header {
                    self.check_arg_count(header.dim as usize, let_stmt.get_arguments().len(), let_stmt.get_identifier_token());
                } else {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(let_stmt.get_identifier_token().span.clone(), CompilationErrorType::InvalidLetVariable);
                }

                self.add_reference_to(let_stmt.get_identifier_token(), idx);

                let mut variable_type = target_type;
                for member_token in let_stmt.get_members() {
                    match variable_type {
                        VariableType::UserData(type_id) if crate::parser::is_user_declared_type(type_id) => {
                            let Token::Identifier(member) = &member_token.token else {
                                break;
                            };
                            variable_type = self.resolve_record_field(type_id, member, &member_token.span);
                        }
                        // Board objects hand out copies, so writing to one would go nowhere.
                        _ => {
                            self.errors
                                .lock()
                                .unwrap()
                                .report_error(member_token.span.clone(), CompilationErrorType::InvalidLetVariable);
                            break;
                        }
                    }
                }
                target_type = variable_type;
            }
        } else {
            self.errors.lock().unwrap().report_error(
                let_stmt.get_identifier_token().span.clone(),
                CompilationErrorType::VariableNotFound(let_stmt.get_identifier().to_string()),
            );
        }
        for arg in let_stmt.get_arguments() {
            arg.visit(self);
        }
        let value_type = let_stmt.get_value_expression().visit(self);
        if (self.type_registry.is_enum_type(target_type) || self.type_registry.is_enum_type(value_type)) && target_type != value_type {
            self.errors.lock().unwrap().report_error(
                let_stmt.get_eq_token().span.clone(),
                CompilationErrorType::EnumAssignmentTypeMismatch(self.source_type_name(target_type), self.source_type_name(value_type)),
            );
            return VariableType::None;
        }
        if target_type != value_type && (matches!(target_type, VariableType::UserData(_)) || matches!(value_type, VariableType::UserData(_))) {
            self.errors.lock().unwrap().report_error(
                let_stmt.get_eq_token().span.clone(),
                CompilationErrorType::AssignmentTypeMismatch(target_type, value_type),
            );
        }
        VariableType::None
    }

    fn visit_for_statement(&mut self, for_stmt: &crate::ast::ForStatement) -> VariableType {
        if let Some(idx) = self.lookup_variable(for_stmt.get_identifier()) {
            let (_rt, r) = &mut self.references[idx];
            let identifier = for_stmt.get_identifier_token();
            r.usages.push((
                self.errors.lock().unwrap().file_name().to_path_buf(),
                Spanned::new(identifier.token.to_string(), identifier.span.clone()),
            ));
        } else {
            self.errors.lock().unwrap().report_error(
                for_stmt.get_identifier_token().span.clone(),
                CompilationErrorType::VariableNotFound(for_stmt.get_identifier().to_string()),
            );
        };
        crate::ast::walk_for_stmt(self, for_stmt);
        VariableType::None
    }

    fn visit_const_declaration_statement(&mut self, const_decl: &ConstDeclarationStatement) -> VariableType {
        // The value is never read at runtime, so walking it would put literals nobody
        // uses into the table.
        if self.has_variable_defined(const_decl.get_identifier()) {
            self.errors.lock().unwrap().report_error(
                const_decl.get_identifier_token().span.clone(),
                CompilationErrorType::VariableAlreadyDefined(const_decl.get_identifier().to_string()),
            );
            return VariableType::None;
        }

        let value = const_value_with_members(
            const_decl.get_value(),
            &|id| self.lookup_constant(id).map(|(_, value)| value.clone()),
            &|type_name, member| {
                self.type_registry
                    .get_enum(type_name)
                    .and_then(|definition| definition.value(member))
                    .map(VariableValue::new_int)
            },
        );
        let Some(value) = value else {
            self.errors
                .lock()
                .unwrap()
                .report_error(const_decl.get_value().get_span(), CompilationErrorType::ConstantValueExpected);
            return VariableType::None;
        };

        let declared_type = const_decl.get_variable_type();
        if self.type_registry.is_enum_type(declared_type) {
            let actual = self.declared_constant_type(const_decl.get_value()).unwrap_or_else(|| value.get_type());
            if actual != declared_type {
                self.errors.lock().unwrap().report_error(
                    const_decl.get_value().get_span(),
                    CompilationErrorType::EnumAssignmentTypeMismatch(self.source_type_name(declared_type), self.source_type_name(actual)),
                );
                return VariableType::None;
            }
        }

        let name = const_decl.get_identifier().clone();
        // An enum keeps the value its member stands for; converting to the type itself would mean nothing.
        let entry = if self.type_registry.is_enum_type(declared_type) {
            (declared_type, value)
        } else {
            (declared_type, value.convert_to(declared_type))
        };
        if let Some(local) = &mut self.local_constants {
            local.insert(name, entry);
        } else {
            self.global_constants.insert(name, entry);
        }
        VariableType::None
    }

    fn visit_variable_declaration_statement(&mut self, var_decl: &VariableDeclarationStatement) -> VariableType {
        for v in var_decl.get_variables() {
            if self.has_variable_defined(v.get_identifier()) {
                self.errors.lock().unwrap().report_error(
                    v.get_identifier_token().span.clone(),
                    CompilationErrorType::VariableAlreadyDefined(v.get_identifier().to_string()),
                );
                continue;
            }
            let (dims, vs) = if let Some(Expression::ArrayInitializer(arr_expr)) = v.get_initalizer() {
                for expr in arr_expr.get_expressions() {
                    expr.visit(self);
                }
                (1, arr_expr.get_expressions().len())
            } else {
                (v.get_dimensions().len() as u8, v.get_vector_size())
            };
            self.add_variable(
                var_decl.get_variable_type(),
                v.get_identifier_token(),
                dims,
                vs,
                v.get_matrix_size(),
                v.get_cube_size(),
            );
        }
        VariableType::None
    }

    fn visit_procedure_call_statement(&mut self, call: &ProcedureCallStatement) -> VariableType {
        let mut found = false;
        if let Some(idx) = self.lookup_variable(call.get_identifier()) {
            if matches!(self.references[idx].0, ReferenceType::Variable(_)) {
                self.add_reference_to(call.get_identifier_token(), idx);
                found = true;
            }

            if matches!(self.references[idx].0, ReferenceType::Function(_)) {
                let f = self.function_containers.iter().find(|p| p.name == call.get_identifier()).unwrap();
                if let FunctionDeclaration::Function(f) = &f.functions.clone() {
                    let param_count = f.get_parameters().len();
                    let arg_count = call.get_arguments().len();
                    let identifier_token = call.get_identifier_token();
                    self.check_arg_count(param_count, arg_count, identifier_token);
                    self.check_arg_types(f.get_parameters(), call.get_arguments());
                }

                self.add_reference_to(call.get_identifier_token(), idx);
                found = true;
            }

            if matches!(self.references[idx].0, ReferenceType::Procedure(_)) {
                let func_container = self.function_containers.iter().find(|p| p.name == call.get_identifier()).unwrap();

                if let FunctionDeclaration::Procedure(f) = &func_container.functions.clone() {
                    let arg_count = call.get_arguments().len();
                    let par_len = f.get_parameters().len();

                    self.check_arg_count(par_len, arg_count, call.get_identifier_token());
                    let arg_count = arg_count.min(par_len);
                    let pass_flags = f.get_pass_flags();
                    self.check_arg_types(f.get_parameters(), call.get_arguments());

                    for i in 0..arg_count {
                        if pass_flags & (1 << i) != 0 {
                            self.check_argument_is_variable(i, &call.get_arguments()[i]);
                        }
                    }
                }

                self.add_reference_to(call.get_identifier_token(), idx);
                found = true;
            }
        }

        if !found {
            if self.lang_version < 350 {
                self.errors.lock().unwrap().report_error(
                    call.get_identifier_token().span.clone(),
                    CompilationErrorType::ProcedureNotFound(call.get_identifier().to_string()),
                );
            } else {
                let id = self.add_declaration(VariableType::Procedure, call.get_identifier_token());
                self.global_lookup.variable_lookup.insert(call.get_identifier().clone(), id);
                self.function_containers.push(FunctionContainer {
                    name: call.get_identifier().clone(),
                    parameter_index: None,
                    id,
                    functions: FunctionDeclaration::Procedure(ProcedureDeclarationAstNode::empty(
                        call.get_identifier().clone(),
                        call.get_arguments()
                            .iter()
                            .map(|_a| ParameterSpecifier::Variable(VariableParameterSpecifier::empty(false, VariableType::None, None)))
                            .collect(),
                    )),
                    lookup: VariableLookups::default(),
                    parameters: 0..0,
                    local_variables: 0..0,
                });
                return self.visit_procedure_call_statement(call);
            }
        }

        walk_procedure_call_statement(self, call);
        VariableType::None
    }

    fn visit_function_declaration(&mut self, func_decl: &FunctionDeclarationAstNode) -> VariableType {
        if self.has_variable_defined(func_decl.get_identifier()) {
            self.errors.lock().unwrap().report_error(
                func_decl.get_identifier_token().span.clone(),
                CompilationErrorType::VariableAlreadyDefined(func_decl.get_identifier().to_string()),
            );
            return VariableType::None;
        }
        let id = self.add_declaration(VariableType::Function, func_decl.get_identifier_token());
        self.global_lookup.variable_lookup.insert(func_decl.get_identifier().clone(), id);
        self.function_containers.push(FunctionContainer {
            name: func_decl.get_identifier().clone(),
            parameter_index: None,
            id,
            functions: FunctionDeclaration::Function(func_decl.clone()),
            lookup: VariableLookups::default(),
            parameters: 0..0,
            local_variables: 0..0,
        });
        VariableType::None
    }

    fn visit_function_implementation(&mut self, function: &FunctionImplementation) -> VariableType {
        if let Some(idx) = self.lookup_variable(function.get_identifier()) {
            // Procedure call may've added a function wrongly as a procedure, fix that here.
            {
                let (ref_kind, refs) = &mut self.references[idx];
                match ref_kind.clone() {
                    ReferenceType::Procedure(container_idx) => {
                        // Switch the reference kind.
                        *ref_kind = ReferenceType::Function(container_idx);
                        // Update semantic type.
                        refs.variable_type = VariableType::Function;
                        if let Some(h) = refs.header.as_mut() {
                            h.variable_type = VariableType::Function;
                        }
                    }
                    ReferenceType::Function(_) => {
                        // All good.
                    }
                    _ => {
                        self.errors.lock().unwrap().report_error(
                            function.get_identifier_token().span.clone(),
                            CompilationErrorType::InternalError(format!(
                                "Internal error: Found function implementation for non-procedure: {}",
                                function.get_identifier()
                            )),
                        );
                    }
                }
            }

            let identifier = function.get_identifier_token();
            self.cur_func_impl = idx;
            self.references[idx].1.implementation = Some((
                self.errors.lock().unwrap().file_name().to_path_buf(),
                Spanned::new(identifier.token.to_string(), identifier.span.clone()),
            ));
            for cont in &mut self.function_containers {
                if cont.id == idx {
                    if let FunctionDeclaration::Function(func) = &cont.functions {
                        if func.get_parameters().len() != function.get_parameters().len() {
                            self.errors.lock().unwrap().report_error(
                                function.get_identifier_token().span.clone(),
                                CompilationErrorType::ParameterMismatch(function.get_identifier().to_string()),
                            );
                        }
                        if func.get_return_type() != function.get_return_type() {
                            self.errors.lock().unwrap().report_error(
                                function.get_return_type_token().span.clone(),
                                CompilationErrorType::ReturnTypeMismatch(function.get_identifier().to_string()),
                            );
                        } // may've been wrongly added as procedure before - get's corrected.
                    } else if let FunctionDeclaration::Procedure(func) = &cont.functions {
                        if func.get_parameters().len() != function.get_parameters().len() {
                            self.errors.lock().unwrap().report_error(
                                function.get_identifier_token().span.clone(),
                                CompilationErrorType::ParameterMismatch(function.get_identifier().to_string()),
                            );
                        }
                    }
                    cont.functions = FunctionDeclaration::Function(FunctionDeclarationAstNode::empty(
                        function.get_identifier().clone(),
                        function.get_parameters().clone(),
                        function.get_return_type().clone(),
                    ));
                    break;
                }
            }
        } else {
            if self.lang_version < 350 {
                self.errors.lock().unwrap().report_error(
                    function.get_identifier_token().span.clone(),
                    CompilationErrorType::FunctionNotFound(function.get_identifier().to_string()),
                );
            } else {
                let id = self.add_declaration(VariableType::Function, function.get_identifier_token());
                self.cur_func_impl = id;
                self.global_lookup.variable_lookup.insert(function.get_identifier().clone(), id);

                self.function_containers.push(FunctionContainer {
                    name: function.get_identifier().clone(),
                    parameter_index: None,
                    id,
                    functions: FunctionDeclaration::Function(FunctionDeclarationAstNode::empty(
                        function.get_identifier().clone(),
                        function.get_parameters().clone(),
                        function.get_return_type().clone(),
                    )),
                    lookup: VariableLookups::default(),
                    parameters: 0..0,
                    local_variables: 0..0,
                });
            }
        }

        self.start_parse_function_body();
        let start_parameter = self.references.len();
        self.add_parameters(function.get_parameters());
        let end_parameter = self.references.len();

        let start_locals = self.references.len();
        walk_function_implementation(self, function);
        let end_locals = self.references.len();
        let lookup = self.end_parse_function_body().unwrap();

        for f in &mut self.function_containers {
            if f.name == function.get_identifier() {
                f.lookup = lookup;
                f.parameters = start_parameter..end_parameter;
                f.local_variables = start_locals..end_locals;
                break;
            }
        }
        VariableType::None
    }

    fn visit_procedure_declaration(&mut self, proc_decl: &ProcedureDeclarationAstNode) -> VariableType {
        if self.has_variable_defined(proc_decl.get_identifier()) {
            self.errors.lock().unwrap().report_error(
                proc_decl.get_identifier_token().span.clone(),
                CompilationErrorType::VariableAlreadyDefined(proc_decl.get_identifier().to_string()),
            );
            return VariableType::None;
        }

        let id = self.add_declaration(VariableType::Procedure, proc_decl.get_identifier_token());
        self.global_lookup.variable_lookup.insert(proc_decl.get_identifier().clone(), id);

        self.function_containers.push(FunctionContainer {
            name: proc_decl.get_identifier().clone(),
            parameter_index: None,
            id,
            functions: FunctionDeclaration::Procedure(proc_decl.clone()),
            lookup: VariableLookups::default(),
            parameters: 0..0,
            local_variables: 0..0,
        });
        VariableType::None
    }

    fn visit_procedure_implementation(&mut self, procedure: &ProcedureImplementation) -> VariableType {
        if let Some(idx) = self.lookup_variable(procedure.get_identifier()) {
            // Procedure call may've added a function wrongly as a procedure, fix that here.
            {
                let (ref_kind, _refs) = &mut self.references[idx];
                match ref_kind.clone() {
                    ReferenceType::Procedure(_container_idx) => {
                        // All good.
                    }
                    ReferenceType::Function(_) => {
                        self.errors
                            .lock()
                            .unwrap()
                            .report_error(procedure.get_identifier_token().span.clone(), CompilationErrorType::ProcedureUsedAsFunction);
                    }
                    _ => {
                        self.errors.lock().unwrap().report_error(
                            procedure.get_identifier_token().span.clone(),
                            CompilationErrorType::InternalError(format!(
                                "Internal error: Found function implementation for non-procedure: {}",
                                procedure.get_identifier()
                            )),
                        );
                    }
                }
            }

            let identifier = procedure.get_identifier_token();
            self.references[idx].1.implementation = Some((
                self.errors.lock().unwrap().file_name().to_path_buf(),
                Spanned::new(identifier.token.to_string(), identifier.span.clone()),
            ));
            for cont in &mut self.function_containers {
                if cont.id == idx {
                    if let FunctionDeclaration::Procedure(func) = &cont.functions {
                        if func.get_parameters().len() != procedure.get_parameters().len() {
                            self.errors.lock().unwrap().report_error(
                                procedure.get_identifier_token().span.clone(),
                                CompilationErrorType::ParameterMismatch(procedure.get_identifier().to_string()),
                            );
                        }
                    }
                    cont.functions = FunctionDeclaration::Procedure(ProcedureDeclarationAstNode::empty(
                        procedure.get_identifier().clone(),
                        procedure.get_parameters().clone(),
                    ));
                    break;
                }
            }
        } else {
            if self.lang_version < 350 {
                self.errors.lock().unwrap().report_error(
                    procedure.get_identifier_token().span.clone(),
                    CompilationErrorType::ProcedureNotFound(procedure.get_identifier().to_string()),
                );
            } else {
                let id = self.add_declaration(VariableType::Procedure, procedure.get_identifier_token());
                self.global_lookup.variable_lookup.insert(procedure.get_identifier().clone(), id);
                self.references[id].1.implementation = Some((
                    self.errors.lock().unwrap().file_name().to_path_buf(),
                    Spanned::new(
                        procedure.get_identifier_token().token.to_string(),
                        procedure.get_identifier_token().span.clone(),
                    ),
                ));
                self.function_containers.push(FunctionContainer {
                    name: procedure.get_identifier().clone(),
                    parameter_index: None,
                    id,
                    functions: FunctionDeclaration::Procedure(ProcedureDeclarationAstNode::empty(
                        procedure.get_identifier().clone(),
                        procedure.get_parameters().clone(),
                    )),
                    lookup: VariableLookups::default(),
                    parameters: 0..0,
                    local_variables: 0..0,
                });
            }
        }

        self.start_parse_function_body();
        let start_parameter = self.references.len();
        self.add_parameters(procedure.get_parameters());
        let end_parameter = self.references.len();

        let start_locals = self.references.len();
        walk_procedure_implementation(self, procedure);
        let end_locals = self.references.len();
        let lookup = self.end_parse_function_body().unwrap();

        for f in &mut self.function_containers {
            if f.name == procedure.get_identifier() {
                if let FunctionDeclaration::Procedure(decl) = &f.functions {
                    if decl.get_parameters().len() != procedure.get_parameters().len() {
                        self.errors.lock().unwrap().report_error(
                            procedure.get_identifier_token().span.clone(),
                            CompilationErrorType::ParameterMismatch(procedure.get_identifier().to_string()),
                        );
                    }
                }
                f.lookup = lookup;

                f.parameters = start_parameter..end_parameter;
                f.local_variables = start_locals..end_locals;
                break;
            }
        }
        VariableType::None
    }

    /// The layout only reaches the PPE from `FIRST_TYPE_TABLE_RUNTIME` on, so an older
    /// target would drop it and leave every field access reading nothing.
    fn visit_type_declaration(&mut self, type_decl: &TypeDeclarationAstNode) -> VariableType {
        if self.runtime < FIRST_TYPE_TABLE_RUNTIME {
            self.errors.lock().unwrap().report_error(
                type_decl.get_identifier_token().span.clone(),
                ParserErrorType::TypeNeedsNewerRuntime(FIRST_TYPE_TABLE_RUNTIME),
            );
        }
        VariableType::None
    }

    fn visit_ast(&mut self, program: &crate::ast::Ast) -> VariableType {
        // Each file says which language it was read as, so the checks follow it.
        self.lang_version = program.language_version;
        // A routine may be called before the file gets to it, so every signature is
        // registered first - the same thing an explicit DECLARE does. A routine that
        // has one is left to it, so its own checks still run.
        let declared: Vec<unicase::Ascii<String>> = program
            .nodes
            .iter()
            .filter_map(|node| match node {
                crate::ast::AstNode::FunctionDeclaration(declaration) => Some(declaration.get_identifier().clone()),
                crate::ast::AstNode::ProcedureDeclaration(declaration) => Some(declaration.get_identifier().clone()),
                _ => None,
            })
            .collect();
        for node in &program.nodes {
            match node {
                crate::ast::AstNode::Function(function) if !declared.contains(function.get_identifier()) => self.predeclare_function(function),
                crate::ast::AstNode::Procedure(procedure) if !declared.contains(procedure.get_identifier()) => self.predeclare_procedure(procedure),
                _ => {}
            }
        }
        for node in &program.nodes {
            match node {
                crate::ast::AstNode::Function(_) | crate::ast::AstNode::Procedure(_) => {}
                _ => {
                    node.visit(self);
                }
            }
        }
        for node in &program.nodes {
            match node {
                crate::ast::AstNode::Function(_) | crate::ast::AstNode::Procedure(_) => {
                    node.visit(self);
                }
                _ => {}
            }
        }

        VariableType::None
    }
}
