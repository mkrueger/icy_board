//! Turning a name or a member chain into the type it has, so that completion
//! and hover can say what a record or a board object holds.

use icy_board_engine::{
    executable::{FUNCTION_DEFINITIONS, VariableType},
    parser::{UserTypeRegistry, is_user_declared_type},
    semantic::{ReferenceType, SemanticVisitor},
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
    fallback
}

/// The type a field of `var_type` has.
pub fn type_of_member(registry: &UserTypeRegistry, var_type: VariableType, member: &str) -> Option<VariableType> {
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
            .map(|(name, field_type)| Member {
                name: name.to_string(),
                detail: type_name(registry, *field_type),
                kind: MemberKind::Field,
            })
            .collect();
    }

    let Some(object) = registry.get_type_from_id(id) else {
        return Vec::new();
    };
    let mut members = Vec::new();
    for (name, field_type) in &object.fields {
        members.push(Member {
            name: name.to_string(),
            detail: type_name(registry, *field_type),
            kind: MemberKind::Field,
        });
    }
    for (name, function) in &object.functions {
        members.push(Member {
            name: name.to_string(),
            detail: format!(
                "({}) {}",
                parameter_types(registry, &function.parameters),
                type_name(registry, function.return_type)
            ),
            kind: MemberKind::Method,
        });
    }
    for (name, procedure) in &object.procedures {
        members.push(Member {
            name: name.to_string(),
            detail: format!("({})", parameter_types(registry, &procedure.parameters)),
            kind: MemberKind::Method,
        });
    }
    members.sort_by(|a, b| a.name.cmp(&b.name));
    members
}

fn parameter_types(registry: &UserTypeRegistry, parameters: &[VariableType]) -> String {
    parameters.iter().map(|p| type_name(registry, *p)).collect::<Vec<_>>().join(", ")
}
