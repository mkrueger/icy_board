//! Turning a name or a member chain into the type it has, so that completion
//! and hover can say what a record or a board object holds.

use icy_board_engine::{
    executable::{FUNCTION_DEFINITIONS, VariableType},
    parser::{UserTypeRegistry, is_user_declared_type},
    semantic::{BYTES_MEMBERS, ReferenceType, STRING_MEMBERS, SemanticVisitor},
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
        for (name, registered) in &registry.registered_types {
            if *registered == var_type {
                return name.to_string();
            }
        }
    }
    var_type.to_string().to_ascii_uppercase()
}

pub fn record_field_type_name(registry: &UserTypeRegistry, field: icy_board_engine::executable::RecordField) -> String {
    let mut name = type_name(registry, field.variable_type);
    if field.dim > 0 {
        let dimensions = [field.vector_size, field.matrix_size, field.cube_size]
            .into_iter()
            .take(field.dim as usize)
            .map(|bound| bound.to_string())
            .collect::<Vec<_>>()
            .join(", ");
        name.push('(');
        name.push_str(&dimensions);
        name.push(')');
    }
    name
}

/// The type of a variable, or the return type of a routine, by name.
pub fn type_of_name(visitor: &SemanticVisitor, name: &str) -> Option<VariableType> {
    let name = unicase::Ascii::new(name.to_string());

    for (reference_type, reference) in &visitor.references {
        if !matches!(reference_type, ReferenceType::Variable(_) | ReferenceType::Function(_)) {
            continue;
        }
        let declared = reference
            .declaration
            .as_ref()
            .or(reference.implementation.as_ref())
            .map(|(_, decl)| unicase::Ascii::new(decl.token.clone()));
        if declared == Some(name.clone()) {
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
        return Some(VariableType::String);
    }
    if name == "BIGSTR" {
        return Some(VariableType::BigStr);
    }
    if let Some(var_type) = visitor.type_registry.get_board_object(&name) {
        return Some(var_type);
    }
    fallback
}

pub fn static_type_of_name(visitor: &SemanticVisitor, name: &str) -> Option<VariableType> {
    let identifier = unicase::Ascii::new(name.to_string());
    let shadowed = visitor.references.iter().any(|(reference_type, reference)| {
        matches!(reference_type, ReferenceType::Variable(_) | ReferenceType::Function(_))
            && reference
                .declaration
                .as_ref()
                .or(reference.implementation.as_ref())
                .is_some_and(|(_, declaration)| unicase::Ascii::new(declaration.token.clone()) == identifier)
    });
    (!shadowed).then(|| visitor.type_registry.get_board_object(&identifier)).flatten()
}

/// The type a field of `var_type` has.
pub fn type_of_member(registry: &UserTypeRegistry, var_type: VariableType, member: &str) -> Option<VariableType> {
    if matches!(var_type, VariableType::String | VariableType::BigStr) {
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

    if is_user_declared_type(id) {
        let def = registry.get_user_type_from_id(id)?;
        return def.field_type(def.field_index(&member)?);
    }

    let object = registry.get_type_from_id(id)?;
    if let Some(field) = object.fields.get(&member) {
        return Some(*field);
    }
    object.functions.get(&member).map(|function| function.return_type)
}

/// Walks a member chain such as `members[0].Home` and answers the type it ends in.
pub fn type_of_chain(visitor: &SemanticVisitor, path: &[String]) -> Option<VariableType> {
    let (first, rest) = path.split_first()?;
    let mut var_type = type_of_name(visitor, first)?;
    for member in rest {
        // An array indexes into its own type, so a step it does not have is no step.
        if member == crate::context::INDEXED {
            var_type = type_of_member(&visitor.type_registry, var_type, member).unwrap_or(var_type);
            continue;
        }
        var_type = type_of_member(&visitor.type_registry, var_type, member)?;
    }
    Some(var_type)
}

/// Everything that may follow a `.` on a value of this type.
pub fn members_of(registry: &UserTypeRegistry, var_type: VariableType) -> Vec<Member> {
    if matches!(var_type, VariableType::String | VariableType::BigStr) {
        return string_members(false);
    }
    if var_type == VariableType::Bytes {
        return bytes_members(false);
    }
    let VariableType::UserData(id) = var_type else {
        return Vec::new();
    };

    if is_user_declared_type(id) {
        let Some(def) = registry.get_user_type_from_id(id) else {
            return Vec::new();
        };
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

fn user_data_members(registry: &UserTypeRegistry, object: &icy_board_engine::compiler::user_data::UserDataRegistry, statik: bool) -> Vec<Member> {
    let mut members = Vec::new();
    for (name, field_type) in object.fields.iter().filter(|_| !statik) {
        members.push(Member {
            name: name.to_string(),
            detail: format!(
                "{}{}",
                type_name(registry, *field_type),
                "[]".repeat(object.field_ranks.get(name).copied().unwrap_or(0) as usize)
            ),
            kind: MemberKind::Field,
        });
    }
    for (name, function) in object.functions.iter().filter(|(name, _)| object.statics.contains(*name) == statik) {
        members.push(Member {
            name: name.to_string(),
            detail: format!(
                "({}) {}",
                parameter_types(registry, &function.parameters),
                format!("{}{}", type_name(registry, function.return_type), "[]".repeat(function.return_rank as usize))
            ),
            kind: MemberKind::Method,
        });
    }
    for (name, procedure) in object.procedures.iter().filter(|_| !statik) {
        members.push(Member {
            name: name.to_string(),
            detail: format!("({})", parameter_types(registry, &procedure.parameters)),
            kind: MemberKind::Method,
        });
    }
    members.sort_by(|a, b| a.name.cmp(&b.name));
    members
}

pub fn string_members(statik: bool) -> Vec<Member> {
    STRING_MEMBERS
        .iter()
        .filter(|member| member.is_static == statik)
        .map(|member| Member {
            name: member.name.to_string(),
            detail: format!(
                "({}..{} args) {}{}",
                member.arguments.start(),
                member.arguments.end(),
                type_name(&UserTypeRegistry::default(), member.return_type),
                if member.name == "Split" { "[]" } else { "" }
            ),
            kind: MemberKind::Method,
        })
        .collect()
}

pub fn bytes_members(statik: bool) -> Vec<Member> {
    BYTES_MEMBERS
        .iter()
        .filter(|member| member.is_static == statik)
        .map(|member| Member {
            name: member.name.to_string(),
            detail: format!(
                "({}..{} args) {}",
                member.arguments.start(),
                member.arguments.end(),
                type_name(&UserTypeRegistry::default(), member.return_type)
            ),
            kind: MemberKind::Method,
        })
        .collect()
}

fn parameter_types(registry: &UserTypeRegistry, parameters: &[VariableType]) -> String {
    parameters.iter().map(|p| type_name(registry, *p)).collect::<Vec<_>>().join(", ")
}
