//! Turning a name or a member chain into the type it has, so that completion
//! and hover can say what a record or a board object holds.

use icy_board_ppl::{
    executable::{FUNCTION_DEFINITIONS, VariableType},
    parser::UserTypeRegistry,
    semantic::{ARRAY_MEMBERS, ARRAY_PROCEDURES, BYTES_MEMBERS, FunctionDeclaration, ReferenceType, STRING_MEMBERS, SemanticVisitor},
};

/// One member of a record or of a board object.
pub struct Member {
    pub name: String,
    pub detail: String,
    pub kind: MemberKind,
}

pub enum MemberKind {
    Field,
    Method,
}

/// How a type is spelled in source.
pub fn type_name(registry: &UserTypeRegistry, var_type: VariableType) -> String {
    if let VariableType::UserData(id) = var_type {
        if let Some(def) = registry.get_user_type_from_id(id) {
            return def.name.to_string();
        }
        if let Some(def) = registry.get_enum_from_id(id) {
            return def.name.to_string();
        }
        for (name, registered) in &registry.registered_types {
            if *registered == var_type {
                return name.to_string();
            }
        }
    }
    var_type.to_string().to_ascii_uppercase()
}

pub fn record_field_type_name(registry: &UserTypeRegistry, field: icy_board_ppl::executable::RecordField) -> String {
    let mut name = type_name(registry, field.variable_type);
    if field.dim > 0 {
        let dimensions = [field.vector_size, field.matrix_size, field.cube_size]
            .into_iter()
            .take(field.dim as usize)
            .map(|bound| bound.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        name.push('[');
        name.push_str(&dimensions);
        name.push(']');
    }
    name
}

/// The type of a variable, or the return type of a routine, by name.
pub fn type_of_name(visitor: &SemanticVisitor, name: &str) -> Option<VariableType> {
    let name = unicase::Ascii::new(name.to_string());

    for (reference_type, reference) in &visitor.references {
        if !matches!(
            reference_type,
            ReferenceType::Variable(_) | ReferenceType::Constant(_) | ReferenceType::Function(_)
        ) {
            continue;
        }
        let declared = reference
            .declaration
            .as_ref()
            .or(reference.implementation.as_ref())
            .map(|(_, decl)| unicase::Ascii::new(decl.token.clone()));
        if declared == Some(name.clone()) {
            if matches!(reference_type, ReferenceType::Function(_)) || reference.variable_type == VariableType::Function {
                return visitor
                    .function_containers
                    .iter()
                    .find(|container| container.name.eq_ignore_ascii_case(name.as_ref()))
                    .and_then(|container| match &container.functions {
                        icy_board_ppl::semantic::FunctionDeclaration::Function(function) => Some(function.get_return_type()),
                        _ => None,
                    });
            }
            return Some(reference.variable_type);
        }
    }

    // A built-in function may be overloaded; the one answering an object wins,
    // because that is the one whose members can be offered.
    let mut fallback = None;
    for def in FUNCTION_DEFINITIONS.iter() {
        if !def.name.eq_ignore_ascii_case(name.as_ref()) {
            continue;
        }
        if matches!(def.return_type, VariableType::UserData(_)) {
            return Some(def.return_type);
        }
        fallback.get_or_insert(def.return_type);
    }
    if name == "STRING" {
        return Some(VariableType::UnboundedString);
    }
    if name == "BIGSTR" {
        return Some(VariableType::BigStr);
    }
    if name == "BYTES" {
        return Some(VariableType::Bytes);
    }
    if let Some(var_type) = visitor.type_registry.get_board_object(&name) {
        return Some(var_type);
    }
    if let Some(definition) = visitor.type_registry.get_enum(&name) {
        return Some(VariableType::UserData(definition.id));
    }
    fallback
}

pub fn static_type_of_name(visitor: &SemanticVisitor, name: &str) -> Option<VariableType> {
    let identifier = unicase::Ascii::new(name.to_string());
    let shadowed = visitor.references.iter().any(|(reference_type, reference)| {
        matches!(
            reference_type,
            ReferenceType::Variable(_) | ReferenceType::Constant(_) | ReferenceType::Function(_)
        ) && reference
            .declaration
            .as_ref()
            .or(reference.implementation.as_ref())
            .is_some_and(|(_, declaration)| unicase::Ascii::new(declaration.token.clone()) == identifier)
    });
    (!shadowed).then(|| visitor.type_registry.get_board_object(&identifier)).flatten()
}

/// The type a field of `var_type` has.
pub fn type_of_member(registry: &UserTypeRegistry, var_type: VariableType, member: &str) -> Option<VariableType> {
    if matches!(var_type, VariableType::String | VariableType::BigStr | VariableType::UnboundedString) {
        return STRING_MEMBERS
            .iter()
            .find(|definition| !definition.is_static && definition.name.eq_ignore_ascii_case(member))
            .map(|definition| definition.return_type);
    }
    if var_type == VariableType::Bytes {
        return BYTES_MEMBERS
            .iter()
            .find(|definition| definition.name.eq_ignore_ascii_case(member))
            .map(|definition| definition.return_type);
    }
    let VariableType::UserData(id) = var_type else {
        return None;
    };
    let member = unicase::Ascii::new(member.to_string());

    if registry.get_enum_from_id(id).is_some() {
        return (member == "Has").then_some(VariableType::Boolean);
    }
    if let Some(def) = registry.get_record_type_from_id(id) {
        return def.field_type(def.field_index(&member)?);
    }

    let object = registry.get_type_from_id(id)?;
    if let Some(field) = object.fields.get(&member) {
        return Some(*field);
    }
    object.functions.get(&member).map(|function| function.return_type)
}

/// Element type and shape must travel together, including through calls. Only
/// a named mutable array slot can be redimensioned; fields and results cannot.
#[derive(Clone, Copy, Debug)]
pub struct ReceiverType {
    pub variable_type: VariableType,
    pub rank: u8,
    pub namespace: bool,
    pub resizable: bool,
    callable: bool,
}

impl ReceiverType {
    fn scalar(variable_type: VariableType) -> Self {
        Self {
            variable_type,
            rank: 0,
            namespace: false,
            resizable: false,
            callable: false,
        }
    }
}

pub fn receiver_type(visitor: &SemanticVisitor, path: &[String]) -> Option<ReceiverType> {
    receiver_type_for_version(visitor, path, 400)
}

pub fn receiver_type_for_version(visitor: &SemanticVisitor, path: &[String], language_version: u16) -> Option<ReceiverType> {
    let (first, rest) = path.split_first()?;
    let mut result = ReceiverType::scalar(type_of_name(visitor, first)?);
    let reference = visitor.references.iter().find(|(kind, reference)| {
        matches!(kind, ReferenceType::Variable(_) | ReferenceType::Constant(_) | ReferenceType::Function(_))
            && reference
                .declaration
                .as_ref()
                .or(reference.implementation.as_ref())
                .is_some_and(|(_, token)| token.token.eq_ignore_ascii_case(first))
    });
    let identifier = unicase::Ascii::new(first.clone());
    result.namespace = reference.is_none()
        && (visitor.type_registry.get_enum(&identifier).is_some()
            || visitor.type_registry.get_board_object(&identifier).is_some()
            || matches!(first.to_ascii_uppercase().as_str(), "STRING" | "BIGSTR" | "BYTES"));
    result.callable = reference
        .is_some_and(|(kind, reference)| matches!(kind, ReferenceType::Function(_)) || reference.variable_type == VariableType::Function)
        || (!result.namespace && FUNCTION_DEFINITIONS.iter().any(|def| def.name.eq_ignore_ascii_case(first)));
    result.rank = reference.and_then(|(_, reference)| reference.header.as_ref()).map_or(0, |header| header.dim);
    result.resizable = reference.is_some_and(|(kind, _)| matches!(kind, ReferenceType::Variable(_))) && !result.callable;
    if result.callable {
        result.rank = visitor
            .function_containers
            .iter()
            .find(|container| container.name.eq_ignore_ascii_case(first))
            .and_then(|container| match &container.functions {
                FunctionDeclaration::Function(function) => Some(function.get_return_rank()),
                _ => None,
            })
            .unwrap_or(0);
    }
    for member in rest {
        if member == crate::context::CALLED || member == crate::context::INDEXED {
            if member == crate::context::CALLED && (result.callable || result.namespace) {
                result.callable = false;
                result.namespace = false;
            } else if result.rank > 0 {
                result.rank = 0;
            } else {
                result.variable_type = type_of_member(&visitor.type_registry, result.variable_type, crate::context::INDEXED)?;
            }
            result.resizable = false;
            continue;
        }
        if result.namespace
            && let VariableType::UserData(id) = result.variable_type
            && visitor.type_registry.is_enum_type(result.variable_type)
        {
            let definition = visitor.type_registry.get_enum_from_id(id)?;
            definition.value(&unicase::Ascii::new(member.clone()))?;
            result.namespace = false;
            continue;
        }
        if result.rank == 0 && scalar_type(result.variable_type) && language_version < 400 {
            return None;
        }
        if let Some(method) = callable_member(&visitor.type_registry, result, member) {
            result = ReceiverType {
                callable: true,
                rank: method.return_rank,
                ..ReceiverType::scalar(method.return_type?)
            };
            continue;
        }
        if result.rank > 0 {
            return None;
        }
        let VariableType::UserData(id) = result.variable_type else { return None };
        let name = unicase::Ascii::new(member.clone());
        if let Some(record) = visitor.type_registry.get_record_type_from_id(id) {
            let field = record.field(record.field_index(&name)?)?;
            result = ReceiverType {
                rank: field.dim,
                ..ReceiverType::scalar(field.variable_type)
            };
        } else {
            let object = visitor.type_registry.get_type_from_id(id)?;
            if result.namespace && object.instance_provider.is_none() {
                return None;
            }
            result = ReceiverType {
                rank: object.field_ranks.get(&name).copied().unwrap_or(0),
                ..ReceiverType::scalar(*object.fields.get(&name)?)
            };
        }
    }
    Some(result)
}

/// Compatibility helper for hover callers which only need the element type.
pub fn type_of_chain(visitor: &SemanticVisitor, path: &[String]) -> Option<VariableType> {
    receiver_type(visitor, path).map(|receiver| receiver.variable_type)
}

pub fn enum_instance_type(visitor: &SemanticVisitor, path: &[String]) -> Option<VariableType> {
    let receiver = receiver_type(visitor, path)?;
    (!receiver.namespace && receiver.rank == 0 && visitor.type_registry.is_enum_type(receiver.variable_type)).then_some(receiver.variable_type)
}

/// One callable description shared by completion and signature help. Registry
/// members supply all their own metadata; scalar tables currently omit only
/// parameter names/types and return rank, supplied by the adapter below.
pub struct CallableMember {
    pub parameters: Vec<VariableType>,
    pub parameter_names: Vec<String>,
    pub parameter_ranks: Vec<u8>,
    pub required: usize,
    pub return_type: Option<VariableType>,
    pub return_rank: u8,
}

pub fn ranked_type_name(registry: &UserTypeRegistry, typ: VariableType, rank: u8) -> String {
    let mut name = type_name(registry, typ);
    if rank > 0 {
        name.push_str(&format!("[{}]", ",".repeat(usize::from(rank - 1))));
    }
    name
}

pub fn scalar_type(typ: VariableType) -> bool {
    matches!(
        typ,
        VariableType::String | VariableType::BigStr | VariableType::UnboundedString | VariableType::Bytes
    )
}

pub fn callable_member(registry: &UserTypeRegistry, receiver: ReceiverType, member: &str) -> Option<CallableMember> {
    let typ = receiver.variable_type;
    let name = unicase::Ascii::new(member.to_string());
    let mut result = CallableMember {
        parameters: Vec::new(),
        parameter_names: Vec::new(),
        parameter_ranks: Vec::new(),
        required: 0,
        return_type: None,
        return_rank: 0,
    };
    if receiver.rank > 0 {
        if let Some(definition) = ARRAY_MEMBERS.iter().find(|definition| name == definition.name) {
            result.parameters = vec![VariableType::Integer; *definition.arguments.end()];
            result.parameter_names = vec!["dimension".into()];
            result.required = *definition.arguments.start();
            result.return_type = Some(definition.return_type);
        } else if receiver.resizable && ARRAY_PROCEDURES.iter().any(|(member, _, _)| name == *member) {
            result.parameters = vec![VariableType::Integer; receiver.rank as usize];
            result.parameter_names = ["vector", "matrix", "cube"]
                .into_iter()
                .take(receiver.rank as usize)
                .map(String::from)
                .collect();
            result.required = receiver.rank as usize;
        } else {
            return None;
        }
        return Some(result);
    }
    if registry.is_enum_type(typ) {
        if receiver.namespace || name != "Has" {
            return None;
        }
        result.parameters.push(typ);
        result.parameter_names.push("mask".into());
        result.required = 1;
        result.return_type = Some(VariableType::Boolean);
        return Some(result);
    }
    if scalar_type(typ) {
        let definitions = if typ == VariableType::Bytes { BYTES_MEMBERS } else { STRING_MEMBERS };
        let definition = definitions
            .iter()
            .find(|definition| definition.is_static == receiver.namespace && name == definition.name)?;
        result.required = *definition.arguments.start();
        result.return_type = Some(definition.return_type);
        // No opcode argument metadata exists for the new scalar opcodes. Keep
        // this single adapter until the compiler exports typed scalar parameters.
        use VariableType::{Integer as I, UnboundedString as S, UserData};
        let comparison = UserData(icy_board_ppl::parser::STRING_COMPARISON_ENUM_ID);
        let parameters: Vec<(&str, VariableType)> = match definition.name {
            "Find" | "FindLast" => vec![("value", S), ("start", I), ("comparison", comparison)],
            "Contains" | "StartsWith" | "EndsWith" | "Count" | "Equals" => vec![("value", S), ("comparison", comparison)],
            "Replace" => vec![("oldValue", S), ("newValue", S)],
            "Trim" | "TrimStart" | "TrimEnd" => vec![("characters", S)],
            "Substring" | "Remove" => vec![("start", I), ("length", I)],
            "Left" | "Right" => vec![("length", I)],
            "Split" => {
                result.return_rank = 1;
                if receiver.namespace {
                    vec![("text", S), ("separator", S), ("limit", I)]
                } else {
                    vec![("separator", S), ("limit", I)]
                }
            }
            "Join" => {
                result.parameter_ranks = vec![1, 0];
                vec![("values", S), ("separator", S)]
            }
            "Repeat" => vec![("text", S), ("count", I)],
            "PadLeft" | "PadRight" => vec![("width", I), ("character", S)],
            "Insert" => vec![("start", I), ("value", S)],
            "ToInt" => vec![("base", I)],
            "GetChecksum" => vec![("algorithm", UserData(icy_board_ppl::parser::CHECKSUM_ENUM_ID))],
            "FromBase64" => vec![("text", S)],
            _ if *definition.arguments.end() == 0 => Vec::new(),
            _ => return None,
        };
        debug_assert_eq!(parameters.len(), *definition.arguments.end());
        for (name, typ) in parameters {
            result.parameter_names.push(name.into());
            result.parameters.push(typ);
        }
        return Some(result);
    }
    let VariableType::UserData(id) = typ else { return None };
    let object = registry.get_type_from_id(id)?;
    if let Some(function) = object.functions.get(&name) {
        let is_static = object.statics.contains(&name);
        if is_static != receiver.namespace && !(receiver.namespace && !is_static && object.instance_provider.is_some()) {
            return None;
        }
        result.parameters = function.parameters.clone();
        result.parameter_names = function.parameter_names.clone();
        result.required = function.required;
        result.return_type = Some(function.return_type);
        result.return_rank = function.return_rank;
    } else {
        if receiver.namespace && object.instance_provider.is_none() {
            return None;
        }
        let procedure = object.procedures.get(&name)?;
        result.parameters = procedure.parameters.clone();
        result.parameter_names = procedure.parameter_names.clone();
        result.required = procedure.required;
    }
    Some(result)
}

pub fn array_members(registry: &UserTypeRegistry, receiver: ReceiverType) -> Vec<Member> {
    ARRAY_MEMBERS
        .iter()
        .map(|member| member.name)
        .chain(ARRAY_PROCEDURES.iter().filter(|_| receiver.resizable).map(|(name, _, _)| *name))
        .filter_map(|name| {
            let method = callable_member(registry, receiver, name)?;
            Some(Member {
                name: name.into(),
                detail: callable_detail(registry, &method),
                kind: MemberKind::Method,
            })
        })
        .collect()
}

pub fn callable_detail(registry: &UserTypeRegistry, method: &CallableMember) -> String {
    let parameters = method
        .parameters
        .iter()
        .enumerate()
        .map(|(index, typ)| {
            let typ = ranked_type_name(registry, *typ, method.parameter_ranks.get(index).copied().unwrap_or(0));
            let text = method.parameter_names.get(index).map_or(typ.clone(), |name| format!("{typ} {name}"));
            if index < method.required { text } else { format!("[{text}]") }
        })
        .collect::<Vec<_>>()
        .join(", ");
    let tail = method
        .return_type
        .map_or(String::new(), |typ| format!(" {}", ranked_type_name(registry, typ, method.return_rank)));
    format!("({parameters}){tail}")
}

/// Everything that may follow a `.` on a value of this type.
pub fn members_of(registry: &UserTypeRegistry, var_type: VariableType) -> Vec<Member> {
    if registry.is_enum_type(var_type) {
        return vec![Member {
            name: "Has".to_string(),
            detail: format!("({} mask) BOOLEAN", type_name(registry, var_type)),
            kind: MemberKind::Method,
        }];
    }
    if matches!(var_type, VariableType::String | VariableType::BigStr | VariableType::UnboundedString) {
        return string_members(false);
    }
    if var_type == VariableType::Bytes {
        return bytes_members(false);
    }
    let VariableType::UserData(id) = var_type else {
        return Vec::new();
    };

    if let Some(def) = registry.get_record_type_from_id(id) {
        return def
            .fields
            .iter()
            .map(|(name, field)| Member {
                name: name.to_string(),
                detail: record_field_type_name(registry, *field),
                kind: MemberKind::Field,
            })
            .collect();
    }

    let Some(object) = registry.get_type_from_id(id) else {
        return Vec::new();
    };
    user_data_members(registry, object, false)
}

pub fn static_members_of(registry: &UserTypeRegistry, var_type: VariableType) -> Vec<Member> {
    if matches!(var_type, VariableType::String | VariableType::BigStr | VariableType::UnboundedString) {
        return string_members(true);
    }
    if var_type == VariableType::Bytes {
        return bytes_members(true);
    }
    let VariableType::UserData(id) = var_type else {
        return Vec::new();
    };
    let Some(object) = registry.get_type_from_id(id) else {
        return Vec::new();
    };
    user_data_members(registry, object, object.instance_provider.is_none())
}

fn user_data_members(registry: &UserTypeRegistry, object: &icy_board_ppl::compiler::user_data::UserDataRegistry, statik: bool) -> Vec<Member> {
    let mut members = Vec::new();
    for (name, field_type) in object.fields.iter().filter(|_| !statik) {
        members.push(Member {
            name: name.to_string(),
            detail: format!(
                "{}",
                ranked_type_name(registry, *field_type, object.field_ranks.get(name).copied().unwrap_or(0))
            ),
            kind: MemberKind::Field,
        });
    }
    for (name, function) in object.functions.iter().filter(|(name, _)| object.statics.contains(*name) == statik) {
        members.push(Member {
            name: name.to_string(),
            detail: format!(
                "({}) {}",
                named_parameters(registry, &function.parameters, &function.parameter_names, function.required),
                ranked_type_name(registry, function.return_type, function.return_rank)
            ),
            kind: MemberKind::Method,
        });
    }
    for (name, procedure) in object.procedures.iter().filter(|_| !statik) {
        members.push(Member {
            name: name.to_string(),
            detail: format!(
                "({})",
                named_parameters(registry, &procedure.parameters, &procedure.parameter_names, procedure.required)
            ),
            kind: MemberKind::Method,
        });
    }
    members.sort_by(|a, b| a.name.cmp(&b.name));
    members
}

pub fn string_members(statik: bool) -> Vec<Member> {
    scalar_members(VariableType::UnboundedString, statik)
}

pub fn bytes_members(statik: bool) -> Vec<Member> {
    scalar_members(VariableType::Bytes, statik)
}

fn scalar_members(typ: VariableType, statik: bool) -> Vec<Member> {
    let registry = UserTypeRegistry::icy_board_registry();
    let receiver = ReceiverType {
        namespace: statik,
        ..ReceiverType::scalar(typ)
    };
    let definitions = if typ == VariableType::Bytes { BYTES_MEMBERS } else { STRING_MEMBERS };
    definitions
        .iter()
        .filter(|member| member.is_static == statik)
        .filter_map(|member| {
            Some(Member {
                name: member.name.to_string(),
                detail: callable_detail(&registry, &callable_member(&registry, receiver, member.name)?),
                kind: MemberKind::Method,
            })
        })
        .collect()
}

fn named_parameters(registry: &UserTypeRegistry, parameters: &[VariableType], names: &[String], required: usize) -> String {
    parameters
        .iter()
        .enumerate()
        .map(|(index, parameter)| {
            let var_type = type_name(registry, *parameter);
            let text = names.get(index).map_or(var_type.clone(), |name| format!("{var_type} {name}"));
            if index < required { text } else { format!("[{text}]") }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

/// The parameter list of a callable member, so a signature can read as a call.
pub fn member_parameters(registry: &UserTypeRegistry, receiver: VariableType, member: &unicase::Ascii<String>) -> Option<String> {
    if registry.is_enum_type(receiver) && *member == "Has" {
        return Some(format!("{} mask", type_name(registry, receiver)));
    }
    let VariableType::UserData(id) = receiver else {
        return None;
    };
    let object = registry.get_type_from_id(id)?;
    if let Some(function) = object.functions.get(member) {
        return Some(named_parameters(registry, &function.parameters, &function.parameter_names, function.required));
    }
    object
        .procedures
        .get(member)
        .map(|procedure| named_parameters(registry, &procedure.parameters, &procedure.parameter_names, procedure.required))
}
