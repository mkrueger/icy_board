use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    ast::{
        walk_function_implementation, walk_indexer_expression, walk_predefined_call_statement, walk_procedure_call_statement, walk_procedure_implementation,
        AstVisitor, CommentAstNode, Constant, ConstantExpression, Expression, FunctionCallExpression, FunctionDeclarationAstNode, FunctionImplementation,
        GosubStatement, GotoStatement, IdentifierExpression, LabelStatement, LetStatement, ParameterSpecifier, PredefinedCallStatement, ProcedureCallStatement,
        ProcedureDeclarationAstNode, ProcedureImplementation, VariableDeclarationStatement, VariableParameterSpecifier,
    },
    compiler::{user_data::UserDataMemberRegistry, CompilationErrorType, CompilationWarningType},
    executable::{
        EntryType, FuncOpCode, FunctionDefinition, FunctionValue, GenericVariableData, OpCode, ProcedureValue, TableEntry, VarHeader, VariableData,
        VariableTable, VariableType, VariableValue, FUNCTION_DEFINITIONS, USER_VARIABLES,
    },
    parser::{
        self,
        lexer::{Spanned, Token},
        ErrorReporter, ParserErrorType, UserTypeRegistry,
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
        if let Some(header) = &self.header {
            if let Some((_, decl)) = self.declaration.as_ref() {
                TableEntry::new(
                    decl.token.to_string(),
                    header.clone(),
                    self.variable_type.create_empty_value(),
                    EntryType::Variable,
                )
            } else if !self.usages.is_empty() {
                TableEntry::new(
                    self.usages.first().unwrap().1.token.to_string(),
                    header.clone(),
                    self.variable_type.create_empty_value(),
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
    version: u16,
    pub type_registry: UserTypeRegistry,

    pub errors: Arc<Mutex<ErrorReporter>>,
    pub references: Vec<(ReferenceType, References)>,

    /// Maps member references -> user type IDs
    pub user_type_lookup: HashMap<usize, u8>,

    pub function_type_lookup: HashMap<u64, SemanticInfo>,

    pub require_user_variables: bool,

    // labels
    label_count: usize,
    label_lookup_table: NameTableLookup,

    // variables
    global_lookup: VariableLookups,

    local_variable_lookup: Option<VariableLookups>,

    // constants
    pub function_containers: Vec<FunctionContainer>,

    cur_func_impl: usize,
    cur_func_call: u64,
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
    pub fn new(version: u16, errors: Arc<Mutex<ErrorReporter>>, type_registry: UserTypeRegistry) -> Self {
        let mut result = Self {
            version,
            errors,
            references: Vec::new(),
            type_registry,

            label_count: 0,
            label_lookup_table: HashMap::new(),
            user_type_lookup: HashMap::new(),
            function_type_lookup: HashMap::new(),

            global_lookup: VariableLookups::default(),
            local_variable_lookup: None,
            require_user_variables: false,
            cur_func_call: 0,
            cur_func_impl: 0,
            function_containers: Vec::new(),
        };
        for user_var in USER_VARIABLES.iter() {
            if user_var.version <= version {
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
                if user_var.version <= self.version {
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
            let (rt, r) = &mut self.references[i];
            if !matches!(rt, ReferenceType::Variable(_)) {
                continue;
            }
            if r.usages.is_empty() {
                continue;
            }
            r.variable_table_index = variable_table.len() + 1;
            variable_table.push(r.create_table_entry());
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
                    variable_table.push(new_entry);
                    continue;
                }
                if !matches!(rt, ReferenceType::Variable(_)) {
                    continue;
                }
                let mut new_entry = r.create_table_entry();
                new_entry.entry_type = EntryType::Parameter;
                variable_table.push(new_entry);
            }

            for idx in f.local_variables.clone() {
                let (rt, r) = &self.references[idx];
                if !matches!(rt, ReferenceType::Variable(_)) {
                    continue;
                }
                let mut new_entry = r.create_table_entry();
                new_entry.entry_type = EntryType::LocalVariable;
                variable_table.push(new_entry);
            }

            if let FunctionDeclaration::Function(f) = &f.functions {
                let return_type = f.get_return_type();
                let header = VarHeader {
                    id,
                    dim: 0,
                    vector_size: 0,
                    matrix_size: 0,
                    cube_size: 0,
                    variable_type: return_type,
                    flags: 0,
                };
                variable_table.push(TableEntry::new(
                    format!("{} result", f.get_identifier()),
                    header,
                    return_type.create_empty_value(),
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

    fn add_declaration(&mut self, reftype: ReferenceType, variable_type: VariableType, identifier_token: &Spanned<parser::lexer::Token>) {
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

        // TODO: clear the local label lookup on each new functions for future language versions?
        // self.label_lookup_table.clear();
    }

    fn end_parse_function_body(&mut self) -> Option<VariableLookups> {
        self.local_variable_lookup.take()
    }

    fn has_variable_defined(&self, id: &unicase::Ascii<String>) -> bool {
        if let Some(local_lookup) = &self.local_variable_lookup {
            return local_lookup.variable_lookup.contains_key(id);
        }
        self.global_lookup.variable_lookup.contains_key(id)
    }

    fn add_predefined_variable(&mut self, name: &str, val: &VariableValue) {
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
        let id = self.references.len();
        self.add_declaration(ReferenceType::Variable(id), variable_type, identifier);

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

        if let Some(local_lookup) = &mut self.local_variable_lookup {
            local_lookup.variable_lookup.insert(unicase::Ascii::new(identifier.token.to_string()), id);
        } else {
            self.global_lookup.variable_lookup.insert(unicase::Ascii::new(identifier.token.to_string()), id);
        }
    }

    fn lookup_variable(&mut self, id: &unicase::Ascii<String>) -> Option<usize> {
        if let Some(local_lookup) = &self.local_variable_lookup {
            if let Some(idx) = local_lookup.variable_lookup.get(id) {
                return Some(*idx);
            }
        }

        if let Some(idx) = self.global_lookup.variable_lookup.get(id) {
            return Some(*idx);
        }
        None
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
                    let id = self.references.len();
                    self.add_declaration(
                        ReferenceType::Variable(id),
                        param.get_variable_type(),
                        param.get_variable().as_ref().unwrap().get_identifier_token(),
                    );
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
                    let id = self.references.len();
                    self.add_declaration(
                        ReferenceType::Function(self.function_containers.len()),
                        VariableType::Function,
                        func.get_identifier_token(),
                    );
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
                            func.get_parameters()
                                .iter()
                                .map(|_a| ParameterSpecifier::Variable(VariableParameterSpecifier::empty(false, VariableType::None, None)))
                                .collect(),
                            VariableType::None,
                        )),
                        lookup: VariableLookups::default(),
                        parameters: 0..0,
                        local_variables: 0..0,
                    });
                }
                ParameterSpecifier::Procedure(func) => {
                    let id = self.references.len();
                    self.add_declaration(
                        ReferenceType::Function(self.function_containers.len()),
                        VariableType::Function,
                        func.get_identifier_token(),
                    );
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
                            func.get_parameters()
                                .iter()
                                .map(|_a| ParameterSpecifier::Variable(VariableParameterSpecifier::empty(false, VariableType::None, None)))
                                .collect(),
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

    pub fn finish(&mut self) {
        for (rt, r) in &mut self.references.iter() {
            if matches!(rt, ReferenceType::Label(_)) {
                if r.declaration.is_none() {
                    self.errors.lock().unwrap().report_error(
                        r.usages.first().unwrap().1.span.clone(),
                        CompilationErrorType::LabelNotFound(r.usages.first().unwrap().1.token.to_string()),
                    );
                } else if r.usages.is_empty() {
                    if let Some((_, declaration)) = &r.declaration {
                        if ":~BEGIN~" == declaration.token || declaration.token.starts_with(":*(") {
                            continue;
                        }
                    }
                    self.errors.lock().unwrap().report_warning(
                        r.declaration.as_ref().unwrap().1.span.clone(),
                        CompilationWarningType::UnusedLabel(r.declaration.as_ref().unwrap().1.token.to_string()),
                    );
                }
                continue;
            }

            let Some((_, decl)) = &r.declaration else {
                continue;
            };

            if r.variable_type == VariableType::Function || r.variable_type == VariableType::Procedure {
                if r.implementation.is_none() {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(decl.span.clone(), CompilationErrorType::MissingImplementation(decl.token.to_string()));
                }
                if r.usages.is_empty() {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_warning(decl.span.clone(), CompilationErrorType::UnusedFunction(decl.token.to_string()));
                }
            } else if matches!(rt, ReferenceType::Variable(_)) && r.usages.is_empty() {
                self.errors
                    .lock()
                    .unwrap()
                    .report_warning(decl.span.clone(), CompilationErrorType::UnusedVariable(decl.token.to_string()));
            }
        }

        // search if any user variables are used.
        if !self.require_user_variables {
            for (i, user_var) in USER_VARIABLES.iter().enumerate() {
                if user_var.version <= self.version && !self.references[i].1.usages.is_empty() {
                    self.require_user_variables = true;
                    break;
                }
            }
        }
    }
}

impl AstVisitor<VariableType> for SemanticVisitor {
    fn visit_identifier_expression(&mut self, identifier: &IdentifierExpression) -> VariableType {
        let predef = FunctionDefinition::get_function_definitions(identifier.get_identifier());
        if !predef.is_empty() {
            let def = &FUNCTION_DEFINITIONS[predef[0]];
            if self.cur_func_call > 0 {
                self.function_type_lookup.insert(self.cur_func_call, SemanticInfo::PredefFunctionGroup(predef));
            }
            return def.return_type;
        } else if let Some(idx) = self.lookup_variable(identifier.get_identifier()) {
            let (rt, r) = &mut self.references[idx];
            let identifier = identifier.get_identifier_token();
            if self.cur_func_call > 0 {
                if let ReferenceType::Function(func_idx) = rt {
                    println!("Function call: {} -> {}", self.cur_func_call, *func_idx);
                    self.function_type_lookup.insert(self.cur_func_call, SemanticInfo::FunctionReference(*func_idx));
                } else if let ReferenceType::Variable(func_idx) = rt {
                    self.function_type_lookup.insert(self.cur_func_call, SemanticInfo::VariableReference(*func_idx));
                }
            }

            r.usages.push((
                self.errors.lock().unwrap().file_name().to_path_buf(),
                Spanned::new(identifier.token.to_string(), identifier.span.clone()),
            ));

            r.variable_type
        } else {
            if self.version < 350 || self.cur_func_call == 0 {
                self.errors.lock().unwrap().report_error(
                    identifier.get_identifier_token().span.clone(),
                    CompilationErrorType::VariableNotFound(identifier.get_identifier().to_string()),
                );
            }
            VariableType::None
        }
    }

    fn visit_member_reference_expression(&mut self, member_reference_expression: &crate::ast::MemberReferenceExpression) -> VariableType {
        let t = member_reference_expression.get_expression().visit(self);
        if let VariableType::UserData(d) = t {
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
        for arg in call.get_arguments() {
            arg.visit(self);
        }

        match self.function_type_lookup.get(&call.id) {
            Some(SemanticInfo::FunctionReference(idx)) => {
                let func = &self.function_containers[*idx];
                let arg_count = match &func.functions {
                    FunctionDeclaration::Function(f) => {
                        res = f.get_return_type();
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

                let (rt, r) = &mut self.references[*idx];

                let arg_count = if let ReferenceType::Variable(_func) = rt {
                    r.header.as_ref().unwrap().dim as usize
                } else {
                    0
                };
                res = r.variable_type;
                self.check_expr_arg_count(arg_count, call.get_arguments().len(), call.get_expression());
            }
            Some(SemanticInfo::PredefFunctionGroup(funcs)) => {
                for func in funcs {
                    let def = &FUNCTION_DEFINITIONS[*func];
                    if def.arg_descr as usize == call.get_arguments().len() {
                        if self.version < def.version {
                            self.errors.lock().unwrap().report_error(
                                call.get_expression().get_span(),
                                ParserErrorType::FunctionVersionNotSupported(def.opcode, def.version, self.version),
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
                    FUNCTION_DEFINITIONS[funcs[0]].arg_descr as usize,
                    call.get_arguments().len(),
                    call.get_expression(),
                );
            }

            _ => {
                if self.version < 350 || !is_ident {
                    self.errors.lock().unwrap().report_error(
                        call.get_expression().get_span(),
                        CompilationErrorType::FunctionNotFound(call.get_expression().to_string()),
                    );
                } else if let Expression::Identifier(ident) = call.get_expression() {
                    let id: usize = self.references.len();
                    self.global_lookup.variable_lookup.insert(ident.get_identifier().clone(), id);
                    self.add_declaration(
                        ReferenceType::Function(self.function_containers.len()),
                        VariableType::Function,
                        ident.get_identifier_token(),
                    );
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
            } else {
                if let Some(header) = &self.references[idx].1.header {
                    self.check_arg_count(header.dim as usize, let_stmt.get_arguments().len(), let_stmt.get_identifier_token());
                } else {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(let_stmt.get_identifier_token().span.clone(), CompilationErrorType::InvalidLetVariable);
                }

                self.add_reference_to(let_stmt.get_identifier_token(), idx);
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
        let_stmt.get_value_expression().visit(self);
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
                if let FunctionDeclaration::Function(f) = &f.functions {
                    self.check_arg_count(f.get_parameters().len(), call.get_arguments().len(), call.get_identifier_token());
                }

                self.add_reference_to(call.get_identifier_token(), idx);
                found = true;
            }

            if matches!(self.references[idx].0, ReferenceType::Procedure(_)) {
                let func_container = self.function_containers.iter().find(|p| p.name == call.get_identifier()).unwrap();

                if let FunctionDeclaration::Procedure(f) = &func_container.functions {
                    let arg_count = call.get_arguments().len();
                    let par_len = f.get_parameters().len();

                    let arg_count = arg_count.min(par_len);
                    let pass_flags = f.get_pass_flags();
                    self.check_arg_count(par_len, arg_count, call.get_identifier_token());
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
            if self.version < 350 {
                self.errors.lock().unwrap().report_error(
                    call.get_identifier_token().span.clone(),
                    CompilationErrorType::ProcedureNotFound(call.get_identifier().to_string()),
                );
            } else {
                let id = self.references.len();
                self.global_lookup.variable_lookup.insert(call.get_identifier().clone(), id);
                self.add_declaration(
                    ReferenceType::Procedure(self.function_containers.len()),
                    VariableType::Procedure,
                    call.get_identifier_token(),
                );
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
        let id = self.references.len();
        self.global_lookup.variable_lookup.insert(func_decl.get_identifier().clone(), id);
        self.add_declaration(
            ReferenceType::Function(self.function_containers.len()),
            VariableType::Function,
            func_decl.get_identifier_token(),
        );
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
            if self.version < 350 {
                self.errors.lock().unwrap().report_error(
                    function.get_identifier_token().span.clone(),
                    CompilationErrorType::FunctionNotFound(function.get_identifier().to_string()),
                );
            } else {
                let id = self.references.len();
                self.cur_func_impl = id;
                self.global_lookup.variable_lookup.insert(function.get_identifier().clone(), id);
                self.add_declaration(
                    ReferenceType::Function(self.function_containers.len()),
                    VariableType::Function,
                    function.get_identifier_token(),
                );
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
                if let FunctionDeclaration::Function(decl) = &f.functions {
                    if decl.get_return_type() != function.get_return_type() {
                        self.errors.lock().unwrap().report_error(
                            function.get_return_type_token().span.clone(),
                            CompilationErrorType::ReturnTypeMismatch(function.get_identifier().to_string()),
                        );
                    }

                    if decl.get_parameters().len() != function.get_parameters().len() {
                        self.errors.lock().unwrap().report_error(
                            function.get_identifier_token().span.clone(),
                            CompilationErrorType::ParameterMismatch(function.get_identifier().to_string()),
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

    fn visit_procedure_declaration(&mut self, proc_decl: &ProcedureDeclarationAstNode) -> VariableType {
        if self.has_variable_defined(proc_decl.get_identifier()) {
            self.errors.lock().unwrap().report_error(
                proc_decl.get_identifier_token().span.clone(),
                CompilationErrorType::VariableAlreadyDefined(proc_decl.get_identifier().to_string()),
            );
            return VariableType::None;
        }
        let id = self.references.len();
        self.global_lookup.variable_lookup.insert(proc_decl.get_identifier().clone(), id);
        self.add_declaration(
            ReferenceType::Procedure(self.function_containers.len()),
            VariableType::Procedure,
            proc_decl.get_identifier_token(),
        );

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
            if self.version < 350 {
                self.errors.lock().unwrap().report_error(
                    procedure.get_identifier_token().span.clone(),
                    CompilationErrorType::ProcedureNotFound(procedure.get_identifier().to_string()),
                );
            } else {
                let id = self.references.len();
                self.global_lookup.variable_lookup.insert(procedure.get_identifier().clone(), id);
                self.add_declaration(
                    ReferenceType::Procedure(self.function_containers.len()),
                    VariableType::Procedure,
                    procedure.get_identifier_token(),
                );
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

    fn visit_ast(&mut self, program: &crate::ast::Ast) -> VariableType {
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
