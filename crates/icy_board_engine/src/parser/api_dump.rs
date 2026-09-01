//! Dumps the whole 4.00 object surface, so a review reads what the registry has
//! rather than what the source looks like.
//! `cargo test -p icy_board_engine --lib dump_api -- --ignored --nocapture`

use std::fmt::Write;

use crate::{
    compiler::user_data::{MemberFunction, MemberProcedure, UserDataEntry, UserDataRegistry},
    executable::{RecordField, VariableType},
    parser::UserTypeRegistry,
};

fn type_name(registry: &UserTypeRegistry, variable_type: VariableType) -> String {
    match variable_type {
        VariableType::None => "NONE".to_string(),
        VariableType::Boolean => "BOOLEAN".to_string(),
        VariableType::Unsigned => "UNSIGNED".to_string(),
        VariableType::Date => "DATE".to_string(),
        VariableType::EDate => "EDATE".to_string(),
        VariableType::Integer => "INTEGER".to_string(),
        VariableType::Money => "MONEY".to_string(),
        VariableType::Float => "FLOAT".to_string(),
        VariableType::String => "STRING".to_string(),
        VariableType::Time => "TIME".to_string(),
        VariableType::Byte => "BYTE".to_string(),
        VariableType::Word => "WORD".to_string(),
        VariableType::SByte => "SBYTE".to_string(),
        VariableType::SWord => "SWORD".to_string(),
        VariableType::BigStr => "BIGSTR".to_string(),
        VariableType::Double => "DOUBLE".to_string(),
        VariableType::Function => "FUNCTION".to_string(),
        VariableType::Procedure => "PROCEDURE".to_string(),
        VariableType::DDate => "DDATE".to_string(),
        VariableType::Table => "TABLE".to_string(),
        VariableType::MessageAreaID => "MSGAREAID".to_string(),
        VariableType::Password => "PASSWORD".to_string(),
        VariableType::Long => "LONG".to_string(),
        VariableType::ULong => "ULONG".to_string(),
        VariableType::Bytes => "BYTES".to_string(),
        VariableType::UnboundedString => "STRING".to_string(),
        VariableType::UserData(id) => registry
            .get_enum_from_id(id)
            .map(|definition| definition.name.to_string())
            .or_else(|| registry.get_record_type_from_id(id).map(|definition| definition.name.to_string()))
            .or_else(|| {
                registry
                    .registered_types
                    .iter()
                    .find_map(|(name, registered)| (*registered == variable_type).then(|| name.to_string()))
            })
            .unwrap_or_else(|| format!("USERDATA<{id}>")),
    }
}

fn ranked_type(registry: &UserTypeRegistry, variable_type: VariableType, rank: u8) -> String {
    let mut result = type_name(registry, variable_type);
    if rank > 0 {
        result.push('[');
        result.push_str(&",".repeat(rank.saturating_sub(1) as usize));
        result.push(']');
    }
    result
}

fn record_field_type(registry: &UserTypeRegistry, field: RecordField) -> String {
    let mut result = type_name(registry, field.variable_type);
    if field.dim > 0 {
        let bounds = [field.vector_size, field.matrix_size, field.cube_size];
        result.push('(');
        for (index, bound) in bounds[..field.dim as usize].iter().enumerate() {
            if index > 0 {
                result.push_str(", ");
            }
            write!(result, "{bound}").unwrap();
        }
        result.push(')');
    }
    result
}

fn parameters(registry: &UserTypeRegistry, types: &[VariableType], names: &[String], required: usize) -> String {
    types
        .iter()
        .enumerate()
        .map(|(index, variable_type)| {
            let name = names.get(index).cloned().unwrap_or_else(|| format!("arg{}", index + 1));
            let parameter = format!("{name}: {}", type_name(registry, *variable_type));
            if index < required { parameter } else { format!("[{parameter}]") }
        })
        .collect::<Vec<_>>()
        .join(", ")
}

fn function_signature(registry: &UserTypeRegistry, signature: &MemberFunction) -> String {
    format!(
        "({}) -> {}",
        parameters(registry, &signature.parameters, &signature.parameter_names, signature.required),
        ranked_type(registry, signature.return_type, signature.return_rank)
    )
}

fn procedure_signature(registry: &UserTypeRegistry, signature: &MemberProcedure) -> String {
    format!(
        "({})",
        parameters(registry, &signature.parameters, &signature.parameter_names, signature.required)
    )
}

fn property_is_writable(definition: &UserDataRegistry, name: &unicase::Ascii<String>) -> bool {
    definition
        .member_id_lookup
        .get(name)
        .and_then(|id| definition.id_table.get(*id))
        .is_some_and(|entry| matches!(entry, UserDataEntry::Field(_)))
}

fn api_dump() -> String {
    let registry = UserTypeRegistry::icy_board_registry();
    let mut output = String::new();
    let mut names: Vec<_> = registry.registered_types.iter().collect();
    names.sort_by_key(|(name, _)| name.to_string().to_ascii_lowercase());

    for (name, variable_type) in names {
        let VariableType::UserData(id) = variable_type else {
            continue;
        };
        if let Some(record) = registry.get_record_type_from_id(*id) {
            writeln!(output, "\nRECORD {name} (id {id})").unwrap();
            for (field_name, field) in record.fields {
                writeln!(output, "  FIELD {field_name}: {}", record_field_type(&registry, field)).unwrap();
            }
            continue;
        }
        let Some(definition) = registry.get_type_from_id(*id) else {
            continue;
        };
        writeln!(output, "\nOBJECT {name} (id {id})").unwrap();

        let mut fields: Vec<_> = definition.fields.iter().collect();
        fields.sort_by_key(|(member, _)| member.to_string().to_ascii_lowercase());
        for (member, member_type) in fields {
            let rank = definition.field_ranks.get(member).copied().unwrap_or(0);
            let access = if property_is_writable(definition, member) { "writable" } else { "read-only" };
            writeln!(output, "  PROPERTY {member}: {} [{access}]", ranked_type(&registry, *member_type, rank)).unwrap();
        }

        let mut functions: Vec<_> = definition.functions.iter().collect();
        functions.sort_by_key(|(member, _)| member.to_string().to_ascii_lowercase());
        for (member, signature) in functions {
            let owner = if definition.statics.contains(member) { "static" } else { "member" };
            writeln!(output, "  FUNCTION {member}{} [{owner}]", function_signature(&registry, signature)).unwrap();
        }

        let mut procedures: Vec<_> = definition.procedures.iter().collect();
        procedures.sort_by_key(|(member, _)| member.to_string().to_ascii_lowercase());
        for (member, signature) in procedures {
            writeln!(output, "  PROCEDURE {member}{} [member]", procedure_signature(&registry, signature)).unwrap();
        }
    }

    let mut enums = registry.enums();
    enums.sort_by_key(|definition| definition.name.to_string().to_ascii_lowercase());
    for definition in enums {
        writeln!(output, "\nENUM {} (id {})", definition.name, definition.id).unwrap();
        for (variant, value) in definition.variants {
            writeln!(output, "  VALUE {variant} = {value}").unwrap();
        }
    }
    output
}

#[test]
#[ignore = "prints the API for review"]
fn dump_api() {
    let output = api_dump();
    print!("{output}");
    assert!(output.contains("PROPERTY Conferences: Conference[] [read-only]"));
    assert!(output.contains("FUNCTION FindAll(text: STRING, [start: INTEGER], [limit: INTEGER]) -> RegexMatch[] [member]"));
    assert!(output.contains("RECORD CONTACT (id 34)\n  FIELD Service: STRING\n  FIELD Account: STRING"));
    assert!(output.contains("PROPERTY Alias: STRING [writable]"));
    assert!(output.contains("FUNCTION New(method: HttpMethod, url: STRING) -> HttpRequest [static]"));
    assert!(output.contains("FUNCTION SetText(text: STRING, [contentType: STRING]) -> BOOLEAN [member]"));
    assert!(output.contains("ENUM GfxBackend (id 250)\n  VALUE None = -1\n  VALUE Auto = 0"));
}
