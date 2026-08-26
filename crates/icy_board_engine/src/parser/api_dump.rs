//! Dumps the whole 4.00 object surface, so a review reads what the registry has
//! rather than what the source looks like.
//! `cargo test -p icy_board_engine --lib dump_api -- --ignored --nocapture`

use crate::parser::UserTypeRegistry;

#[test]
#[ignore = "prints the API for review"]
fn dump_api() {
    let registry = UserTypeRegistry::icy_board_registry();
    let mut names: Vec<_> = registry.registered_types.iter().collect();
    names.sort_by_key(|(name, _)| name.to_string());

    for (name, var_type) in names {
        let crate::executable::VariableType::UserData(id) = var_type else {
            continue;
        };
        let Some(def) = registry.get_type_from_id(*id) else {
            continue;
        };
        println!("\n{name} (id {id})");
        let mut fields: Vec<_> = def.fields.iter().collect();
        fields.sort_by_key(|(name, _)| name.to_string());
        for (member, member_type) in fields {
            println!("  . {member}: {member_type:?}");
        }
        let mut functions: Vec<_> = def.functions.iter().collect();
        functions.sort_by_key(|(name, _)| name.to_string());
        for (member, signature) in functions {
            let statik = if def.statics.contains(member) { "static " } else { "" };
            println!("  {statik}{member}({:?}) -> {:?}", signature.parameters, signature.return_type);
        }
        let mut procedures: Vec<_> = def.procedures.iter().collect();
        procedures.sort_by_key(|(name, _)| name.to_string());
        for (member, signature) in procedures {
            println!("  {member}({:?}) [procedure]", signature.parameters);
        }
    }

    println!("\n--- enums ---");
    for definition in registry.enums() {
        println!("{} (id {})", definition.name, definition.id);
    }
}
