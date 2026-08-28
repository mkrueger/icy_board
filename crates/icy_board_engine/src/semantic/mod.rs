use core::panic;
use std::{
    collections::{HashMap, HashSet},
    path::PathBuf,
    sync::{Arc, Mutex},
};

use crate::{
    ast::{
        AstVisitor, CommentAstNode, ConstDeclarationStatement, Constant, ConstantExpression, EnumDeclarationAstNode, Expression, FunctionCallExpression,
        FunctionDeclarationAstNode, FunctionImplementation, GosubStatement, GotoStatement, IdentifierExpression, LabelStatement, LetStatement, OnErrorMode,
        OnErrorStatement, ParameterSpecifier, PredefinedCallStatement, ProcedureCallStatement, ProcedureDeclarationAstNode, ProcedureImplementation,
        TypeDeclarationAstNode, VariableDeclarationStatement, VariableParameterSpecifier, const_value_with_members, walk_function_implementation,
        walk_indexer_expression, walk_predefined_call_statement, walk_procedure_call_statement, walk_procedure_implementation,
    },
    compiler::{CompilationErrorType, CompilationWarningType, user_data::UserDataMemberRegistry, workspace::Workspace},
    executable::{
        EntryType, FIRST_RECORD_LITERAL_RUNTIME, FIRST_ROUTINE_REFERENCE_RUNTIME, FIRST_STATIC_MEMBER_RUNTIME, FIRST_TYPE_TABLE_RUNTIME, FUNCTION_DEFINITIONS,
        FuncOpCode, FunctionDefinition, FunctionValue, GenericVariableData, OpCode, ProcedureValue, StatementSignature, TableEntry, USER_VARIABLES, VarHeader,
        VariableData, VariableTable, VariableType, VariableValue,
    },
    parser::{
        self, ErrorReporter, FIRST_BOARD_OBJECT_LANGUAGE_VERSION, ParserErrorType, UserTypeRegistry,
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
    MemberSetterCall(usize),
    IndexedRecordField(usize),

    /// A built-in array function written as a member, `a.Len()` for `Len(a)`. The
    /// array is passed as the first argument, plus the constants the member fills in.
    ArrayMemberFunc(FuncOpCode, Vec<i32>),

    /// A scalar string function written with its receiver first.
    StringMemberFunc(FuncOpCode),

    /// A built-in scalar type namespace function, `STRING.Join(...)`.
    StringStaticFunc(FuncOpCode),

    /// `value.Split(...)` or `STRING.Split(value, ...)`, lowered to one statement.
    StringSplitProc {
        static_call: bool,
        default_limit: bool,
    },

    /// `regex.Split(text, target [, limit])`, lowered to an array-writing statement.
    RegexSplitProc {
        default_limit: bool,
    },

    /// The same for a built-in array statement, `a.Redim(10)` for `REDIM a, 10`.
    ArrayMemberProc(OpCode),

    PredefFunctionGroup(Vec<usize>),

    /// id looks up into '`function_containers`'
    FunctionReference(usize),

    /// id looks up into 'references'
    VariableReference(usize),
}

/// A built-in array function that may also be written as a member of the array.
///
/// Every function that takes an array first has an entry here, so `a.Len(0)` and
/// `Len(a, 0)` are the same call and neither can drift from the other.
pub struct ArrayMember {
    pub name: &'static str,
    pub opcode: FuncOpCode,
    /// Arguments the member takes, on top of the array itself.
    pub arguments: std::ops::RangeInclusive<usize>,
    /// What a left out trailing argument stands for.
    pub defaults: &'static [i32],
    pub return_type: VariableType,
}

/// The members every array carries. `Redim` is a statement rather than a function
/// and is resolved next to the other member call statements.
pub const ARRAY_MEMBERS: &[ArrayMember] = &[ArrayMember {
    name: "Len",
    opcode: FuncOpCode::Len_Dim,
    arguments: 0..=1,
    defaults: &[0],
    return_type: VariableType::Integer,
}];

pub fn array_member(name: &unicase::Ascii<String>) -> Option<&'static ArrayMember> {
    ARRAY_MEMBERS.iter().find(|member| *name == member.name)
}

pub struct StringMember {
    pub name: &'static str,
    pub arguments: std::ops::RangeInclusive<usize>,
    pub return_type: VariableType,
    pub is_static: bool,
    pub is_procedure: bool,
}

pub const STRING_MEMBERS: &[StringMember] = &[
    StringMember {
        name: "Len",
        arguments: 0..=0,
        return_type: VariableType::Integer,
        is_static: false,
        is_procedure: false,
    },
    StringMember {
        name: "Find",
        arguments: 1..=2,
        return_type: VariableType::Integer,
        is_static: false,
        is_procedure: false,
    },
    StringMember {
        name: "FindLast",
        arguments: 1..=2,
        return_type: VariableType::Integer,
        is_static: false,
        is_procedure: false,
    },
    StringMember {
        name: "Contains",
        arguments: 1..=1,
        return_type: VariableType::Boolean,
        is_static: false,
        is_procedure: false,
    },
    StringMember {
        name: "StartsWith",
        arguments: 1..=1,
        return_type: VariableType::Boolean,
        is_static: false,
        is_procedure: false,
    },
    StringMember {
        name: "EndsWith",
        arguments: 1..=1,
        return_type: VariableType::Boolean,
        is_static: false,
        is_procedure: false,
    },
    StringMember {
        name: "Count",
        arguments: 1..=1,
        return_type: VariableType::Integer,
        is_static: false,
        is_procedure: false,
    },
    StringMember {
        name: "Replace",
        arguments: 2..=2,
        return_type: VariableType::BigStr,
        is_static: false,
        is_procedure: false,
    },
    StringMember {
        name: "Trim",
        arguments: 0..=1,
        return_type: VariableType::BigStr,
        is_static: false,
        is_procedure: false,
    },
    StringMember {
        name: "TrimStart",
        arguments: 0..=1,
        return_type: VariableType::BigStr,
        is_static: false,
        is_procedure: false,
    },
    StringMember {
        name: "TrimEnd",
        arguments: 0..=1,
        return_type: VariableType::BigStr,
        is_static: false,
        is_procedure: false,
    },
    StringMember {
        name: "ToUpper",
        arguments: 0..=0,
        return_type: VariableType::BigStr,
        is_static: false,
        is_procedure: false,
    },
    StringMember {
        name: "ToLower",
        arguments: 0..=0,
        return_type: VariableType::BigStr,
        is_static: false,
        is_procedure: false,
    },
    StringMember {
        name: "Split",
        arguments: 2..=3,
        return_type: VariableType::None,
        is_static: false,
        is_procedure: true,
    },
    StringMember {
        name: "Join",
        arguments: 2..=2,
        return_type: VariableType::BigStr,
        is_static: true,
        is_procedure: false,
    },
    StringMember {
        name: "Repeat",
        arguments: 2..=2,
        return_type: VariableType::BigStr,
        is_static: true,
        is_procedure: false,
    },
    StringMember {
        name: "Split",
        arguments: 3..=4,
        return_type: VariableType::None,
        is_static: true,
        is_procedure: true,
    },
];

fn string_member(name: &unicase::Ascii<String>, arguments: usize) -> Option<(FuncOpCode, VariableType)> {
    let normalized = name.as_ref().to_ascii_lowercase();
    let (opcode, return_type, expected) = match (normalized.as_str(), arguments) {
        ("len", 0) => (FuncOpCode::LEN, VariableType::Integer, true),
        ("find", 1) => (FuncOpCode::INSTR, VariableType::Integer, true),
        ("find", 2) => (FuncOpCode::StringFindFrom, VariableType::Integer, true),
        ("findlast", 1) => (FuncOpCode::INSTRR, VariableType::Integer, true),
        ("findlast", 2) => (FuncOpCode::StringFindLastFrom, VariableType::Integer, true),
        ("contains", 1) => (FuncOpCode::StringContains, VariableType::Boolean, true),
        ("startswith", 1) => (FuncOpCode::StringStartsWith, VariableType::Boolean, true),
        ("endswith", 1) => (FuncOpCode::StringEndsWith, VariableType::Boolean, true),
        ("count", 1) => (FuncOpCode::StringCount, VariableType::Integer, true),
        ("replace", 2) => (FuncOpCode::REPLACESTR, VariableType::BigStr, true),
        ("trim", 0) => (FuncOpCode::StringTrim, VariableType::BigStr, true),
        ("trim", 1) => (FuncOpCode::StringTrimChars, VariableType::BigStr, true),
        ("trimstart", 0) => (FuncOpCode::StringTrimStart, VariableType::BigStr, true),
        ("trimstart", 1) => (FuncOpCode::StringTrimStartChars, VariableType::BigStr, true),
        ("trimend", 0) => (FuncOpCode::StringTrimEnd, VariableType::BigStr, true),
        ("trimend", 1) => (FuncOpCode::StringTrimEndChars, VariableType::BigStr, true),
        ("toupper", 0) => (FuncOpCode::UPPER, VariableType::BigStr, true),
        ("tolower", 0) => (FuncOpCode::LOWER, VariableType::BigStr, true),
        _ => (FuncOpCode::END, VariableType::None, false),
    };
    expected.then_some((opcode, return_type))
}

fn string_member_type(name: &unicase::Ascii<String>) -> Option<VariableType> {
    STRING_MEMBERS
        .iter()
        .find(|member| !member.is_static && *name == member.name)
        .map(|member| member.return_type)
}

fn string_type_name(expression: &Expression, lang_version: u16) -> bool {
    let Expression::Identifier(identifier) = expression else {
        return false;
    };
    matches!(
        crate::parser::built_in_type(identifier.get_identifier(), lang_version),
        Some(VariableType::String | VariableType::BigStr)
    )
}

/// The built-in array statements that may also be written as a member. `REDIM` is
/// the only one, and it takes one bound per dimension.
pub const ARRAY_PROCEDURES: &[(&str, OpCode, std::ops::RangeInclusive<usize>)] = &[("Redim", OpCode::REDIM, 1..=3)];

pub fn array_procedure(name: &unicase::Ascii<String>) -> Option<&'static (&'static str, OpCode, std::ops::RangeInclusive<usize>)> {
    ARRAY_PROCEDURES.iter().find(|(member, _, _)| *name == *member)
}

/// True where a statement wants the array itself rather than one of its elements,
/// the positions `PCBoard` compiled with `wrVID` instead of `wrVIDSUB`.
fn takes_whole_array(opcode: OpCode, signature: crate::executable::StatementSignature, index: usize) -> bool {
    if opcode == OpCode::REDIM {
        return index == 0;
    }
    if matches!(opcode, OpCode::StringSplit | OpCode::RegexSplit) {
        return index == 2;
    }
    match signature {
        StatementSignature::SpecialCaseDlockg => index == 2,
        StatementSignature::SpecialCaseDcreate => index == 3,
        StatementSignature::SpecialCaseSort => index < 2,
        StatementSignature::Invalid
        | StatementSignature::ArgumentsWithVariable(_, _)
        | StatementSignature::VariableArguments(_, _, _)
        | StatementSignature::SpecialCaseVarSeg
        | StatementSignature::SpecialCasePop => false,
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ArrayShape {
    element_type: VariableType,
    rank: u8,
    bounds: [usize; 3],
    resizable: bool,
    field_name: Option<String>,
}

impl ArrayShape {
    fn source_name(&self) -> String {
        let bounds = self.bounds[..self.rank as usize].iter().map(usize::to_string).collect::<Vec<_>>().join(", ");
        format!("{}({bounds})", self.element_type)
    }

    fn same_layout(&self, other: &Self) -> bool {
        self.element_type == other.element_type && self.rank == other.rank && (self.resizable || self.bounds == other.bounds)
    }
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
                TableEntry::new(decl.token.clone(), header, storage_type.create_empty_value(), EntryType::Variable)
            } else if !self.usages.is_empty() {
                TableEntry::new(
                    self.usages.first().unwrap().1.token.clone(),
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
            if self.string_lookup_table.insert(str.clone()) {
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

/// What an expression in receiver position turned out to be.
enum StaticReceiver {
    /// A board object's type name standing in for its one instance.
    Instance(u8),
    /// A board object's type name standing in for the type itself.
    StaticMember(u8),
    /// Not a type name; the expression has to be visited as a value.
    NotAType,
    /// A type name that cannot stand in for a value. The reason is already reported.
    Rejected,
}

pub struct SemanticVisitor {
    lang_version: u16,
    runtime: u16,
    pub type_registry: UserTypeRegistry,

    pub errors: Arc<Mutex<ErrorReporter>>,
    pub references: Vec<(ReferenceType, References)>,

    /// Maps member references -> user type IDs
    pub user_type_lookup: HashMap<usize, u8>,

    /// Maps a type name used as a receiver -> the builtin that hands its instance back.
    pub instance_provider_lookup: HashMap<usize, FuncOpCode>,

    /// Maps a type name a static member was called on -> that type's id.
    pub static_receiver_lookup: HashMap<usize, u8>,

    pub function_type_lookup: HashMap<u64, SemanticInfo>,
    member_array_returns: HashMap<u64, (VariableType, u8)>,

    pub require_user_variables: bool,
    allow_routine_reference: bool,
    allowed_routine_reference_spans: HashSet<usize>,
    function_return_value_spans: HashSet<usize>,

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

    cur_func_impl: Option<usize>,
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

    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.variable_table.is_empty()
    }

    /// .
    ///
    /// # Panics
    ///
    /// Panics if .
    pub fn lookup_variable_index(&self, identifier: &unicase::Ascii<String>) -> Option<usize> {
        if let Some(local) = &self.local_variable_lookup
            && let Some(c) = self.local_lookups.get(local).unwrap().get(identifier)
        {
            return Some(*c);
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
        log::error!("Constant not found {constant:?}");
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

    pub fn is_function_return_value(&self, span_start: usize) -> bool {
        self.function_return_value_spans.contains(&span_start)
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
            instance_provider_lookup: HashMap::new(),
            static_receiver_lookup: HashMap::new(),
            function_type_lookup: HashMap::new(),
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
            cur_func_impl: None,
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
            }

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
                variable_table.push(TableEntry::new(
                    format!("{} result", f.get_identifier()),
                    header,
                    value,
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
        id
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
            log::error!("Invalid label token {label_token:?}");
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
            log::error!("Invalid label token {label_token:?}");
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

    fn array_shape(&mut self, expression: &Expression) -> Option<ArrayShape> {
        match expression {
            Expression::Identifier(identifier) => {
                let index = self.lookup_variable(identifier.get_identifier())?;
                let reference = &self.references[index].1;
                // A routine reference keeps its parameter count in `dim`, which is not a bound.
                if matches!(reference.variable_type, VariableType::Function | VariableType::Procedure) {
                    return None;
                }
                let header = reference.header.as_ref()?;
                (header.dim > 0).then(|| ArrayShape {
                    element_type: reference.variable_type,
                    rank: header.dim,
                    bounds: [header.vector_size, header.matrix_size, header.cube_size],
                    resizable: true,
                    field_name: None,
                })
            }
            Expression::Parens(parens) => self.array_shape(parens.get_expression()),
            Expression::FunctionCall(call) => {
                if let Some((element_type, rank)) = self.member_array_returns.get(&call.id).copied() {
                    return Some(ArrayShape {
                        element_type,
                        rank,
                        bounds: [0; 3],
                        resizable: true,
                        field_name: None,
                    });
                }
                let SemanticInfo::FunctionReference(index) = self.function_type_lookup.get(&call.id)? else {
                    return None;
                };
                let FunctionDeclaration::Function(function) = &self.function_containers[*index].functions else {
                    return None;
                };
                (function.get_return_rank() > 0).then(|| ArrayShape {
                    element_type: function.get_return_type(),
                    rank: function.get_return_rank(),
                    bounds: [0; 3],
                    resizable: true,
                    field_name: None,
                })
            }
            Expression::MemberReference(member) => {
                let type_id = self.user_type_lookup.get(&member.get_identifier_token().span.start).copied()?;
                let definition = self.type_registry.get_record_type_from_id(type_id)?;
                let field = definition.field(definition.field_index(member.get_identifier())?)?;
                (field.dim > 0).then(|| ArrayShape {
                    element_type: field.variable_type,
                    rank: field.dim,
                    bounds: [field.vector_size as usize, field.matrix_size as usize, field.cube_size as usize],
                    resizable: false,
                    field_name: Some(member.get_identifier().to_string()),
                })
            }
            _ => None,
        }
    }

    fn is_whole_custom_type_array(&mut self, expression: &Expression) -> bool {
        self.array_shape(expression)
            .is_some_and(|shape| matches!(shape.element_type, VariableType::UserData(_)))
    }

    /// A bare array is not a value: `PCBoard` wanted one subscript per dimension
    /// everywhere a variable was read (`wrVIDSUB`), and only the statements that take a
    /// whole array saw one.
    fn reject_bare_array_value(&mut self, expression: &Expression) {
        let Some(shape) = self.array_shape(expression) else {
            return;
        };
        if shape.field_name.is_some() {
            self.errors
                .lock()
                .unwrap()
                .report_error(expression.get_span(), CompilationErrorType::WholeArrayUsedAsScalar);
            return;
        }
        let mut expression = expression;
        while let Expression::Parens(parens) = expression {
            expression = parens.get_expression();
        }
        if let Expression::Identifier(identifier) = expression {
            self.check_arg_count(shape.rank as usize, 0, identifier.get_identifier_token());
        }
    }

    /// Reports what stops `value` from being stored in an array shaped target.
    fn check_array_target_assignment(&mut self, target_shape: &ArrayShape, value: &Expression, span: &core::ops::Range<usize>) {
        let field_name = target_shape.field_name.clone().unwrap_or_default();
        match self.array_shape(value) {
            Some(value_shape) if target_shape.same_layout(&value_shape) => {}
            Some(value_shape) => {
                let expected = target_shape.source_name();
                let actual = value_shape.source_name();
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(span.clone(), CompilationErrorType::RecordArrayShapeMismatch(field_name, expected, actual));
            }
            None => {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(span.clone(), CompilationErrorType::RecordArrayValueExpected(field_name));
            }
        }
    }

    fn is_assignable_explicit_target(&mut self, expression: &Expression) -> bool {
        match expression {
            Expression::Identifier(_) | Expression::Indexer(_) => true,
            Expression::MemberReference(member) => self.is_assignable_explicit_target(member.get_expression()),
            Expression::FunctionCall(call) => {
                if matches!(
                    self.function_type_lookup.get(&call.id),
                    Some(SemanticInfo::IndexedRecordField(_) | SemanticInfo::VariableReference(_))
                ) {
                    return true;
                }
                match call.get_expression() {
                    Expression::Identifier(identifier) => {
                        let Some(index) = self.lookup_variable(identifier.get_identifier()) else {
                            return false;
                        };
                        self.references[index].1.header.as_ref().is_some_and(|header| header.dim > 0)
                    }
                    Expression::MemberReference(member) => {
                        let Some(type_id) = self.user_type_lookup.get(&member.get_identifier_token().span.start).copied() else {
                            return false;
                        };
                        self.type_registry
                            .get_record_type_from_id(type_id)
                            .and_then(|definition| definition.field_index(member.get_identifier()).and_then(|field_id| definition.field(field_id)))
                            .is_some_and(|field| field.dim > 0)
                    }
                    _ => false,
                }
            }
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

        if let Expression::FunctionCall(a) = expr
            && let Some(SemanticInfo::VariableReference(_)) = self.function_type_lookup.get(&a.id)
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

    fn validate_string_split_target(&mut self, expression: &Expression) {
        let valid = self
            .array_shape(expression)
            .is_some_and(|shape| shape.rank == 1 && shape.resizable && matches!(shape.element_type, VariableType::String | VariableType::BigStr));
        if !valid {
            self.errors.lock().unwrap().report_error(
                expression.get_span(),
                CompilationErrorType::ArgumentTypeMismatch(3, "dynamic one-dimensional string array".to_string(), "value".to_string()),
            );
        }
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

    /// Looks a callable member up on a board object.
    fn member_function_signature(&self, user_type: u8, name: &unicase::Ascii<String>) -> Option<(usize, usize, Vec<VariableType>, VariableType, u8)> {
        let registry = self.type_registry.get_type_from_id(user_type)?;
        let function = registry.functions.get(name)?;
        let member_id = registry.get_member_id(name)?;
        Some((member_id, function.required, function.parameters.clone(), function.return_type, function.return_rank))
    }

    fn check_member_arg_types(&mut self, expected: &[VariableType], arguments: &[Expression]) {
        for (index, (argument, expected)) in arguments.iter().zip(expected).enumerate() {
            let actual = argument.visit(self);
            self.reject_bare_array_value(argument);
            if *expected != actual && (matches!(expected, VariableType::UserData(_)) || matches!(actual, VariableType::UserData(_))) {
                self.errors.lock().unwrap().report_error(
                    argument.get_span(),
                    CompilationErrorType::ArgumentTypeMismatch(index + 1, self.source_type_name(*expected), self.source_type_name(actual)),
                );
            }
        }
        for argument in arguments.iter().skip(expected.len()) {
            argument.visit(self);
        }
    }

    /// A board object's type name standing in for something with members: the type's own
    /// static members, or its one instance. A variable of the same name shadows the type.
    fn static_receiver(&mut self, expr: &Expression, member: &unicase::Ascii<String>) -> StaticReceiver {
        let Expression::Identifier(base) = expr else {
            return StaticReceiver::NotAType;
        };
        let identifier = base.get_identifier();
        if self.lookup_variable(identifier).is_some() {
            return StaticReceiver::NotAType;
        }
        let Some(VariableType::UserData(type_id)) = self.type_registry.get_board_object(identifier) else {
            return StaticReceiver::NotAType;
        };
        let span = base.get_identifier_token().span.clone();

        if self.lang_version < FIRST_BOARD_OBJECT_LANGUAGE_VERSION {
            self.errors
                .lock()
                .unwrap()
                .report_error(span, CompilationErrorType::TypeUsedAsValue(identifier.to_string()));
            return StaticReceiver::Rejected;
        }

        // A static member belongs to the type, so it needs no instance behind it.
        if self
            .type_registry
            .get_type_from_id(type_id)
            .is_some_and(|registry| registry.statics.contains(member))
        {
            if self.runtime < FIRST_STATIC_MEMBER_RUNTIME {
                self.errors.lock().unwrap().report_error(
                    span,
                    CompilationErrorType::BuiltinNeedsRuntime(format!("{identifier}.{member}"), FIRST_STATIC_MEMBER_RUNTIME),
                );
                return StaticReceiver::Rejected;
            }
            self.add_constant(&Constant::Integer(i32::from(type_id), crate::ast::constant::NumberFormat::Default));
            self.static_receiver_lookup.insert(span.start, type_id);
            return StaticReceiver::StaticMember(type_id);
        }

        let provider = self.type_registry.get_type_from_id(type_id).and_then(|registry| registry.instance_provider);
        let Some(provider) = provider else {
            self.errors
                .lock()
                .unwrap()
                .report_error(span, CompilationErrorType::TypeUsedAsValue(identifier.to_string()));
            return StaticReceiver::Rejected;
        };
        let minimum_runtime = provider.minimum_runtime();
        if self.runtime < minimum_runtime {
            self.errors.lock().unwrap().report_error(
                span.clone(),
                CompilationErrorType::BuiltinNeedsRuntime(provider.get_definition().name.to_string(), minimum_runtime),
            );
            return StaticReceiver::Rejected;
        }
        self.instance_provider_lookup.insert(span.start, provider);
        StaticReceiver::Instance(type_id)
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
        for (rt, r) in &mut self.references.iter() {
            if matches!(rt, ReferenceType::Label(_)) {
                if r.declaration.is_none() {
                    if let Some((file, span)) = r.usages.first() {
                        self.errors
                            .lock()
                            .unwrap()
                            .report_error_file(file.clone(), span.span.clone(), CompilationErrorType::LabelNotFound(span.token.clone()));
                    }
                } else if r.usages.is_empty()
                    && let Some((file_name, declaration)) = &r.declaration
                {
                    if ":~BEGIN~" == declaration.token || declaration.token.starts_with(":*(") {
                        continue;
                    }
                    self.errors.lock().unwrap().report_warning_file(
                        file_name.clone(),
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
                        file.clone(),
                        decl.span.clone(),
                        CompilationErrorType::MissingImplementation(decl.token.clone()),
                    );
                }
                if r.usages.is_empty() {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_warning_file(file.clone(), decl.span.clone(), CompilationErrorType::UnusedFunction(decl.token.clone()));
                }
            } else if matches!(rt, ReferenceType::Variable(_)) && r.usages.is_empty() {
                self.errors
                    .lock()
                    .unwrap()
                    .report_warning_file(file.clone(), decl.span.clone(), CompilationErrorType::UnusedVariable(decl.token.clone()));
            }
        }

        // search if any user variables are used.
        if !self.require_user_variables {
            for user_var in USER_VARIABLES.iter() {
                if user_var.runtime_version > self.runtime {
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
                            FunctionDeclaration::Procedure(_) => false,
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
                            FunctionDeclaration::Function(_) => false,
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
                    self.reject_bare_array_value(&arguments[i]);
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
    fn visit_unary_expression(&mut self, unary: &crate::ast::UnaryExpression) -> VariableType {
        let result = unary.get_expression().visit(self);
        self.reject_bare_array_value(unary.get_expression());
        result
    }

    fn visit_if_statement(&mut self, if_stmt: &crate::ast::IfStatement) -> VariableType {
        crate::ast::walk_if_stmt(self, if_stmt);
        self.reject_bare_array_value(if_stmt.get_condition());
        VariableType::None
    }

    fn visit_if_then_statement(&mut self, if_then: &crate::ast::IfThenStatement) -> VariableType {
        crate::ast::walk_if_then_stmt(self, if_then);
        self.reject_bare_array_value(if_then.get_condition());
        VariableType::None
    }

    fn visit_while_statement(&mut self, while_stmt: &crate::ast::WhileStatement) -> VariableType {
        crate::ast::walk_while_stmt(self, while_stmt);
        self.reject_bare_array_value(while_stmt.get_condition());
        VariableType::None
    }

    fn visit_while_do_statement(&mut self, while_do: &crate::ast::WhileDoStatement) -> VariableType {
        crate::ast::walk_while_do_stmt(self, while_do);
        self.reject_bare_array_value(while_do.get_condition());
        VariableType::None
    }

    fn visit_repeat_until_statement(&mut self, repeat_until: &crate::ast::RepeatUntilStatement) -> VariableType {
        crate::ast::walk_repeat_until_stmt(self, repeat_until);
        self.reject_bare_array_value(repeat_until.get_condition());
        VariableType::None
    }

    fn visit_select_statement(&mut self, select_stmt: &crate::ast::SelectStatement) -> VariableType {
        crate::ast::walk_select_stmt(self, select_stmt);
        self.reject_bare_array_value(select_stmt.get_expression());
        VariableType::None
    }

    fn visit_return_statement(&mut self, return_stmt: &crate::ast::ReturnStatement) -> VariableType {
        crate::ast::walk_return_stmt(self, return_stmt);
        if let Some(expression) = return_stmt.get_expression() {
            self.reject_bare_array_value(expression);
        }
        VariableType::None
    }

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
            let expected_field = definition.field(index);
            let expected = expected_field.map_or(VariableType::None, |field| field.variable_type);
            let actual = field.get_value().visit(self);
            let value_shape = self.array_shape(field.get_value());
            if let Some(expected_field) = expected_field
                && expected_field.dim > 0
            {
                let target_shape = ArrayShape {
                    element_type: expected_field.variable_type,
                    rank: expected_field.dim,
                    bounds: [
                        expected_field.vector_size as usize,
                        expected_field.matrix_size as usize,
                        expected_field.cube_size as usize,
                    ],
                    resizable: false,
                    field_name: Some(name.to_string()),
                };
                self.check_array_target_assignment(&target_shape, field.get_value(), &field.get_value().get_span());
                continue;
            }
            if value_shape.is_some() {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(field.get_value().get_span(), CompilationErrorType::WholeArrayUsedAsScalar);
                continue;
            }
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
        let left_array = self.array_shape(binary.get_left_expression());
        let right_array = self.array_shape(binary.get_right_expression());
        if left_array.is_some() || right_array.is_some() {
            let compares_record_arrays = matches!(binary.get_op(), crate::ast::BinOp::Eq | crate::ast::BinOp::NotEq)
                && left_array
                    .iter()
                    .chain(right_array.iter())
                    .any(|shape| matches!(shape.element_type, VariableType::UserData(_)));
            if compares_record_arrays {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(binary.get_op_token().span.clone(), CompilationErrorType::CustomTypeArrayComparisonNotSupported);
                return VariableType::None;
            }
            self.reject_bare_array_value(binary.get_left_expression());
            self.reject_bare_array_value(binary.get_right_expression());
            return VariableType::None;
        }
        let has_enum = self.type_registry.is_enum_type(left) || self.type_registry.is_enum_type(right);
        if left == VariableType::UserData(crate::parser::REGEX_OPTIONS_ENUM_ID)
            && right == left
            && matches!(binary.get_op(), crate::ast::BinOp::And | crate::ast::BinOp::Or)
        {
            return left;
        }
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
        if !predef.is_empty() && (self.cur_func_call > 0 || self.lookup_variable(identifier.get_identifier()).is_none()) {
            let def = predef
                .iter()
                .map(|index| &FUNCTION_DEFINITIONS[*index])
                .filter(|definition| definition.version <= self.lang_version)
                .max_by_key(|definition| definition.version)
                .unwrap_or(&FUNCTION_DEFINITIONS[predef[0]]);
            if self.cur_func_call > 0 {
                self.function_type_lookup.insert(self.cur_func_call, SemanticInfo::PredefFunctionGroup(predef));
            } else {
                self.errors.lock().unwrap().report_error(
                    identifier.get_identifier_token().span.clone(),
                    CompilationErrorType::FunctionUsedAsVariable(identifier.get_identifier().to_string()),
                );
            }
            def.return_type
        } else if let Some(idx) = self.lookup_variable(identifier.get_identifier()) {
            if self.cur_func_call == 0
                && self.cur_func_impl == Some(idx)
                && let ReferenceType::Function(container_idx) = self.references[idx].0
                && let FunctionDeclaration::Function(function) = &self.function_containers[container_idx].functions
            {
                self.function_return_value_spans.insert(identifier.get_identifier_token().span.start);
                return function.get_return_type();
            }
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
        if let Expression::Identifier(base) = member_reference_expression.get_expression()
            && let Some(definition) = self.type_registry.get_enum(base.get_identifier())
        {
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
        if let Expression::Identifier(base) = member_reference_expression.get_expression()
            && self.lookup_variable(base.get_identifier()).is_none()
            && matches!(
                crate::parser::built_in_type(base.get_identifier(), self.lang_version),
                Some(VariableType::String | VariableType::BigStr)
            )
        {
            if self.lang_version < 400 {
                self.errors.lock().unwrap().report_error(
                    member_reference_expression.get_identifier_token().span.clone(),
                    CompilationErrorType::BuiltinNeedsRuntime(format!("STRING.{}", member_reference_expression.get_identifier()), 400),
                );
                return VariableType::None;
            }
            return match member_reference_expression.get_identifier().as_ref().to_ascii_lowercase().as_str() {
                "join" | "repeat" => VariableType::BigStr,
                "split" => VariableType::None,
                _ => {
                    self.errors.lock().unwrap().report_error(
                        member_reference_expression.get_identifier_token().span.clone(),
                        CompilationErrorType::InvalidMemberReferenceExpression,
                    );
                    VariableType::None
                }
            };
        }
        let receiver = self.static_receiver(member_reference_expression.get_expression(), member_reference_expression.get_identifier());
        let called_on_the_type = matches!(receiver, StaticReceiver::StaticMember(_));

        // An array carries the built-in array functions as members. Its type is the
        // element's, so the declaration is what says it has them.
        if matches!(receiver, StaticReceiver::NotAType)
            && self.array_shape(member_reference_expression.get_expression()).is_some()
            && (array_member(member_reference_expression.get_identifier()).is_some() || array_procedure(member_reference_expression.get_identifier()).is_some())
        {
            member_reference_expression.get_expression().visit(self);
            return array_member(member_reference_expression.get_identifier()).map_or(VariableType::None, |member| member.return_type);
        }

        let t = match receiver {
            StaticReceiver::Instance(type_id) | StaticReceiver::StaticMember(type_id) => VariableType::UserData(type_id),
            StaticReceiver::NotAType => member_reference_expression.get_expression().visit(self),
            StaticReceiver::Rejected => return VariableType::None,
        };
        if matches!(t, VariableType::String | VariableType::BigStr)
            && let Some(return_type) = string_member_type(member_reference_expression.get_identifier())
        {
            if self.lang_version < 400 {
                self.errors.lock().unwrap().report_error(
                    member_reference_expression.get_identifier_token().span.clone(),
                    CompilationErrorType::BuiltinNeedsRuntime(format!("STRING.{}", member_reference_expression.get_identifier()), 400),
                );
                return VariableType::None;
            }
            return return_type;
        }
        if let VariableType::UserData(d) = t {
            if self.type_registry.is_record_type(d) {
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
                for (name, function) in &t.functions {
                    if name == member_reference_expression.get_identifier() {
                        if t.statics.contains(name) && !called_on_the_type {
                            self.errors.lock().unwrap().report_error(
                                member_reference_expression.get_identifier_token().span.clone(),
                                CompilationErrorType::StaticMemberOnValue(name.to_string()),
                            );
                            return VariableType::None;
                        }
                        self.user_type_lookup.insert(member_reference_expression.get_identifier_token().span.start, d);
                        return function.return_type;
                    }
                }
                for name in t.procedures.keys() {
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
            Constant::String(_) => VariableType::String,
            Constant::Boolean(_) => VariableType::Boolean,
            Constant::Money(_) => VariableType::Money,
            Constant::Unsigned(_, _) => VariableType::Unsigned,
            Constant::Double(_) => VariableType::Double,
            Constant::Integer(_, _) | Constant::Builtin(_) => VariableType::Integer,
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
        if def.opcode == OpCode::REDIM && !call_stmt.get_arguments().is_empty() {
            call_stmt.get_arguments()[0].visit(self);
        }
        if def.opcode == OpCode::REDIM
            && let Some(shape) = call_stmt.get_arguments().first().and_then(|argument| self.array_shape(argument))
            && !shape.resizable
        {
            for argument in call_stmt.get_arguments().iter().skip(1) {
                argument.visit(self);
            }
            self.errors.lock().unwrap().report_error(
                call_stmt.get_arguments()[0].get_span(),
                CompilationErrorType::FixedRecordArrayCannotBeRedimmed(shape.field_name.unwrap_or_default()),
            );
            self.add_reference(
                ReferenceType::PredefinedProc(def.opcode),
                VariableType::Procedure,
                call_stmt.get_identifier_token(),
            );
            return VariableType::None;
        }
        if def.opcode != OpCode::REDIM {
            walk_predefined_call_statement(self, call_stmt);
        } else {
            for argument in call_stmt.get_arguments().iter().skip(1) {
                argument.visit(self);
            }
        }
        for (index, argument) in call_stmt.get_arguments().iter().enumerate() {
            if !takes_whole_array(def.opcode, def.sig, index) {
                self.reject_bare_array_value(argument);
            }
        }

        let minimum_runtime = def.opcode.minimum_runtime();
        if self.runtime < minimum_runtime {
            self.errors.lock().unwrap().report_error(
                call_stmt.get_identifier_token().span.clone(),
                CompilationErrorType::BuiltinNeedsRuntime(def.name.to_string(), minimum_runtime),
            );
        }

        match def.sig {
            crate::executable::StatementSignature::Invalid => panic!("Invalid signature"),
            crate::executable::StatementSignature::ArgumentsWithVariable(v, arg_count) => {
                self.check_arg_count(arg_count, call_stmt.get_arguments().len(), call_stmt.get_identifier_token());
                if v > 0
                    && let Some(arg) = call_stmt.get_arguments().get(v - 1)
                {
                    self.check_argument_is_variable(v - 1, arg);
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
                            if let Some(header) = &r.header
                                && header.dim != 1
                            {
                                self.errors.lock().unwrap().report_error(
                                    a.get_identifier_token().span.clone(),
                                    CompilationErrorType::SortArgumentDimensionError(header.dim),
                                );
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

        if matches!(def.opcode, OpCode::FGetRec | OpCode::FPutRec | OpCode::FReadRec | OpCode::FWriteRec)
            && let Some(record) = call_stmt.get_arguments().get(1)
        {
            let actual = self.resolved_record_io_type(record);
            let VariableType::UserData(type_id) = actual else {
                self.errors.lock().unwrap().report_error(
                    record.get_span(),
                    CompilationErrorType::ArgumentTypeMismatch(2, "user-defined record".to_string(), self.source_type_name(actual)),
                );
                return VariableType::None;
            };
            if !crate::parser::is_user_declared_type(type_id) {
                self.errors.lock().unwrap().report_error(
                    record.get_span(),
                    CompilationErrorType::ArgumentTypeMismatch(2, "user-defined record".to_string(), self.source_type_name(actual)),
                );
            } else if let Some((path, field_type)) = self.first_unserializable_record_field(type_id, "") {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(record.get_span(), CompilationErrorType::RecordIoFieldNotSerializable(path, field_type));
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
        if let Expression::MemberReference(member) = call.get_expression()
            && let Some((_, opcode, arguments)) = array_procedure(member.get_identifier())
        {
            member.get_expression().visit(self);
            if let Some(shape) = self.array_shape(member.get_expression()) {
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                if !shape.resizable {
                    self.errors.lock().unwrap().report_error(
                        member.get_expression().get_span(),
                        CompilationErrorType::FixedRecordArrayCannotBeRedimmed(shape.field_name.unwrap_or_default()),
                    );
                    return VariableType::None;
                }
                let given = call.get_arguments().len();
                if !arguments.contains(&given) {
                    self.check_expr_arg_range(*arguments.start(), *arguments.end(), given, call.get_expression());
                    return VariableType::None;
                }
                self.function_type_lookup.insert(call.id, SemanticInfo::ArrayMemberProc(*opcode));
                return VariableType::None;
            }
        }
        if let Expression::MemberReference(member) = call.get_expression()
            && let Some(array_member) = array_member(member.get_identifier())
        {
            member.get_expression().visit(self);
            if self.array_shape(member.get_expression()).is_some() {
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                let given = call.get_arguments().len();
                if !array_member.arguments.contains(&given) {
                    self.check_expr_arg_range(*array_member.arguments.start(), *array_member.arguments.end(), given, call.get_expression());
                    return VariableType::None;
                }
                let filled_in = array_member.defaults[given.min(array_member.defaults.len())..].to_vec();
                for value in &filled_in {
                    self.add_constant(&Constant::Integer(*value, crate::ast::constant::NumberFormat::Default));
                }
                self.function_type_lookup
                    .insert(call.id, SemanticInfo::ArrayMemberFunc(array_member.opcode, filled_in));
                return array_member.return_type;
            }
        }
        let outer_func_call = self.cur_func_call;
        self.cur_func_call = call.id;
        call.get_expression().visit(self);
        self.cur_func_call = outer_func_call;

        // A member call is decided by the receiver's type, whatever expression produced it.
        if let Expression::MemberReference(member) = call.get_expression() {
            if string_type_name(member.get_expression(), self.lang_version)
                && let Expression::Identifier(base) = member.get_expression()
                && self.lookup_variable(base.get_identifier()).is_none()
            {
                match member.get_identifier().as_ref().to_ascii_lowercase().as_str() {
                    "join" if call.get_arguments().len() == 2 => {
                        for argument in call.get_arguments() {
                            argument.visit(self);
                        }
                        let valid_array = call
                            .get_arguments()
                            .first()
                            .and_then(|argument| self.array_shape(argument))
                            .is_some_and(|shape| shape.rank == 1 && matches!(shape.element_type, VariableType::String | VariableType::BigStr));
                        if !valid_array {
                            self.errors.lock().unwrap().report_error(
                                call.get_arguments()[0].get_span(),
                                CompilationErrorType::ArgumentTypeMismatch(1, "one-dimensional string array".to_string(), "value".to_string()),
                            );
                        }
                        self.function_type_lookup
                            .insert(call.id, SemanticInfo::StringStaticFunc(FuncOpCode::StringJoin));
                        return VariableType::BigStr;
                    }
                    "repeat" if call.get_arguments().len() == 2 => {
                        for argument in call.get_arguments() {
                            argument.visit(self);
                        }
                        self.function_type_lookup
                            .insert(call.id, SemanticInfo::StringStaticFunc(FuncOpCode::StringRepeat));
                        return VariableType::BigStr;
                    }
                    "split" if (3..=4).contains(&call.get_arguments().len()) => {
                        for argument in call.get_arguments() {
                            argument.visit(self);
                        }
                        self.validate_string_split_target(&call.get_arguments()[2]);
                        if call.get_arguments().len() == 3 {
                            self.add_constant(&Constant::Integer(0, crate::ast::constant::NumberFormat::Default));
                        }
                        self.function_type_lookup.insert(
                            call.id,
                            SemanticInfo::StringSplitProc {
                                static_call: true,
                                default_limit: call.get_arguments().len() == 3,
                            },
                        );
                        return VariableType::None;
                    }
                    _ => {}
                }
            }

            let registered_type_receiver = matches!(
                member.get_expression(),
                Expression::Identifier(identifier) if self.type_registry.get_board_object(identifier.get_identifier()).is_some()
            );
            let receiver_type = if registered_type_receiver {
                VariableType::None
            } else {
                member.get_expression().visit(self)
            };
            if matches!(receiver_type, VariableType::String | VariableType::BigStr)
                && *member.get_identifier() == "Split"
                && (2..=3).contains(&call.get_arguments().len())
            {
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                self.validate_string_split_target(&call.get_arguments()[1]);
                if call.get_arguments().len() == 2 {
                    self.add_constant(&Constant::Integer(0, crate::ast::constant::NumberFormat::Default));
                }
                self.function_type_lookup.insert(
                    call.id,
                    SemanticInfo::StringSplitProc {
                        static_call: false,
                        default_limit: call.get_arguments().len() == 2,
                    },
                );
                return VariableType::None;
            }
            if receiver_type == VariableType::UserData(crate::parser::REGEX_ID as u8)
                && *member.get_identifier() == "Split"
                && (2..=3).contains(&call.get_arguments().len())
            {
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                self.validate_string_split_target(&call.get_arguments()[1]);
                if call.get_arguments().len() == 2 {
                    self.add_constant(&Constant::Integer(0, crate::ast::constant::NumberFormat::Default));
                }
                self.function_type_lookup.insert(
                    call.id,
                    SemanticInfo::RegexSplitProc {
                        default_limit: call.get_arguments().len() == 2,
                    },
                );
                return VariableType::None;
            }
            if matches!(receiver_type, VariableType::String | VariableType::BigStr)
                && let Some((opcode, return_type)) = string_member(member.get_identifier(), call.get_arguments().len())
            {
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                if self.runtime < opcode.minimum_runtime() {
                    self.errors.lock().unwrap().report_error(
                        member.get_identifier_token().span.clone(),
                        CompilationErrorType::BuiltinNeedsRuntime(format!("STRING.{}", member.get_identifier()), opcode.minimum_runtime()),
                    );
                }
                self.function_type_lookup.insert(call.id, SemanticInfo::StringMemberFunc(opcode));
                return return_type;
            }

            // An array's members are the built-in array functions written the other way round.
            if self.array_shape(member.get_expression()).is_some()
                && let Some(array_member) = array_member(member.get_identifier())
            {
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                let given = call.get_arguments().len();
                if !array_member.arguments.contains(&given) {
                    self.check_expr_arg_range(*array_member.arguments.start(), *array_member.arguments.end(), given, call.get_expression());
                    return VariableType::None;
                }
                let filled_in = array_member.defaults[given.min(array_member.defaults.len())..].to_vec();
                for value in &filled_in {
                    self.add_constant(&Constant::Integer(*value, crate::ast::constant::NumberFormat::Default));
                }
                self.function_type_lookup
                    .insert(call.id, SemanticInfo::ArrayMemberFunc(array_member.opcode, filled_in));
                return array_member.return_type;
            }

            if self.array_shape(member.get_expression()).is_some()
                && let Some((_, opcode, arguments)) = array_procedure(member.get_identifier())
            {
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                let given = call.get_arguments().len();
                if !arguments.contains(&given) {
                    self.check_expr_arg_range(*arguments.start(), *arguments.end(), given, call.get_expression());
                    return VariableType::None;
                }
                self.function_type_lookup.insert(call.id, SemanticInfo::ArrayMemberProc(*opcode));
                return VariableType::None;
            }

            if let Some(type_id) = self.user_type_lookup.get(&member.get_identifier_token().span.start).copied()
                && self.type_registry.is_record_type(type_id)
                && let Some(member_id) = self.type_registry.record_field_index(type_id, member.get_identifier())
                && let Some(field) = self
                    .type_registry
                    .get_record_type_from_id(type_id)
                    .and_then(|definition| definition.field(member_id))
                && field.dim > 0
            {
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                if field.dim as usize != call.get_arguments().len() {
                    self.errors.lock().unwrap().report_error(
                        call.get_expression().get_span(),
                        CompilationErrorType::RecordArrayIndexCount(
                            member.get_identifier().to_string(),
                            field.dim,
                            call.get_arguments().len(),
                            if call.get_arguments().len() == 1 { "index was" } else { "indices were" },
                        ),
                    );
                }
                self.function_type_lookup.insert(call.id, SemanticInfo::IndexedRecordField(member_id));
                return field.variable_type;
            }

            let Some(user_type) = self.user_type_lookup.get(&member.get_identifier_token().span.start).copied() else {
                // Visiting the member reference already reported why it could not be resolved.
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                return VariableType::None;
            };
            if !member.get_identifier().starts_with('<')
                && matches!(
                    call.get_lpar_token().token,
                    Token::Eq
                        | Token::AddAssign
                        | Token::SubAssign
                        | Token::MulAssign
                        | Token::DivAssign
                        | Token::ModAssign
                        | Token::AndAssign
                        | Token::OrAssign
                )
            {
                let Some(registry) = self.type_registry.get_type_from_id(user_type) else {
                    for argument in call.get_arguments() {
                        argument.visit(self);
                    }
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(member.get_identifier_token().span.clone(), CompilationErrorType::InvalidLetVariable);
                    return VariableType::None;
                };
                let Some(member_id) = registry.get_member_id(member.get_identifier()) else {
                    return VariableType::None;
                };
                if !matches!(registry.id_table.get(member_id), Some(crate::compiler::user_data::UserDataEntry::Field(_))) {
                    for argument in call.get_arguments() {
                        argument.visit(self);
                    }
                    self.errors.lock().unwrap().report_error(
                        member.get_identifier_token().span.clone(),
                        CompilationErrorType::MemberIsReadOnly(member.get_identifier().to_string()),
                    );
                    return VariableType::None;
                }
                let expected = registry.fields.get(member.get_identifier()).copied().unwrap_or(VariableType::None);
                self.check_member_arg_types(&[expected], call.get_arguments());
                self.function_type_lookup.insert(call.id, SemanticInfo::MemberSetterCall(member_id));
                return VariableType::None;
            }
            // A record field indexed like `rec.field(1)` is not a member call; the variable path below takes it.
            if self.type_registry.get_type_from_id(user_type).is_some() {
                if let Some((member_id, required, parameters, return_type, return_rank)) = self.member_function_signature(user_type, member.get_identifier()) {
                    self.check_expr_arg_range(required, parameters.len(), call.get_arguments().len(), call.get_expression());
                    self.check_member_arg_types(&parameters, call.get_arguments());
                    self.function_type_lookup.insert(call.id, SemanticInfo::MemberFunctionCall(member_id));
                    if return_rank > 0 {
                        self.member_array_returns.insert(call.id, (return_type, return_rank));
                    }
                    return return_type;
                }
                for argument in call.get_arguments() {
                    argument.visit(self);
                }
                self.errors.lock().unwrap().report_error(
                    member.get_identifier_token().span.clone(),
                    CompilationErrorType::FunctionNotFound(member.get_identifier().to_string()),
                );
                return VariableType::None;
            }
        }

        match self.function_type_lookup.get(&call.id).cloned() {
            Some(SemanticInfo::FunctionReference(idx)) => {
                let declaration = self.function_containers[idx].functions.clone();
                let arg_count = if let FunctionDeclaration::Function(f) = &declaration {
                    res = f.get_return_type();
                    self.check_arg_types(f.get_parameters(), call.get_arguments());
                    f.get_parameters().len()
                } else {
                    self.errors.lock().unwrap().report_error(
                        call.get_expression().get_span(),
                        CompilationErrorType::FunctionNotFound(call.get_expression().to_string()),
                    );
                    0
                };
                self.check_expr_arg_count(arg_count, call.get_arguments().len(), call.get_expression());
            }
            Some(SemanticInfo::VariableReference(idx)) => {
                for argument in call.get_arguments() {
                    argument.visit(self);
                    self.reject_bare_array_value(argument);
                }

                let (rt, r) = &mut self.references[idx];

                if self.lang_version >= 400
                    && r.header.as_ref().is_some_and(|header| header.dim > 0)
                    && call.get_lpar_token().token == Token::LPar
                {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_warning(call.get_lpar_token().span.clone(), CompilationWarningType::ArrayBracketsRequired);
                }

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
                let mut funcs = funcs;
                funcs.sort_by_key(|func| std::cmp::Reverse(FUNCTION_DEFINITIONS[*func].version));
                for func in &funcs {
                    let def = &FUNCTION_DEFINITIONS[*func];
                    if def.parameter_count() == call.get_arguments().len() && def.version <= self.lang_version {
                        let minimum_runtime = def.opcode.minimum_runtime();
                        if self.runtime < minimum_runtime {
                            self.errors.lock().unwrap().report_error(
                                call.get_expression().get_span(),
                                CompilationErrorType::BuiltinNeedsRuntime(def.name.to_string(), minimum_runtime),
                            );
                            return res;
                        }
                        self.function_type_lookup.insert(call.id, SemanticInfo::PredefinedFunc(def.opcode));
                        if !matches!(def.opcode, FuncOpCode::Len_Dim | FuncOpCode::ElementCount | FuncOpCode::ElementAt) {
                            for argument in call.get_arguments() {
                                self.reject_bare_array_value(argument);
                            }
                        }
                        if let Expression::Identifier(id) = call.get_expression() {
                            self.add_reference(ReferenceType::PredefinedFunc(def.opcode), VariableType::Function, id.get_identifier_token());
                        }
                        return def.return_type;
                    }
                }
                if let Some(def) = funcs
                    .iter()
                    .map(|func| &FUNCTION_DEFINITIONS[*func])
                    .filter(|def| def.parameter_count() == call.get_arguments().len())
                    .min_by_key(|def| def.version)
                {
                    self.errors.lock().unwrap().report_error(
                        call.get_expression().get_span(),
                        ParserErrorType::FunctionVersionNotSupported(def.opcode, def.version, self.lang_version),
                    );
                    return res;
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
        for argument in indexer.get_arguments() {
            self.reject_bare_array_value(argument);
        }
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

    fn visit_on_error_statement(&mut self, on_error: &OnErrorStatement) -> VariableType {
        match on_error.get_mode() {
            OnErrorMode::Off => {}
            OnErrorMode::Goto | OnErrorMode::Gosub => self.add_label_usage(on_error.get_target_token()),
            OnErrorMode::Procedure => {
                let Some(name) = on_error.get_target() else {
                    return VariableType::None;
                };
                let Some(idx) = self.lookup_variable(name) else {
                    self.errors.lock().unwrap().report_error(
                        on_error.get_target_token().span.clone(),
                        CompilationErrorType::ProcedureNotFound(name.to_string()),
                    );
                    return VariableType::None;
                };
                if !matches!(self.references[idx].0, ReferenceType::Procedure(_)) {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(on_error.get_target_token().span.clone(), CompilationErrorType::ProcedureExpected);
                    return VariableType::None;
                }
                if let Some(container) = self.function_containers.iter().find(|p| p.name == *name)
                    && let FunctionDeclaration::Procedure(procedure) = &container.functions.clone()
                {
                    let parameters = procedure.get_parameters();
                    // The handler is called from wherever the failure happened, so there is no
                    // argument expression a VAR parameter could be written back to.
                    let takes_the_error = match parameters.len() {
                        0 => true,
                        1 => match &parameters[0] {
                            ParameterSpecifier::Variable(var) => {
                                !var.is_var() && var.get_variable_type() == VariableType::UserData(crate::parser::ERROR_ID as u8)
                            }
                            ParameterSpecifier::Function(_) | ParameterSpecifier::Procedure(_) => false,
                        },
                        _ => false,
                    };
                    if !takes_the_error {
                        self.errors.lock().unwrap().report_error(
                            on_error.get_target_token().span.clone(),
                            CompilationErrorType::InvalidErrorHandler(name.to_string()),
                        );
                    }
                }
                self.add_reference_to(on_error.get_target_token(), idx);
            }
        }
        VariableType::None
    }

    fn visit_label_statement(&mut self, label: &LabelStatement) -> VariableType {
        self.set_label_declaration(label.get_label_token());
        VariableType::None
    }

    fn visit_let_statement(&mut self, let_stmt: &LetStatement) -> VariableType {
        if let Some(target) = let_stmt.get_target_expression() {
            let target_type = target.visit(self);
            if !self.is_assignable_explicit_target(target) {
                if let Expression::MemberReference(member) = target
                    && let Some(type_id) = self.user_type_lookup.get(&member.get_identifier_token().span.start).copied()
                {
                    if self.type_registry.is_record_type(type_id) {
                        self.errors
                            .lock()
                            .unwrap()
                            .report_error(member.get_identifier_token().span.clone(), CompilationErrorType::InvalidLetVariable);
                    } else if let Some(registry) = self.type_registry.get_type_from_id(type_id)
                        && let Some(member_id) = registry.get_member_id(member.get_identifier())
                        && !matches!(registry.id_table.get(member_id), Some(crate::compiler::user_data::UserDataEntry::Field(_)))
                    {
                        self.errors.lock().unwrap().report_error(
                            member.get_identifier_token().span.clone(),
                            CompilationErrorType::MemberIsReadOnly(member.get_identifier().to_string()),
                        );
                    }
                }
                return VariableType::None;
            }
            let value_type = let_stmt.get_value_expression().visit(self);
            if let Some(target_shape) = self.array_shape(target) {
                self.check_array_target_assignment(&target_shape, let_stmt.get_value_expression(), &let_stmt.get_eq_token().span);
                return VariableType::None;
            }
            if self.array_shape(let_stmt.get_value_expression()).is_some() {
                self.reject_bare_array_value(let_stmt.get_value_expression());
                return VariableType::None;
            }
            if target_type != value_type && !matches!(target_type, VariableType::None) {
                let_stmt.get_value_expression().visit(self);
            }
            return VariableType::None;
        }
        let mut target_type = VariableType::None;
        let mut target_array_shape = None;
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
                if let Some(container) = self.function_containers.iter().find(|container| container.id == idx)
                    && let FunctionDeclaration::Function(function) = &container.functions
                {
                    target_type = function.get_return_type();
                    if function.get_return_rank() > 0 {
                        target_array_shape = Some(ArrayShape {
                            element_type: target_type,
                            rank: function.get_return_rank(),
                            bounds: [0; 3],
                            resizable: true,
                            field_name: None,
                        });
                    }
                }
            } else {
                target_type = self.references[idx].1.variable_type;
                if let Some(header) = &self.references[idx].1.header {
                    if self.lang_version >= 400 && header.dim > 0 && let_stmt.get_arguments().is_empty() {
                        target_array_shape = Some(ArrayShape {
                            element_type: target_type,
                            rank: header.dim,
                            bounds: [header.vector_size, header.matrix_size, header.cube_size],
                            resizable: true,
                            field_name: None,
                        });
                    } else {
                        self.check_arg_count(header.dim as usize, let_stmt.get_arguments().len(), let_stmt.get_identifier_token());
                    }
                } else {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(let_stmt.get_identifier_token().span.clone(), CompilationErrorType::InvalidLetVariable);
                }

                self.add_reference_to(let_stmt.get_identifier_token(), idx);

                let mut variable_type = target_type;
                for (position, member_token) in let_stmt.get_members().iter().enumerate() {
                    match variable_type {
                        VariableType::UserData(type_id) if self.type_registry.is_record_type(type_id) => {
                            let Token::Identifier(member) = &member_token.token else {
                                break;
                            };
                            variable_type = self.resolve_record_field(type_id, member, &member_token.span);
                            if position + 1 == let_stmt.get_members().len()
                                && let Some(definition) = self.type_registry.get_record_type_from_id(type_id)
                                && let Some(field_id) = definition.field_index(member)
                                && let Some(field) = definition.field(field_id)
                                && field.dim > 0
                            {
                                target_array_shape = Some(ArrayShape {
                                    element_type: field.variable_type,
                                    rank: field.dim,
                                    bounds: [field.vector_size as usize, field.matrix_size as usize, field.cube_size as usize],
                                    resizable: false,
                                    field_name: Some(member.to_string()),
                                });
                            }
                        }
                        VariableType::UserData(type_id) => {
                            let Token::Identifier(member) = &member_token.token else {
                                break;
                            };
                            let Some(registry) = self.type_registry.get_type_from_id(type_id) else {
                                self.errors
                                    .lock()
                                    .unwrap()
                                    .report_error(member_token.span.clone(), CompilationErrorType::InvalidLetVariable);
                                break;
                            };
                            let Some(member_id) = registry.get_member_id(member) else {
                                self.errors
                                    .lock()
                                    .unwrap()
                                    .report_error(member_token.span.clone(), CompilationErrorType::InvalidLetVariable);
                                break;
                            };
                            let is_last = position + 1 == let_stmt.get_members().len();
                            if is_last && !matches!(registry.id_table.get(member_id), Some(crate::compiler::user_data::UserDataEntry::Field(_))) {
                                self.errors
                                    .lock()
                                    .unwrap()
                                    .report_error(member_token.span.clone(), CompilationErrorType::MemberIsReadOnly(member.to_string()));
                                break;
                            }
                            self.user_type_lookup.insert(member_token.span.start, type_id);
                            variable_type = registry.fields.get(member).copied().unwrap_or(VariableType::None);
                        }
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
            let root = let_stmt.get_identifier();
            let Some(VariableType::UserData(mut type_id)) = self.type_registry.get_board_object(root) else {
                self.errors.lock().unwrap().report_error(
                    let_stmt.get_identifier_token().span.clone(),
                    CompilationErrorType::VariableNotFound(root.to_string()),
                );
                return VariableType::None;
            };
            let Some(provider) = self.type_registry.get_type_from_id(type_id).and_then(|registry| registry.instance_provider) else {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(let_stmt.get_identifier_token().span.clone(), CompilationErrorType::InvalidLetVariable);
                return VariableType::None;
            };
            self.instance_provider_lookup.insert(let_stmt.get_identifier_token().span.start, provider);
            for (position, member_token) in let_stmt.get_members().iter().enumerate() {
                let Token::Identifier(member) = &member_token.token else {
                    return VariableType::None;
                };
                let Some(registry) = self.type_registry.get_type_from_id(type_id) else {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(member_token.span.clone(), CompilationErrorType::InvalidLetVariable);
                    return VariableType::None;
                };
                let Some(member_id) = registry.get_member_id(member) else {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(member_token.span.clone(), CompilationErrorType::InvalidLetVariable);
                    return VariableType::None;
                };
                let is_last = position + 1 == let_stmt.get_members().len();
                if is_last && !matches!(registry.id_table.get(member_id), Some(crate::compiler::user_data::UserDataEntry::Field(_))) {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_error(member_token.span.clone(), CompilationErrorType::MemberIsReadOnly(member.to_string()));
                    return VariableType::None;
                }
                self.user_type_lookup.insert(member_token.span.start, type_id);
                target_type = registry.fields.get(member).copied().unwrap_or(VariableType::None);
                if let VariableType::UserData(next) = target_type {
                    type_id = next;
                }
            }
        }
        for arg in let_stmt.get_arguments() {
            arg.visit(self);
        }
        let value_type = let_stmt.get_value_expression().visit(self);
        if let Some(target_shape) = target_array_shape {
            self.check_array_target_assignment(&target_shape, let_stmt.get_value_expression(), &let_stmt.get_eq_token().span);
            return VariableType::None;
        }
        if self.array_shape(let_stmt.get_value_expression()).is_some() && !let_stmt.get_members().is_empty() {
            self.errors
                .lock()
                .unwrap()
                .report_error(let_stmt.get_eq_token().span.clone(), CompilationErrorType::WholeArrayUsedAsScalar);
            return VariableType::None;
        }
        if self.array_shape(let_stmt.get_value_expression()).is_some() {
            self.reject_bare_array_value(let_stmt.get_value_expression());
            return VariableType::None;
        }
        if (self.type_registry.is_enum_type(target_type) || self.type_registry.is_enum_type(value_type)) && target_type != value_type {
            self.errors.lock().unwrap().report_error(
                let_stmt.get_eq_token().span.clone(),
                CompilationErrorType::EnumAssignmentTypeMismatch(self.source_type_name(target_type), self.source_type_name(value_type)),
            );
            return VariableType::None;
        }
        // A multitype value carries its type at run time, so there is nothing here to
        // disagree with; this is what lets FOREACH hand an element to a typed variable.
        if target_type != value_type
            && value_type != VariableType::None
            && (matches!(target_type, VariableType::UserData(_)) || matches!(value_type, VariableType::UserData(_)))
        {
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
        }
        crate::ast::walk_for_stmt(self, for_stmt);
        self.reject_bare_array_value(for_stmt.get_start_expr());
        self.reject_bare_array_value(for_stmt.get_end_expr());
        if let Some(step) = for_stmt.get_step_expr() {
            self.reject_bare_array_value(step);
        }
        VariableType::None
    }

    fn visit_case_specifier(&mut self, case_specifier: &crate::ast::CaseSpecifier) -> VariableType {
        match case_specifier {
            crate::ast::CaseSpecifier::Expression(expression) => {
                expression.visit(self);
                self.reject_bare_array_value(expression);
            }
            crate::ast::CaseSpecifier::FromTo(from, to) => {
                from.visit(self);
                to.visit(self);
                self.reject_bare_array_value(from);
                self.reject_bare_array_value(to);
            }
        }
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
            self.cur_func_impl = Some(idx);
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
                        if func.get_return_type() != function.get_return_type() || func.get_return_rank() != function.get_return_rank() {
                            self.errors.lock().unwrap().report_error(
                                function.get_return_type_token().span.clone(),
                                CompilationErrorType::ReturnTypeMismatch(function.get_identifier().to_string()),
                            );
                        } // may've been wrongly added as procedure before - get's corrected.
                    } else if let FunctionDeclaration::Procedure(func) = &cont.functions
                        && func.get_parameters().len() != function.get_parameters().len()
                    {
                        self.errors.lock().unwrap().report_error(
                            function.get_identifier_token().span.clone(),
                            CompilationErrorType::ParameterMismatch(function.get_identifier().to_string()),
                        );
                    }
                    cont.functions = FunctionDeclaration::Function(
                        FunctionDeclarationAstNode::empty(
                            function.get_identifier().clone(),
                            function.get_parameters().clone(),
                            function.get_return_type(),
                        )
                        .with_return_rank(function.get_return_rank()),
                    );
                    break;
                }
            }
        } else if self.lang_version < 350 {
            self.errors.lock().unwrap().report_error(
                function.get_identifier_token().span.clone(),
                CompilationErrorType::FunctionNotFound(function.get_identifier().to_string()),
            );
        } else {
            let id = self.add_declaration(VariableType::Function, function.get_identifier_token());
            self.cur_func_impl = Some(id);
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

        self.start_parse_function_body();
        let start_parameter = self.references.len();
        self.add_parameters(function.get_parameters());
        let end_parameter = self.references.len();

        let start_locals = self.references.len();
        walk_function_implementation(self, function);
        let end_locals = self.references.len();
        let lookup = self.end_parse_function_body().unwrap();
        self.cur_func_impl = None;

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
                    if let FunctionDeclaration::Procedure(func) = &cont.functions
                        && func.get_parameters().len() != procedure.get_parameters().len()
                    {
                        self.errors.lock().unwrap().report_error(
                            procedure.get_identifier_token().span.clone(),
                            CompilationErrorType::ParameterMismatch(procedure.get_identifier().to_string()),
                        );
                    }
                    cont.functions = FunctionDeclaration::Procedure(ProcedureDeclarationAstNode::empty(
                        procedure.get_identifier().clone(),
                        procedure.get_parameters().clone(),
                    ));
                    break;
                }
            }
        } else if self.lang_version < 350 {
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
                if let FunctionDeclaration::Procedure(decl) = &f.functions
                    && decl.get_parameters().len() != procedure.get_parameters().len()
                {
                    self.errors.lock().unwrap().report_error(
                        procedure.get_identifier_token().span.clone(),
                        CompilationErrorType::ParameterMismatch(procedure.get_identifier().to_string()),
                    );
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
