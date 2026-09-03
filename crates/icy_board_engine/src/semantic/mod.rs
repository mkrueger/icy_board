use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    ast::{Constant, Expression, FunctionDeclarationAstNode, ParameterSpecifier, ProcedureDeclarationAstNode, Statement},
    compiler::{CompilationErrorType, CompilationWarningType, optimizer::statement_reachability, workspace::Workspace},
    executable::{
        EntryType, FuncOpCode, FunctionValue, GenericVariableData, ProcedureValue, TableEntry, USER_VARIABLES, VarHeader, VariableData, VariableType,
        VariableValue,
    },
    hir::{CallId, SymbolId},
    parser::{
        self, ErrorReporter, ParserErrorType, UserTypeRegistry,
        lexer::{Spanned, Token},
    },
};

mod arrays;
pub mod call_graph;
#[cfg(test)]
mod find_references_tests;
mod members;
mod references;
mod symbols;
mod variable_table;
mod visitor;

use arrays::ArrayShape;
use call_graph::CallGraph;
pub use members::{ARRAY_MEMBERS, ARRAY_PROCEDURES, ArrayMember, BYTES_MEMBERS, STRING_MEMBERS, ScalarMember, array_member, array_procedure};
use members::{StaticReceiver, bytes_member, bytes_member_type, string_member, string_member_type, string_type_name, takes_whole_array};
use symbols::parameter_lists_match;
pub use symbols::{FunctionContainer, FunctionDeclaration, ModuleExport, ModuleSymbolKind, ReferenceType, References, SemanticInfo, VariableLookups};
pub use variable_table::LookupVariabeleTable;

type NameTableLookup = HashMap<unicase::Ascii<String>, usize>;

pub struct SemanticVisitor {
    lang_version: u16,
    runtime: u16,
    pub type_registry: UserTypeRegistry,

    pub errors: Arc<Mutex<ErrorReporter>>,
    current_file: Arc<PathBuf>,
    pub references: Vec<(ReferenceType, References)>,
    reference_owners: HashMap<usize, HashSet<Option<usize>>>,
    pub module_exports: HashMap<unicase::Ascii<String>, Vec<ModuleExport>>,

    /// Maps member references -> user type IDs
    pub user_type_lookup: HashMap<usize, u8>,

    /// Maps built-in scalar member references -> receiver types.
    pub member_receiver_type_lookup: HashMap<usize, VariableType>,

    /// Maps a type name used as a receiver -> the builtin that hands its instance back.
    pub instance_provider_lookup: HashMap<usize, FuncOpCode>,

    /// Maps a type name a static member was called on -> that type's id.
    pub static_receiver_lookup: HashMap<usize, u8>,

    pub function_type_lookup: HashMap<CallId, SemanticInfo>,
    pub call_graph: CallGraph,
    member_array_returns: HashMap<CallId, (VariableType, u8)>,

    pub require_user_variables: bool,
    allow_routine_reference: bool,
    allowed_routine_reference_spans: HashSet<usize>,
    function_return_value_spans: HashSet<usize>,

    // labels
    label_count: usize,
    label_lookup_table: NameTableLookup,
    label_reference_lookup: HashMap<usize, usize>,
    predefined_function_reference_lookup: HashMap<i16, usize>,
    predefined_procedure_reference_lookup: HashMap<i16, usize>,

    // variables
    global_lookup: VariableLookups,

    local_variable_lookup: Option<VariableLookups>,

    /// Named constants never reach the variable table - the value takes the place of
    /// the name - so they are kept beside it.
    global_constants: HashMap<unicase::Ascii<String>, (VariableType, VariableValue, usize)>,
    local_constants: Option<HashMap<unicase::Ascii<String>, (VariableType, VariableValue, usize)>>,

    /// Where the FOR statements of the current file keep their count, which a
    /// desugared loop compares and steps itself.
    loop_counters: HashSet<usize>,

    // constants
    pub function_containers: Vec<FunctionContainer>,

    cur_func_impl: Option<usize>,
    cur_func_call: u64,
    control_flow_liveness: bool,
    references_are_reachable: bool,

    /// The type of a receiver a member reference already walked. A call and the member
    /// reference inside it both need it, and walking it twice made a chain like `a[i][j][k]`
    /// cost 2^n.
    receiver_types: HashMap<usize, VariableType>,

    /// The member reference a call is about to resolve, so the reference can tell whether it was
    /// written on its own.
    callee_member: Option<usize>,

    /// The member call whose return value is being discarded as a statement.
    statement_member_call: Option<CallId>,

    last_lookup_index: usize,
}

impl SemanticVisitor {
    pub fn set_file_name(&mut self, file_name: &std::path::Path) {
        self.current_file = Arc::new(file_name.to_path_buf());
        self.errors.lock().unwrap().set_file_name(file_name);
    }

    #[cfg(test)]
    pub(crate) fn set_control_flow_liveness(&mut self, enabled: bool) {
        self.control_flow_liveness = enabled;
    }

    pub fn set_modules(&mut self, asts: &[&crate::ast::Ast]) {
        self.module_exports.clear();
        for ast in asts {
            let Some(module) = &ast.module else { continue };
            let mut exports = Vec::new();
            for node in &ast.nodes {
                let mut add = |name: &str, offset: usize, kind: ModuleSymbolKind| {
                    if module.visibility_at(offset) == crate::ast::Visibility::Public {
                        exports.push(ModuleExport { name: name.to_string(), kind });
                    }
                };
                match node {
                    crate::ast::AstNode::Function(value) => add(
                        value.get_identifier().as_str(),
                        value.get_identifier_token().span.start,
                        ModuleSymbolKind::Function,
                    ),
                    crate::ast::AstNode::Procedure(value) => add(
                        value.get_identifier().as_str(),
                        value.get_identifier_token().span.start,
                        ModuleSymbolKind::Procedure,
                    ),
                    crate::ast::AstNode::FunctionDeclaration(value) => add(
                        value.get_identifier().as_str(),
                        value.get_identifier_token().span.start,
                        ModuleSymbolKind::Function,
                    ),
                    crate::ast::AstNode::ProcedureDeclaration(value) => add(
                        value.get_identifier().as_str(),
                        value.get_identifier_token().span.start,
                        ModuleSymbolKind::Procedure,
                    ),
                    crate::ast::AstNode::TypeDeclaration(value) => {
                        add(value.get_identifier().as_str(), value.get_identifier_token().span.start, ModuleSymbolKind::Type)
                    }
                    crate::ast::AstNode::EnumDeclaration(value) => {
                        add(value.get_identifier().as_str(), value.get_identifier_token().span.start, ModuleSymbolKind::Enum)
                    }
                    crate::ast::AstNode::TopLevelStatement(crate::ast::Statement::VariableDeclaration(value)) => {
                        for variable in value.get_variables() {
                            add(
                                variable.get_identifier().as_str(),
                                variable.get_identifier_token().span.start,
                                ModuleSymbolKind::Variable,
                            );
                        }
                    }
                    crate::ast::AstNode::TopLevelStatement(crate::ast::Statement::ConstDeclaration(value)) => {
                        add(
                            value.get_identifier().as_str(),
                            value.get_identifier_token().span.start,
                            ModuleSymbolKind::Constant,
                        );
                    }
                    crate::ast::AstNode::TopLevelStatement(_) | crate::ast::AstNode::Main(_) => {}
                }
            }
            let module_exports = self.module_exports.entry(module.name().clone()).or_default();
            for export in exports {
                if !module_exports
                    .iter()
                    .any(|existing| existing.name.eq_ignore_ascii_case(&export.name) && existing.kind == export.kind)
                {
                    module_exports.push(export);
                }
            }
        }
    }

    pub(crate) fn storage_type(&self, source_type: VariableType) -> VariableType {
        if self.type_registry.is_enum_type(source_type) {
            VariableType::Integer
        } else {
            source_type
        }
    }

    fn source_type_name(&self, variable_type: VariableType) -> String {
        if let VariableType::UserData(id) = variable_type
            && let Some(definition) = self.type_registry.get_enum_from_id(id)
        {
            return definition.name.to_string();
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
            Expression::Identifier(identifier) => self.lookup_constant(identifier.get_identifier()).map(|(variable_type, _, _)| *variable_type),
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

    pub fn is_function_return_value(&self, span_start: usize) -> bool {
        self.function_return_value_spans.contains(&span_start)
    }

    pub fn new(workspace: &Workspace, errors: Arc<Mutex<ErrorReporter>>, type_registry: UserTypeRegistry) -> Self {
        let current_file = Arc::new(errors.lock().unwrap().file_name().to_path_buf());
        let mut result = Self {
            lang_version: workspace.language_version(),
            runtime: workspace.runtime(),
            errors,
            current_file,
            references: Vec::new(),
            reference_owners: HashMap::new(),
            module_exports: HashMap::new(),
            type_registry,

            label_count: 0,
            label_lookup_table: HashMap::new(),
            label_reference_lookup: HashMap::new(),
            predefined_function_reference_lookup: HashMap::new(),
            predefined_procedure_reference_lookup: HashMap::new(),
            user_type_lookup: HashMap::new(),
            member_receiver_type_lookup: HashMap::new(),
            instance_provider_lookup: HashMap::new(),
            static_receiver_lookup: HashMap::new(),
            function_type_lookup: HashMap::new(),
            call_graph: CallGraph::default(),
            member_array_returns: HashMap::new(),

            global_lookup: VariableLookups::default(),
            local_variable_lookup: None,
            global_constants: HashMap::new(),
            local_constants: None,
            loop_counters: HashSet::new(),
            require_user_variables: false,
            allow_routine_reference: false,
            allowed_routine_reference_spans: HashSet::new(),
            function_return_value_spans: HashSet::new(),
            cur_func_call: 0,
            receiver_types: HashMap::new(),
            callee_member: None,
            statement_member_call: None,
            cur_func_impl: None,
            control_flow_liveness: true,
            references_are_reachable: true,
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
                if user_var.runtime_version <= self.runtime {
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

        let mut variables: Vec<usize> = self.global_lookup.variable_lookup.values().copied().collect();
        variables.sort_unstable();
        for i in variables {
            let is_live = self.reference_is_live(i);
            let storage_type = self.storage_type(self.references[i].1.variable_type);
            let (rt, r) = &mut self.references[i];
            if !matches!(rt, ReferenceType::Variable(_)) {
                continue;
            }
            if !is_live {
                continue;
            }

            // Skip user variables - they've already been added above
            // Check if this is a predefined user variable by checking if it has no declaration
            // but has usages (predefined variables have no declaration)
            if self.require_user_variables && r.declaration.is_none() {
                // This is a predefined variable that's being used
                // Find it in the already-added user variables and update the reference
                if let Some(name) = r.usages.first().map(|(_, s)| &s.token)
                    && let Some(idx) = variable_table.lookup_variable_index(&unicase::Ascii::new(name.clone()))
                {
                    r.variable_table_index = idx;
                    continue;
                }
            }

            r.variable_table_index = variable_table.len() + 1;
            let entry = r.create_table_entry_as(storage_type);
            variable_table.push(entry);
        }

        for f in &self.function_containers {
            if f.parameter_index.is_some() {
                continue;
            }
            {
                let (_rt, r) = &mut self.references[f.id];
                if !self.call_graph.is_reachable(SymbolId(f.id)) {
                    continue;
                }
                r.variable_table_index = variable_table.variable_table.len() + 1;
            }
            let mut locals = 0usize;
            for idx in f.local_variables.start..f.local_variables.end {
                let is_live = self.reference_is_live(idx);
                let (rt, _reference) = &self.references[idx];
                if !matches!(rt, ReferenceType::Variable(_)) || !is_live {
                    continue;
                }
                locals += 1;
            }
            let id = variable_table.variable_table.len() + 1;
            let parameters = f.parameters.len();
            if parameters > u8::MAX as usize {
                let span = match &f.functions {
                    FunctionDeclaration::Function(function) => function.get_identifier_token().span.clone(),
                    FunctionDeclaration::Procedure(procedure) => procedure.get_identifier_token().span.clone(),
                };
                self.errors.lock().unwrap().report_error(
                    span,
                    CompilationErrorType::TooManyRoutineParameters(f.name.to_string(), parameters, u8::MAX as usize),
                );
            }

            if let FunctionDeclaration::Function(func) = &f.functions {
                let maximum_locals = u8::MAX as usize - 1;
                if locals > maximum_locals {
                    self.errors.lock().unwrap().report_error(
                        func.get_identifier_token().span.clone(),
                        CompilationErrorType::TooManyRoutineLocals(f.name.to_string(), locals, maximum_locals),
                    );
                }
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
                    parameters: parameters.min(u8::MAX as usize) as u8,
                    local_variables: (locals + 1).min(u8::MAX as usize) as u8,
                    start_offset: 0,
                    first_var_id: id as i16,
                    return_var: (id + locals + parameters + 1) as i16,
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
                if let Some((index, _)) = proc
                    .get_parameters()
                    .iter()
                    .enumerate()
                    .skip(u16::BITS as usize)
                    .find(|(_, parameter)| parameter.is_var())
                {
                    self.errors.lock().unwrap().report_error(
                        proc.get_identifier_token().span.clone(),
                        CompilationErrorType::VarParameterOutOfRange(f.name.to_string(), index + 1, u16::BITS as usize),
                    );
                }
                if locals > u8::MAX as usize {
                    self.errors.lock().unwrap().report_error(
                        proc.get_identifier_token().span.clone(),
                        CompilationErrorType::TooManyRoutineLocals(f.name.to_string(), locals, u8::MAX as usize),
                    );
                }
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
                    parameters: parameters.min(u8::MAX as usize) as u8,
                    local_variables: locals.min(u8::MAX as usize) as u8,
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
            }

            for idx in f.parameters.start..f.parameters.end {
                let storage_type = self.storage_type(self.references[idx].1.variable_type);
                let (rt, r) = &mut self.references[idx];
                if let ReferenceType::Function(func) = rt {
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

            for idx in f.local_variables.start..f.local_variables.end {
                let is_live = self.reference_is_live(idx);
                let (rt, r) = &self.references[idx];
                if !matches!(rt, ReferenceType::Variable(_)) || !is_live {
                    continue;
                }
                let mut new_entry = r.create_table_entry_as(self.storage_type(r.variable_type));
                new_entry.entry_type = EntryType::LocalVariable;
                variable_table.push(new_entry);
            }

            if let FunctionDeclaration::Function(f) = &f.functions {
                let return_type = f.get_return_type();
                let storage_type = self.storage_type(return_type);
                let return_rank = f.get_return_rank();
                let header = VarHeader {
                    id,
                    dim: return_rank,
                    vector_size: 0,
                    matrix_size: 0,
                    cube_size: 0,
                    variable_type: storage_type,
                    flags: if return_rank > 0 {
                        crate::executable::variable_table::VARIABLE_FLAG_DYNAMIC_ARRAY
                    } else {
                        0
                    },
                };
                let value = if return_rank == 0 {
                    storage_type.create_empty_value()
                } else {
                    VariableValue {
                        vtype: storage_type,
                        data: VariableData::default(),
                        generic_data: header.create_generic_data().unwrap_or_default(),
                    }
                };
                variable_table.push(TableEntry::new(format!("{} result", f.get_identifier()), header, value, EntryType::Variable));
            }

            variable_table.end_compile_function_body();
        }

        for c in &self.global_lookup.constants {
            variable_table.add_constant(c);
        }
        for f in &self.function_containers {
            if !self.call_graph.is_reachable(SymbolId(f.id)) {
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

    fn lookup_constant(&self, id: &unicase::Ascii<String>) -> Option<&(VariableType, VariableValue, usize)> {
        if let Some(local) = &self.local_constants
            && let Some(constant) = local.get(id)
        {
            return Some(constant);
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
        assert!(
            !self.has_variable_defined(&unicase::Ascii::new(name.to_string())),
            "Variable {name} already exists"
        );

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
        let dynamic = dim > 0 && vector_size == usize::MAX;

        let header = VarHeader {
            id,
            variable_type,
            dim,
            vector_size: if dynamic { 0 } else { vector_size },
            matrix_size: if dynamic { 0 } else { matrix_size },
            cube_size: if dynamic { 0 } else { cube_size },
            flags: if dynamic {
                crate::executable::variable_table::VARIABLE_FLAG_DYNAMIC_ARRAY
            } else {
                0
            },
        };
        self.references.last_mut().unwrap().1.header = Some(header);

        assert!(
            !self.has_variable_defined(&unicase::Ascii::new(identifier.token.to_string())),
            "Variable {} already exists",
            identifier.token
        );

        if let Some(local_lookup) = &mut self.local_variable_lookup {
            local_lookup.variable_lookup.insert(unicase::Ascii::new(identifier.token.to_string()), id);
        } else {
            self.global_lookup.variable_lookup.insert(unicase::Ascii::new(identifier.token.to_string()), id);
        }
    }

    fn lookup_variable(&mut self, id: &unicase::Ascii<String>) -> Option<usize> {
        if let Some(local_lookup) = &self.local_variable_lookup
            && let Some(idx) = local_lookup.variable_lookup.get(id)
        {
            self.last_lookup_index = *idx;
            return Some(*idx);
        }

        if let Some(idx) = self.global_lookup.variable_lookup.get(id) {
            self.last_lookup_index = *idx;
            return Some(*idx);
        }
        None
    }

    fn routine_container(&self, reference: usize) -> Option<&FunctionContainer> {
        let container = self.routine_container_index(reference)?;
        self.function_containers.get(container)
    }

    fn routine_container_index(&self, reference: usize) -> Option<usize> {
        match self.references.get(reference)?.0 {
            ReferenceType::Function(container) | ReferenceType::Procedure(container) => Some(container),
            _ => None,
        }
    }

    fn visit_statement_sequence(&mut self, statements: &[Statement]) -> bool {
        if !self.control_flow_liveness {
            for statement in statements {
                statement.visit(self);
            }
            return true;
        }
        let outer_reachability = self.references_are_reachable;
        for (statement, reachable) in statement_reachability(statements) {
            self.references_are_reachable = outer_reachability && reachable;
            statement.visit(self);
        }
        self.references_are_reachable = outer_reachability;
        true
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
                        self.current_file.clone(),
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
                        self.current_file.clone(),
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

        if let Expression::FunctionCall(a) = expr
            && let Some(SemanticInfo::VariableReference(_)) = self.function_type_lookup.get(&CallId(a.id))
        {
            return;
        }
        if let Expression::Indexer(_) = expr {
            return;
        }

        self.errors
            .lock()
            .unwrap()
            .report_error(expr.get_span().clone(), CompilationErrorType::VariableExpected(arg_num + 1));
    }

    fn resolved_record_io_type(&mut self, expression: &Expression) -> VariableType {
        match expression {
            Expression::Identifier(identifier) => self
                .lookup_variable(identifier.get_identifier())
                .map_or(VariableType::None, |index| self.references[index].1.variable_type),
            Expression::RecordLiteral(record) => record.get_variable_type(),
            Expression::Parens(parens) => self.resolved_record_io_type(parens.get_expression()),
            Expression::MemberReference(member) => {
                let Some(type_id) = self.user_type_lookup.get(&member.get_identifier_token().span.start).copied() else {
                    return VariableType::None;
                };
                self.type_registry
                    .get_record_type_from_id(type_id)
                    .and_then(|definition| definition.field_index(member.get_identifier()).and_then(|index| definition.field_type(index)))
                    .unwrap_or(VariableType::None)
            }
            _ => VariableType::None,
        }
    }

    fn first_unserializable_record_field(&self, type_id: u8, prefix: &str) -> Option<(String, VariableType)> {
        let definition = self.type_registry.get_user_type_from_id(type_id)?;
        for (name, field) in &definition.fields {
            let path = if prefix.is_empty() { name.to_string() } else { format!("{prefix}.{name}") };
            match field.variable_type {
                VariableType::UserData(id) if crate::parser::is_user_declared_type(id) => {
                    if let Some(invalid) = self.first_unserializable_record_field(id, &path) {
                        return Some(invalid);
                    }
                }
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
                | VariableType::ULong => {}
                other => return Some((path, other)),
            }
        }
        None
    }

    /// Resolves a field of a record the program declared and remembers the type, so
    /// code generation can look the field up again by the member's source position.
    fn resolve_record_field(&mut self, type_id: u8, member: &unicase::Ascii<String>, span: &core::ops::Range<usize>) -> VariableType {
        let Some(definition) = self.type_registry.get_record_type_from_id(type_id) else {
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

    fn check_expr_arg_range(&self, required: usize, maximum: usize, arg_count: usize, expr: &Expression) {
        if arg_count < required {
            self.errors
                .lock()
                .unwrap()
                .report_error(expr.get_span(), ParserErrorType::TooFewArguments(expr.to_string(), arg_count, required as i8));
        }
        if arg_count > maximum {
            self.errors
                .lock()
                .unwrap()
                .report_error(expr.get_span(), ParserErrorType::TooManyArguments(expr.to_string(), arg_count, maximum as i8));
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
        self.call_graph.finish();
        for (reference_index, (rt, r)) in self.references.iter().enumerate() {
            if matches!(rt, ReferenceType::Label(_)) {
                if r.declaration.is_none() {
                    if let Some((file, span)) = r.usages.first() {
                        self.errors.lock().unwrap().report_error_file(
                            file.as_ref().clone(),
                            span.span.clone(),
                            CompilationErrorType::LabelNotFound(span.token.clone()),
                        );
                    }
                } else if r.usages.is_empty()
                    && let Some((file_name, declaration)) = &r.declaration
                {
                    if ":~BEGIN~" == declaration.token || declaration.token.starts_with(":*(") {
                        continue;
                    }
                    self.errors.lock().unwrap().report_warning_file(
                        file_name.as_ref().clone(),
                        declaration.span.clone(),
                        CompilationWarningType::UnusedLabel(declaration.token.clone()),
                    );
                }
                continue;
            }

            let Some((file, decl)) = &r.declaration else {
                continue;
            };

            if r.variable_type == VariableType::Function || r.variable_type == VariableType::Procedure {
                if r.implementation.is_none() {
                    self.errors.lock().unwrap().report_error_file(
                        file.as_ref().clone(),
                        decl.span.clone(),
                        CompilationErrorType::MissingImplementation(decl.token.clone()),
                    );
                }
                if !self.call_graph.is_reachable(SymbolId(reference_index)) {
                    self.errors.lock().unwrap().report_warning_file(
                        file.as_ref().clone(),
                        decl.span.clone(),
                        CompilationErrorType::UnusedFunction(decl.token.clone()),
                    );
                }
            } else if matches!(rt, ReferenceType::Variable(_)) && r.usages.is_empty() {
                // The enclosing routine already reports variables used only in unreachable code.
                self.errors.lock().unwrap().report_warning_file(
                    file.as_ref().clone(),
                    decl.span.clone(),
                    CompilationErrorType::UnusedVariable(decl.token.clone()),
                );
            }
        }

        // search if any user variables are used.
        if !self.require_user_variables {
            for user_var in USER_VARIABLES.iter() {
                if user_var.runtime_version > self.runtime {
                    continue;
                }
                for (reference_index, (_reference_type, reference)) in self.references.iter().enumerate() {
                    if self.reference_is_live(reference_index) && reference.usages.first().is_some_and(|(_, usage)| usage.token == user_var.name) {
                        self.require_user_variables = true;
                        break;
                    }
                }
            }
        }
    }

    fn check_arg_types(&mut self, call_parameters: &[ParameterSpecifier], arguments: &[Expression]) {
        for (i, (call_parameter, argument)) in call_parameters.iter().zip(arguments).enumerate() {
            match call_parameter {
                ParameterSpecifier::Function(f) => {
                    let previous = self.allow_routine_reference;
                    self.allow_routine_reference = true;
                    let vt: VariableType = argument.visit(self);
                    self.allow_routine_reference = previous;
                    if vt != VariableType::Function {
                        self.errors
                            .lock()
                            .unwrap()
                            .report_error(argument.get_span().clone(), CompilationErrorType::FunctionExpected);
                    }

                    if vt == VariableType::Function {
                        let container = self.routine_container(self.last_lookup_index);
                        let matches = container.is_some_and(|container| match &container.functions {
                            FunctionDeclaration::Function(declaration) => {
                                f.get_return_type() == declaration.get_return_type() && parameter_lists_match(f.get_parameters(), declaration.get_parameters())
                            }
                            FunctionDeclaration::Procedure(_) => false,
                        });
                        if !matches {
                            self.errors
                                .lock()
                                .unwrap()
                                .report_error(argument.get_span().clone(), CompilationErrorType::ParameterMismatch(argument.to_string()));
                        }
                    }
                }
                ParameterSpecifier::Procedure(p) => {
                    let previous = self.allow_routine_reference;
                    self.allow_routine_reference = true;
                    let vt = argument.visit(self);
                    self.allow_routine_reference = previous;
                    if vt != VariableType::Procedure {
                        self.errors
                            .lock()
                            .unwrap()
                            .report_error(argument.get_span().clone(), CompilationErrorType::ProcedureExpected);
                    }
                    if vt == VariableType::Procedure {
                        let container = self.routine_container(self.last_lookup_index);
                        let matches = container.is_some_and(|container| match &container.functions {
                            FunctionDeclaration::Procedure(declaration) => parameter_lists_match(p.get_parameters(), declaration.get_parameters()),
                            FunctionDeclaration::Function(_) => false,
                        });
                        if !matches {
                            self.errors
                                .lock()
                                .unwrap()
                                .report_error(argument.get_span().clone(), CompilationErrorType::ParameterMismatch(argument.to_string()));
                        }
                    }
                }
                ParameterSpecifier::Variable(parameter) => {
                    let expected = parameter.get_variable_type();
                    let actual = argument.visit(self);
                    self.reject_bare_array_value(argument);
                    if expected != actual && (matches!(expected, VariableType::UserData(_)) || matches!(actual, VariableType::UserData(_))) {
                        self.errors.lock().unwrap().report_error(
                            argument.get_span(),
                            CompilationErrorType::ArgumentTypeMismatch(i + 1, self.source_type_name(expected), self.source_type_name(actual)),
                        );
                    }
                }
            }
        }
    }
}
