use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    ast::{Constant, Expression, FunctionDeclarationAstNode, ParameterSpecifier, ProcedureDeclarationAstNode, Statement, VariableSpecifier},
    compiler::{CompilationErrorType, CompilationWarningType, optimizer::SourceReachability, workspace::Workspace},
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
mod checked;
mod var_aliases;
pub use checked::CheckedProgram;
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
    source_annotations: HashMap<PathBuf, Arc<checked::SourceAnnotations>>,
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
    pub enum_binary_types: HashMap<u64, u8>,
    /// Source LET identifier offset -> assigned element/member/function-result type.
    /// Like the member lookups, snapshot and clear this map between source files.
    pub compound_target_types: HashMap<usize, VariableType>,
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

    /// Appended after semantic checking; source reference IDs must never move.
    /// Emitted immediately after each routine's source locals, before its result.
    lowering_local_variables: HashMap<usize, Vec<usize>>,

    /// Named constants never reach the variable table - the value takes the place of
    /// the name - so they are kept beside it.
    global_constants: HashMap<unicase::Ascii<String>, (VariableType, VariableValue, usize)>,
    local_constants: Option<HashMap<unicase::Ascii<String>, (VariableType, VariableValue, usize)>>,

    /// Where the FOR statements of the current file keep their count, which a
    /// desugared loop compares and steps itself.
    loop_counters: HashSet<usize>,

    // constants
    pub function_containers: Vec<FunctionContainer>,
    /// PPLC pass 2 checks calls against implementation parameters, not DECLARE
    /// hints. Keep the original declaration separately for the count check.
    legacy_call_signatures: HashMap<unicase::Ascii<String>, FunctionDeclaration>,

    cur_func_impl: Option<usize>,
    cur_func_call: u64,
    control_flow_liveness: bool,
    references_are_reachable: bool,
    source_reachability: Option<SourceReachability>,
    source_body_is_reachable: bool,

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
        // Receiver cache keys are source offsets, not package-wide identities.
        self.receiver_types.clear();
        self.current_file = Arc::new(file_name.to_path_buf());
        self.errors.lock().unwrap().set_file_name(file_name);
    }

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

    /// Collect from the qualified, normalized package before visiting any file.
    /// Legacy DECLARE only provides the arity; call checking needs the definition
    /// even when it is in a later file. Strict declarations remain untouched.
    pub fn prepare_legacy_call_signatures(&mut self, asts: &[&crate::ast::Ast]) {
        self.legacy_call_signatures.clear();
        for ast in asts {
            self.collect_legacy_call_signatures(ast);
        }
    }

    fn collect_legacy_call_signatures(&mut self, ast: &crate::ast::Ast) {
        if ast.language_version >= 400 {
            return;
        }
        for node in &ast.nodes {
            let (name, signature) = match node {
                crate::ast::AstNode::Function(function) => (
                    function.get_identifier(),
                    FunctionDeclaration::Function(
                        crate::ast::FunctionDeclarationAstNode::empty(
                            function.get_identifier().clone(),
                            function.get_parameters().clone(),
                            function.get_return_type(),
                        )
                        .with_return_rank(function.get_return_rank()),
                    ),
                ),
                crate::ast::AstNode::Procedure(procedure) => (
                    procedure.get_identifier(),
                    FunctionDeclaration::Procedure(crate::ast::ProcedureDeclarationAstNode::empty(
                        procedure.get_identifier().clone(),
                        procedure.get_parameters().clone(),
                    )),
                ),
                _ => continue,
            };
            self.legacy_call_signatures.insert(name.clone(), signature);
        }
    }

    pub(crate) fn storage_type(&self, source_type: VariableType) -> VariableType {
        source_type
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
            Expression::Parens(value) => self.declared_constant_type(value.get_expression()),
            Expression::Binary(value) if matches!(value.get_op(), crate::ast::BinOp::And | crate::ast::BinOp::Or) => {
                let left = self.declared_constant_type(value.get_left_expression())?;
                (self.type_registry.is_enum_type(left) && Some(left) == self.declared_constant_type(value.get_right_expression())).then_some(left)
            }
            Expression::Identifier(identifier) => self.lookup_constant(identifier.get_identifier()).map(|(variable_type, _, _)| *variable_type),
            Expression::FunctionCall(call) => {
                let Expression::Identifier(name) = call.get_expression() else { return None };
                self.type_registry
                    .get_enum(name.get_identifier())
                    .map(|definition| VariableType::UserData(definition.id))
            }
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
    fn enum_constant_value(&self, expr: &Expression) -> Option<VariableValue> {
        crate::ast::const_enum_value(
            expr,
            &|name| self.lookup_constant(name).map(|(_, value, _)| value.clone()),
            &self.type_registry.enums(),
        )
    }

    fn check_enum_binary_value(&mut self, binary: &crate::ast::BinaryExpression, id: u8) {
        let left = self.enum_constant_value(binary.get_left_expression());
        let right = self.enum_constant_value(binary.get_right_expression());
        if let (Some(left), Some(right)) = (left, right) {
            let value = if binary.get_op() == crate::ast::BinOp::And {
                left.as_int() & right.as_int()
            } else {
                left.as_int() | right.as_int()
            };
            let definition = self.type_registry.get_enum_from_id(id).unwrap();
            if !definition.domain.contains(&value) {
                self.errors.lock().unwrap().report_error(
                    binary.get_op_token().span.clone(),
                    CompilationErrorType::InvalidEnumValue(value, definition.name.to_string()),
                );
            }
        }
    }

    /// Validate constant enum operators without visiting runtime expressions or
    /// adding intermediate constants to the emitted variable table.
    fn check_constant_enum_operations(&mut self, expr: &Expression) -> bool {
        match expr {
            Expression::Parens(value) => self.check_constant_enum_operations(value.get_expression()),
            Expression::FunctionCall(call) => {
                if let Expression::MemberReference(member) = call.get_expression() {
                    self.check_constant_enum_operations(member.get_expression());
                }
                for argument in call.get_arguments() {
                    self.check_constant_enum_operations(argument);
                }
                if let Some(VariableType::UserData(id)) = self.declared_constant_type(expr) {
                    let definition = self.type_registry.get_enum_from_id(id).unwrap();
                    if let Some(value) = self.enum_constant_value(expr) {
                        if !definition.domain.contains(&value.as_int()) {
                            self.errors.lock().unwrap().report_error(
                                expr.get_span(),
                                CompilationErrorType::InvalidEnumValue(value.as_int(), definition.name.to_string()),
                            );
                        }
                    }
                    return true;
                }
                false
            }
            Expression::Unary(value) => {
                if self.check_constant_enum_operations(value.get_expression()) {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(expr.get_span(), CompilationErrorType::InvalidEnumOperation);
                }
                false
            }
            Expression::Binary(value) => {
                let left = self.check_constant_enum_operations(value.get_left_expression());
                let right = self.check_constant_enum_operations(value.get_right_expression());
                if left || right {
                    if !matches!(
                        value.get_op(),
                        crate::ast::BinOp::Eq | crate::ast::BinOp::NotEq | crate::ast::BinOp::And | crate::ast::BinOp::Or
                    ) {
                        self.errors
                            .lock()
                            .unwrap()
                            .report_error(expr.get_span(), CompilationErrorType::InvalidEnumOperation);
                    } else if !left
                        || !right
                        || self.declared_constant_type(value.get_left_expression()) != self.declared_constant_type(value.get_right_expression())
                    {
                        self.errors
                            .lock()
                            .unwrap()
                            .report_error(expr.get_span(), CompilationErrorType::InvalidEnumOperation);
                    } else if matches!(value.get_op(), crate::ast::BinOp::And | crate::ast::BinOp::Or) {
                        let Some(VariableType::UserData(id)) = self.declared_constant_type(value.get_left_expression()) else {
                            unreachable!()
                        };
                        self.check_enum_binary_value(value, id);
                        return true;
                    }
                }
                false
            }
            _ => self.declared_constant_type(expr).is_some_and(|kind| self.type_registry.is_enum_type(kind)),
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
            enum_binary_types: HashMap::new(),
            compound_target_types: HashMap::new(),
            call_graph: CallGraph::default(),
            member_array_returns: HashMap::new(),

            global_lookup: VariableLookups::default(),
            local_variable_lookup: None,
            lowering_local_variables: HashMap::new(),
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
            source_reachability: None,
            source_annotations: HashMap::new(),
            source_body_is_reachable: true,
            function_containers: Vec::new(),
            legacy_call_signatures: HashMap::new(),
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
        self.generate_table(true)
    }

    /// Allocate resolved storage only. The backend interns constants from the
    /// lowered code so folded-away source operands do not inflate the PPE.
    pub(crate) fn generate_storage_table(&mut self) -> LookupVariabeleTable {
        self.generate_table(false)
    }

    fn generate_table(&mut self, include_source_constants: bool) -> LookupVariabeleTable {
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
            for idx in f
                .local_variables
                .clone()
                .chain(self.lowering_local_variables.get(&f.id).into_iter().flatten().copied())
            {
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
                r.variable_table_index = variable_table.len() + 1;
                let mut new_entry = r.create_table_entry_as(storage_type);
                new_entry.entry_type = EntryType::Parameter;
                variable_table.push(new_entry);
            }

            for idx in f
                .local_variables
                .clone()
                .chain(self.lowering_local_variables.get(&f.id).into_iter().flatten().copied())
            {
                let is_live = self.reference_is_live(idx);
                let storage_type = self.storage_type(self.references[idx].1.variable_type);
                let (rt, r) = &mut self.references[idx];
                if !matches!(rt, ReferenceType::Variable(_)) || !is_live {
                    continue;
                }
                r.variable_table_index = variable_table.len() + 1;
                let mut new_entry = r.create_table_entry_as(storage_type);
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

        if include_source_constants {
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

    fn call_signature(&self, container: usize) -> FunctionDeclaration {
        let routine = &self.function_containers[container];
        if self.lang_version < 400
            && let Some(signature) = self.legacy_call_signatures.get(&routine.name)
        {
            return signature.clone();
        }
        routine.functions.clone()
    }

    fn routine_container_index(&self, reference: usize) -> Option<usize> {
        match self.references.get(reference)?.0 {
            ReferenceType::Function(container) | ReferenceType::Procedure(container) => Some(container),
            _ => None,
        }
    }

    /// Own one CFG for the complete, unmoved source body. Nested walks only query it.
    fn visit_source_body(&mut self, statements: &[Statement]) -> bool {
        let previous_graph = self.source_reachability.take();
        let previous_body = self.source_body_is_reachable;
        self.source_body_is_reachable = self.references_are_reachable;
        self.source_reachability = self.control_flow_liveness.then(|| SourceReachability::build(statements));
        let falls_through = self.source_reachability.as_ref().is_none_or(SourceReachability::falls_through);
        self.visit_statement_sequence(statements);
        self.source_reachability = previous_graph;
        self.source_body_is_reachable = previous_body;
        falls_through
    }

    fn visit_source_statement(&mut self, statement: &Statement) {
        let previous = self.references_are_reachable;
        if let Some(reachable) = self.source_reachability.as_ref().and_then(|graph| graph.statement_is_reachable(statement)) {
            // A GOTO may enter a nested body without reaching its parent's entry.
            self.references_are_reachable = self.source_body_is_reachable && reachable;
        }
        statement.visit(self);
        self.references_are_reachable = previous;
    }

    fn visit_statement_sequence(&mut self, statements: &[Statement]) {
        for statement in statements {
            self.visit_source_statement(statement);
        }
    }

    fn with_source_expression<T>(&mut self, expression: &Expression, check: impl FnOnce(&mut Self) -> T) -> T {
        let previous = self.references_are_reachable;
        if let Some(reachable) = self.source_reachability.as_ref().and_then(|graph| graph.expression_is_reachable(expression)) {
            self.references_are_reachable = self.source_body_is_reachable && reachable;
        }
        let result = check(self);
        self.references_are_reachable = previous;
        result
    }

    fn visit_source_expression(&mut self, expression: &Expression) -> VariableType {
        self.with_source_expression(expression, |visitor| expression.visit(visitor))
    }

    /// Register trusted lowering declarations AFTER `finish`, BEFORE table generation.
    /// No semantic visitors, call edges, source usages, or warnings are produced.
    /// `None` denotes main/global storage; `Some` is a qualified routine name.
    pub(crate) fn register_lowering_temporaries(&mut self, mut temporaries: HashMap<Option<unicase::Ascii<String>>, Vec<(VariableType, VariableSpecifier)>>) {
        assert!(self.local_variable_lookup.is_none(), "lowering storage requires completed source analysis");
        if let Some(globals) = temporaries.remove(&None) {
            for (variable_type, variable) in globals {
                self.add_variable(
                    variable_type,
                    variable.get_identifier_token(),
                    variable.get_dimensions().len() as u8,
                    variable.get_vector_size(),
                    variable.get_matrix_size(),
                    variable.get_cube_size(),
                );
                self.reference_owners.entry(self.references.len() - 1).or_default().insert(None);
            }
        }
        // Container order (not HashMap iteration) keeps emitted storage deterministic.
        for container_index in 0..self.function_containers.len() {
            let container = &self.function_containers[container_index];
            if container.parameter_index.is_some() {
                continue;
            }
            let Some(locals) = temporaries.remove(&Some(container.name.clone())) else {
                continue;
            };
            let owner = container.id;
            // Lowering may also have processed bodies which will not be emitted.
            if !self.call_graph.is_reachable(SymbolId(owner)) {
                continue;
            }
            self.local_variable_lookup = Some(std::mem::take(&mut self.function_containers[container_index].lookup));
            for (variable_type, variable) in locals {
                self.add_variable(
                    variable_type,
                    variable.get_identifier_token(),
                    variable.get_dimensions().len() as u8,
                    variable.get_vector_size(),
                    variable.get_matrix_size(),
                    variable.get_cube_size(),
                );
                let id = self.references.len() - 1;
                self.reference_owners.entry(id).or_default().insert(Some(owner));
                self.lowering_local_variables.entry(owner).or_default().push(id);
            }
            self.function_containers[container_index].lookup = self.local_variable_lookup.take().unwrap();
        }
        debug_assert!(temporaries.is_empty(), "lowering temporaries name an unknown routine");
    }

    fn check_enum_signature_runtime(&mut self, parameters: &[ParameterSpecifier], result: VariableType, span: std::ops::Range<usize>) {
        if self.runtime >= 400 {
            return;
        }
        if self.type_registry.is_enum_type(result) {
            self.errors.lock().unwrap().report_error(
                span.clone(),
                CompilationErrorType::BuiltinNeedsRuntime("Closed enum signatures".to_string(), 400),
            );
        }
        for parameter in parameters {
            match parameter {
                ParameterSpecifier::Variable(value) => {
                    if self.type_registry.is_enum_type(value.get_variable_type()) {
                        self.errors.lock().unwrap().report_error(
                            span.clone(),
                            CompilationErrorType::BuiltinNeedsRuntime("Closed enum signatures".to_string(), 400),
                        );
                    }
                }
                ParameterSpecifier::Function(value) => self.check_enum_signature_runtime(value.get_parameters(), value.get_return_type(), span.clone()),
                ParameterSpecifier::Procedure(value) => self.check_enum_signature_runtime(value.get_parameters(), VariableType::None, span.clone()),
            }
        }
    }

    fn reject_enum_argument(&mut self, argument: &Expression) {
        let actual = argument.visit(self);
        if self.type_registry.is_enum_type(actual) {
            self.errors
                .lock()
                .unwrap()
                .report_error(argument.get_span(), CompilationErrorType::InvalidEnumOperation);
        }
    }

    fn add_parameters(&mut self, parameters: &[ParameterSpecifier]) {
        for (i, param) in parameters.iter().enumerate() {
            match param {
                ParameterSpecifier::Variable(param) => {
                    let variable = param.get_variable().as_ref().unwrap();
                    let rank = variable.get_dimensions().len() as u8;
                    let array_parameter = self.lang_version >= 400 && rank > 0;
                    if array_parameter && self.runtime < 400 {
                        self.errors.lock().unwrap().report_error(
                            variable.get_identifier_token().span.clone(),
                            CompilationErrorType::BuiltinNeedsRuntime("Array parameters".to_string(), 400),
                        );
                    }
                    let dynamic = array_parameter && variable.get_dimensions()[0].is_dynamic();
                    let id = self.add_declaration(param.get_variable_type(), variable.get_identifier_token());
                    self.references[id].1.header = Some(VarHeader {
                        id,
                        variable_type: param.get_variable_type(),
                        dim: rank,
                        vector_size: if rank == 0 || dynamic { 0 } else { variable.get_vector_size() },
                        matrix_size: if rank == 0 || dynamic { 0 } else { variable.get_matrix_size() },
                        cube_size: if rank == 0 || dynamic { 0 } else { variable.get_cube_size() },
                        flags: if dynamic {
                            crate::executable::variable_table::VARIABLE_FLAG_DYNAMIC_ARRAY
                        } else {
                            0
                        } | if array_parameter {
                            crate::executable::variable_table::VARIABLE_FLAG_ARRAY_PARAMETER
                        } else {
                            0
                        },
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
                        functions: FunctionDeclaration::Function(
                            FunctionDeclarationAstNode::empty(func.get_identifier().clone(), func.get_parameters().clone(), func.get_return_type())
                                .with_return_rank(func.get_return_rank()),
                        ),
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

    fn is_variable_argument(&mut self, expr: &Expression) -> bool {
        match expr {
            Expression::Identifier(identifier) => {
                self.function_return_value_spans.contains(&identifier.get_identifier_token().span.start)
                    || (self.lookup_constant(identifier.get_identifier()).is_none()
                        && self
                            .lookup_variable(identifier.get_identifier())
                            .is_some_and(|index| matches!(self.references[index].0, ReferenceType::Variable(_))))
            }
            Expression::Indexer(indexer) => self.lookup_variable(indexer.get_identifier()).is_some_and(|index| {
                matches!(self.references[index].0, ReferenceType::Variable(_))
                    && self.references[index]
                        .1
                        .header
                        .as_ref()
                        .is_some_and(|header| header.dim > 0 && header.dim as usize == indexer.get_arguments().len())
            }),
            Expression::Parens(parens) => self.is_variable_argument(parens.get_expression()),
            Expression::MemberReference(member) => {
                // Copy-back requires record storage at every step, not a host getter's snapshot.
                self.user_type_lookup
                    .get(&member.get_identifier_token().span.start)
                    .and_then(|type_id| self.type_registry.record_field_index(*type_id, member.get_identifier()))
                    .is_some()
                    && self.is_variable_argument(member.get_expression())
            }
            Expression::FunctionCall(call) => match self.function_type_lookup.get(&CallId(call.id)) {
                Some(SemanticInfo::VariableReference(_)) => true,
                Some(SemanticInfo::IndexedRecordField(_)) => self.is_variable_argument(call.get_expression()),
                Some(SemanticInfo::ArrayValueAt) => match call.get_expression() {
                    Expression::MemberReference(member) => self.is_variable_argument(member.get_expression()),
                    _ => false,
                },
                _ => false,
            },
            _ => false,
        }
    }

    fn check_var_argument(&mut self, arg_num: usize, expr: &Expression) {
        if self.is_variable_argument(expr) {
            return;
        }

        self.errors
            .lock()
            .unwrap()
            .report_error(expr.get_span().clone(), CompilationErrorType::VariableExpected(arg_num + 1));
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

    fn first_unserializable_record_field(&self, type_id: u8, prefix: &str) -> Option<(String, VariableType)> {
        let definition = self.type_registry.get_user_type_from_id(type_id)?;
        for (name, field) in &definition.fields {
            let path = if prefix.is_empty() { name.to_string() } else { format!("{prefix}.{name}") };
            if field.is_dynamic {
                return Some((path, field.variable_type));
            }
            match field.variable_type {
                VariableType::UserData(_) if self.type_registry.is_enum_type(field.variable_type) => {}
                VariableType::UserData(id) if crate::parser::is_user_declared_type(id) => {
                    // Source fields only refer to already declared records, including dynamic edges.
                    if id >= type_id {
                        return Some((path, field.variable_type));
                    }
                    if let Some(invalid) = self.first_unserializable_record_field(id, &path) {
                        return Some(invalid);
                    }
                }
                _ if field.has_record_io_scalar_type() => {}
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
            functions: FunctionDeclaration::Function(
                FunctionDeclarationAstNode::empty(function.get_identifier().clone(), function.get_parameters().clone(), function.get_return_type())
                    .with_return_rank(function.get_return_rank()),
            ),
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
                                f.get_return_type() == declaration.get_return_type()
                                    && f.get_return_rank() == declaration.get_return_rank()
                                    && parameter_lists_match(f.get_parameters(), declaration.get_parameters())
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
                    let rank = parameter.get_variable().as_ref().map_or(0, |variable| variable.get_dimensions().len() as u8);
                    if self.lang_version >= 400 && rank > 0 {
                        let shape = arrays::ArrayShape {
                            element_type: expected,
                            rank,
                            bounds: [0; 3],
                            resizable: true,
                            field_name: None,
                        };
                        self.check_array_target_assignment(&shape, argument, &argument.get_span());
                        continue;
                    }
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
        self.check_var_aliases(call_parameters, arguments);
    }
}

#[cfg(test)]
mod source_reachability_tests {
    use super::*;
    use crate::{
        ast::{Ast, DimensionSpecifier},
        parser::{Encoding, parse_ast},
    };

    fn name(value: &str) -> unicase::Ascii<String> {
        unicase::Ascii::new(value.to_string())
    }

    fn analyze(source: &str, liveness: bool) -> (Ast, SemanticVisitor) {
        let mut workspace = Workspace::default();
        workspace.set_default_language_version(Some(400));
        workspace.package.runtime = Some(400);
        let registry = UserTypeRegistry::icy_board_registry();
        let errors = Arc::new(Mutex::new(ErrorReporter::default()));
        let ast = parse_ast(PathBuf::from("reachability.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
        {
            let errors = errors.lock().unwrap();
            assert!(
                errors.errors.is_empty(),
                "source must parse:\n{source}\nparser errors:\n{}",
                errors
                    .errors
                    .iter()
                    .map(|error| format!("{:?}: {}", error.span, error.error))
                    .collect::<Vec<_>>()
                    .join("\n")
            );
        }
        let original = ast.nodes.clone();
        let mut visitor = SemanticVisitor::new(&workspace, errors, registry);
        visitor.set_control_flow_liveness(liveness);
        visitor.set_file_name(&ast.file_name);
        ast.visit(&mut visitor);
        visitor.finish();
        assert_eq!(original, ast.nodes, "source nodes and IDs must remain untouched");
        assert!(visitor.source_reachability.is_none());
        assert!(visitor.references_are_reachable);
        (ast, visitor)
    }

    fn reference(visitor: &SemanticVisitor, value: &str) -> usize {
        visitor.global_lookup.variable_lookup[&name(value)]
    }

    #[test]
    fn nested_label_entry_overrides_dead_parent_but_keeps_dead_usages() {
        let (_, visitor) = analyze(
            "INTEGER deadValue, liveValue\nGOTO entered\nIF FALSE THEN\nPRINT deadValue\nDeadCall()\n:entered\nPRINT liveValue\nLiveCall()\nENDIF\nEXIT\nPROCEDURE DeadCall()\nENDPROC\nPROCEDURE LiveCall()\nENDPROC\n",
            true,
        );
        let dead = reference(&visitor, "deadValue");
        assert!(!visitor.reference_is_live(dead));
        assert_eq!(visitor.references[dead].1.usages.len(), 1);
        assert!(visitor.reference_is_live(reference(&visitor, "liveValue")));
        assert!(!visitor.routine_is_reachable(reference(&visitor, "DeadCall")));
        assert!(visitor.routine_is_reachable(reference(&visitor, "LiveCall")));
        let errors = visitor.errors.lock().unwrap();
        assert!(errors.errors.is_empty());
        assert!(!errors.warnings.iter().any(|warning| warning.error.to_string().contains("deadValue")));
    }

    #[test]
    fn separately_executed_control_expressions_gate_calls_not_diagnostics() {
        let (_, visitor) = analyze(
            "IF TRUE THEN\nPRINT 1\nELSEIF (HiddenTest()) THEN\nPRINT unknownInDeadBranch\nENDIF\nREPEAT\nBREAK\nUNTIL TrailingTest()\nSELECT CASE UnusedSelector()\nCASE ELSE\nPRINT 2\nENDSELECT\nEXIT\nFUNCTION HiddenTest() BOOLEAN\nRETURN TRUE\nENDFUNC\nFUNCTION TrailingTest() BOOLEAN\nRETURN TRUE\nENDFUNC\nFUNCTION UnusedSelector() INTEGER\nRETURN 1\nENDFUNC\n",
            true,
        );
        for routine in ["HiddenTest", "TrailingTest", "UnusedSelector"] {
            let index = reference(&visitor, routine);
            assert!(!visitor.routine_is_reachable(index), "{routine}");
            assert_eq!(visitor.references[index].1.usages.len(), 1, "{routine}");
        }
        assert!(
            visitor
                .errors
                .lock()
                .unwrap()
                .errors
                .iter()
                .any(|error| error.error.to_string().contains("unknownInDeadBranch"))
        );
        assert_eq!(visitor.function_type_lookup.len(), 3, "dead calls still need source annotations");
    }

    #[test]
    fn for_test_and_step_can_be_live_without_the_initializer() {
        let (_, visitor) = analyze(
            "INTEGER counter\nGOTO body\nFOR counter = Initial() TO Bound() STEP Stride()\n:body\nPRINT 1\nNEXT\nEXIT\nFUNCTION Initial() INTEGER\nRETURN 1\nENDFUNC\nFUNCTION Bound() INTEGER\nRETURN 2\nENDFUNC\nFUNCTION Stride() INTEGER\nRETURN 1\nENDFUNC\n",
            true,
        );
        assert!(visitor.errors.lock().unwrap().errors.is_empty());
        assert!(!visitor.routine_is_reachable(reference(&visitor, "Initial")));
        assert!(visitor.routine_is_reachable(reference(&visitor, "Bound")));
        assert!(visitor.routine_is_reachable(reference(&visitor, "Stride")));
        assert!(visitor.reference_is_live(reference(&visitor, "counter")));
    }

    #[test]
    fn routine_cfg_is_independent_and_liveness_can_be_disabled() {
        let source = "Outer()\nEXIT\nPROCEDURE Outer()\nIF FALSE THEN\nHidden()\nENDIF\nLOOP\nBREAK\nHidden()\nENDLOOP\nENDPROC\nPROCEDURE Hidden()\nENDPROC\n";
        for liveness in [true, false] {
            let (_, visitor) = analyze(source, liveness);
            assert!(visitor.errors.lock().unwrap().errors.is_empty());
            assert!(visitor.routine_is_reachable(reference(&visitor, "Outer")));
            assert_eq!(!liveness, visitor.routine_is_reachable(reference(&visitor, "Hidden")));
        }
    }

    #[test]
    fn isolated_dead_routine_calls_keep_usages_without_live_edges() {
        for body in [
            "IF FALSE THEN\nHidden()\nENDIF\n",
            "IF ((!TRUE)) THEN\nHidden()\nENDIF\n",
            "LOOP\nBREAK\nHidden()\nENDLOOP\n",
            "REPEAT\nCONTINUE\nHidden()\nUNTIL TRUE\n",
            "WHILE ((TRUE)) DO\nCONTINUE\nENDWHILE\nHidden()\n",
        ] {
            let source = format!("Outer()\nEXIT\nPROCEDURE Outer()\n{body}ENDPROC\nPROCEDURE Hidden()\nENDPROC\n");
            for liveness in [true, false] {
                let (_, visitor) = analyze(&source, liveness);
                assert!(visitor.errors.lock().unwrap().errors.is_empty(), "{body}");
                assert!(visitor.routine_is_reachable(reference(&visitor, "Outer")), "{body}");
                let hidden = reference(&visitor, "Hidden");
                assert_eq!(!liveness, visitor.routine_is_reachable(hidden), "{body}");
                assert_eq!(1, visitor.references[hidden].1.usages.len(), "{body}");
            }
        }
    }

    #[test]
    fn compound_annotations_describe_assigned_elements_and_function_results() {
        let source = "TYPE Item\n STRING text\nENDTYPE\nItem items[1]\nINTEGER values[1]\nitems[0].text += \"x\"\nvalues[0] += 1\nPRINT Result()\nFUNCTION Result() INTEGER\nResult += 1\nENDFUNC\n";
        let (_, visitor) = analyze(source, true);
        assert!(visitor.errors.lock().unwrap().errors.is_empty());
        for (assignment, expected) in [
            ("items[0].text +=", VariableType::UnboundedString),
            ("values[0] +=", VariableType::Integer),
            ("Result +=", VariableType::Integer),
        ] {
            assert_eq!(
                Some(&expected),
                visitor.compound_target_types.get(&source.find(assignment).unwrap()),
                "{assignment}"
            );
        }
    }

    #[test]
    fn lowering_storage_preserves_reference_ids_dimensions_and_contiguous_routines() {
        let (_, mut visitor) = analyze(
            "PRINT First(1), Second(2)\nFUNCTION First(INTEGER arg) INTEGER\nINTEGER local\nlocal = arg\nRETURN local\nENDFUNC\nFUNCTION Second(INTEGER arg) INTEGER\nINTEGER local\nlocal = arg\nRETURN local\nENDFUNC\nFUNCTION Dead() INTEGER\nRETURN 0\nENDFUNC\n",
            true,
        );
        assert!(visitor.errors.lock().unwrap().errors.is_empty());
        let original_references = visitor.references.clone();
        let warning_count = visitor.errors.lock().unwrap().warnings.len();
        let mut dynamic = VariableSpecifier::empty(name("__temp"), Vec::new());
        dynamic.get_dimensions_mut().push(DimensionSpecifier::dynamic());
        visitor.register_lowering_temporaries(HashMap::from([
            (None, vec![(VariableType::Integer, VariableSpecifier::empty(name("__global"), vec![3, 4, 5]))]),
            (Some(name("First")), vec![(VariableType::Integer, dynamic)]),
            (
                Some(name("Second")),
                vec![(VariableType::String, VariableSpecifier::empty(name("__temp"), Vec::new()))],
            ),
            (
                Some(name("Dead")),
                vec![(VariableType::Integer, VariableSpecifier::empty(name("__dead"), Vec::new()))],
            ),
        ]));
        assert_eq!(original_references, visitor.references[..original_references.len()]);
        assert_eq!(warning_count, visitor.errors.lock().unwrap().warnings.len());
        assert_eq!(original_references.len() + 3, visitor.references.len());
        assert!(visitor.references[original_references.len()..].iter().all(|(_, refs)| refs.usages.is_empty()));
        let mut table = visitor.generate_variable_table();
        let global = table.lookup_variable(&name("__global")).unwrap();
        assert_eq!(
            (3, 3, 4, 5),
            (global.header.dim, global.header.vector_size, global.header.matrix_size, global.header.cube_size)
        );
        for (routine, dynamic) in [("First", true), ("Second", false)] {
            let container = visitor.routine_container(reference(&visitor, routine)).unwrap();
            let reference_id = container.lookup.variable_lookup[&name("__temp")];
            assert!(visitor.reference_is_live(reference_id));
            let routine_id = table.lookup_variable_index(&name(routine)).unwrap();
            let routine_value = unsafe { table.lookup_variable(&name(routine)).unwrap().value.data.function_value };
            assert_eq!(3, routine_value.local_variables, "source local, generated local, result");
            assert_eq!(routine_id + 4, routine_value.return_var as usize);
            table.start_compile_function_body(&name(routine));
            let temporary = table.lookup_variable(&name("__temp")).unwrap();
            assert_eq!(routine_id + 3, temporary.header.id);
            assert_eq!(EntryType::LocalVariable, temporary.entry_type);
            assert_eq!(u8::from(dynamic), temporary.header.dim);
            assert_eq!(
                dynamic,
                temporary.header.flags & crate::executable::variable_table::VARIABLE_FLAG_DYNAMIC_ARRAY != 0
            );
            table.end_compile_function_body();
        }
        assert!(!table.has_variable(&name("Dead")));
        assert!(visitor.errors.lock().unwrap().errors.is_empty());
    }
}
