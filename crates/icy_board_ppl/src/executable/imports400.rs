use std::collections::{BTreeMap, BTreeSet, HashMap};

use super::code400::{Reader, Result, blob, word};
use super::container::{ContainerError, Section};
use super::{FuncOpCode, PPECommand, PPEExpr, PPEScript, RecordField, VariableTable, VariableType};
use crate::{
    compiler::user_data::UserDataEntry,
    parser::{FIRST_USER_TYPE_ID, UserTypeRegistry},
};

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemberImport {
    pub name: String,
    pub kind: u32,
    pub is_static: bool,
    pub parameters: Vec<VariableType>,
    pub required: usize,
    pub result: VariableType,
    pub rank: u8,
}

#[derive(Clone, Debug)]
pub struct TypeImport {
    pub name: String,
    pub kind: u32,
    pub members: BTreeMap<usize, MemberImport>,
}

#[derive(Clone, Debug, Default)]
pub struct HostCatalog {
    pub types: BTreeMap<u32, TypeImport>,
}

fn invalid() -> ContainerError {
    ContainerError::Invalid("host import contract")
}

impl HostCatalog {
    pub fn from_registry(registry: &UserTypeRegistry) -> Self {
        let mut types = BTreeMap::new();
        for (name, typ) in &registry.registered_types {
            let VariableType::UserData(id) = typ else { continue };
            let mut members = BTreeMap::new();
            let kind;
            if let Some(record) = registry.get_record_type_from_id(*id) {
                kind = 2;
                for (index, (name, field)) in record.fields.iter().enumerate() {
                    members.insert(
                        index,
                        MemberImport {
                            name: name.to_string(),
                            kind: 0,
                            is_static: false,
                            parameters: Vec::new(),
                            required: 0,
                            result: field.variable_type,
                            rank: field.dim,
                        },
                    );
                }
            } else if let Some(object) = registry.get_type_from_id(*id) {
                kind = 1;
                for (index, member) in object.id_table.iter().enumerate() {
                    let (name, member_kind, parameters, required, result, rank) = match member {
                        UserDataEntry::Field(name) | UserDataEntry::Getter(name) => (
                            name,
                            u32::from(matches!(member, UserDataEntry::Getter(_))),
                            Vec::new(),
                            0,
                            object.fields[name],
                            object.field_ranks.get(name).copied().unwrap_or(0),
                        ),
                        UserDataEntry::Procedure(name) => {
                            let value = &object.procedures[name];
                            (name, 2, value.parameters.clone(), value.required, VariableType::None, 0)
                        }
                        UserDataEntry::Function(name) => {
                            let value = &object.functions[name];
                            (name, 3, value.parameters.clone(), value.required, value.return_type, value.return_rank)
                        }
                    };
                    members.insert(
                        index,
                        MemberImport {
                            name: name.to_string(),
                            kind: member_kind,
                            is_static: object.statics.contains(name),
                            parameters,
                            required,
                            result,
                            rank,
                        },
                    );
                }
            } else {
                continue;
            }
            types.insert(
                *id,
                TypeImport {
                    name: format!("icy_board.{name}"),
                    kind,
                    members,
                },
            );
        }
        for definition in registry.enums().into_iter().take(crate::parser::BUILTIN_ENUM_COUNT) {
            types.insert(
                definition.id,
                TypeImport {
                    name: format!("icy_board.{}", definition.name),
                    kind: 3,
                    members: BTreeMap::new(),
                },
            );
        }
        Self { types }
    }

    pub(super) fn encode(&self, used: &BTreeSet<(u32, usize)>) -> Result<Section> {
        let mut output = Vec::new();
        word(&mut output, 1)?;
        for (&id, typ) in &self.types {
            word(&mut output, id as usize)?;
            word(&mut output, typ.kind as usize)?;
            blob(&mut output, typ.name.as_bytes())?;
            let members: Vec<_> = typ.members.iter().filter(|(member, _)| used.contains(&(id, **member))).collect();
            word(&mut output, members.len())?;
            for (&id, member) in members {
                word(&mut output, id)?;
                blob(&mut output, member.name.as_bytes())?;
                word(&mut output, member.kind as usize)?;
                word(&mut output, usize::from(member.is_static))?;
                word(&mut output, u32::from(member.result) as usize)?;
                word(&mut output, member.rank as usize)?;
                word(&mut output, member.required)?;
                word(&mut output, member.parameters.len())?;
                for parameter in &member.parameters {
                    word(&mut output, u32::from(*parameter) as usize)?;
                }
            }
        }
        Ok(Section::new(*b"IMPT", self.types.len() as u32, output))
    }

    pub(super) fn decode(section: &Section) -> Result<Self> {
        let mut input = Reader::new(&section.data);
        if input.word()? != 1 {
            return Err(invalid());
        }
        let mut result = Self::default();
        let mut names = BTreeSet::new();
        let typ = |id| match id {
            u32::MAX => VariableType::None,
            0..=24 => VariableType::from(id as u8),
            _ => VariableType::UserData(id),
        };
        for _ in 0..section.entries {
            let id = input.word()?;
            let kind = input.word()?;
            let name = input.text()?;
            if !(1..=3).contains(&kind) || !names.insert(name.to_ascii_lowercase()) {
                return Err(invalid());
            }
            let count = input.count(32)?;
            let mut members = BTreeMap::new();
            let mut member_names = BTreeSet::new();
            for _ in 0..count {
                let id = input.word()? as usize;
                let name = input.text()?;
                let kind = input.word()?;
                let is_static = input.word()?;
                let result = typ(input.word()?);
                let rank = input.word()?;
                let required = input.word()? as usize;
                let count = input.count(4)?;
                if kind > 3 || is_static > 1 || rank > 3 || required > count || !member_names.insert(name.to_ascii_lowercase()) {
                    return Err(invalid());
                }
                let parameters = (0..count).map(|_| input.word().map(typ)).collect::<Result<Vec<_>>>()?;
                if members
                    .insert(
                        id,
                        MemberImport {
                            name,
                            kind,
                            is_static: is_static == 1,
                            parameters,
                            required,
                            result,
                            rank: rank as u8,
                        },
                    )
                    .is_some()
                {
                    return Err(invalid());
                }
            }
            if result.types.insert(id, TypeImport { name, kind, members }).is_some() {
                return Err(invalid());
            }
        }
        input.finish()?;
        Ok(result)
    }

    pub(super) fn bind(&self, current: &Self) -> Result<(HashMap<u32, u32>, BTreeMap<(u32, usize), usize>)> {
        let mut types = HashMap::new();
        let mut members = BTreeMap::new();
        for (&id, expected) in &self.types {
            let (&new_id, actual) = current
                .types
                .iter()
                .find(|(_, typ)| typ.name.eq_ignore_ascii_case(&expected.name))
                .ok_or_else(|| ContainerError::Unsupported(format!("host type {}", expected.name)))?;
            if expected.kind != actual.kind {
                return Err(invalid());
            }
            types.insert(id, new_id);
        }
        let remap = |typ: VariableType| match typ {
            VariableType::UserData(id) => types.get(&id).copied().map(VariableType::UserData).ok_or_else(invalid),
            _ => Ok(typ),
        };
        for (&id, expected) in &self.types {
            let actual = &current.types[&types[&id]];
            for (&member_id, expected) in &expected.members {
                let (&new_id, actual) = actual
                    .members
                    .iter()
                    .find(|(_, member)| member.name.eq_ignore_ascii_case(&expected.name))
                    .ok_or_else(|| ContainerError::Unsupported(format!("host member {}.{}", actual.name, expected.name)))?;
                let stored = expected.parameters.iter().copied().map(remap).collect::<Result<Vec<_>>>()?;
                // A newer host may append optional parameters and may ask for fewer of
                // them, but must not change what a stored call already passes. The other
                // direction stays a mismatch: a program cannot run on a host that is older
                // than the signature it was built against.
                let compatible = expected.kind == actual.kind
                    && expected.is_static == actual.is_static
                    && expected.rank == actual.rank
                    && remap(expected.result)? == actual.result
                    && actual.parameters.len() >= stored.len()
                    && actual.parameters[..stored.len()] == stored[..]
                    && actual.required <= expected.required;
                if !compatible {
                    return Err(ContainerError::Unsupported(format!("host signature {}", expected.name)));
                }
                members.insert((id, member_id), new_id);
            }
        }
        Ok((types, members))
    }

    pub(super) fn rewrite(
        &self,
        script: &mut PPEScript,
        table: &VariableTable,
        records: &[Vec<RecordField>],
        types: &HashMap<u32, u32>,
        members: &BTreeMap<(u32, usize), usize>,
    ) -> Result<(BTreeSet<(u32, usize)>, Vec<super::TableEntry>)> {
        struct Rewrite<'a> {
            catalog: &'a HostCatalog,
            table: &'a VariableTable,
            records: &'a [Vec<RecordField>],
            types: &'a HashMap<u32, u32>,
            members: &'a BTreeMap<(u32, usize), usize>,
            used: BTreeSet<(u32, usize)>,
            type_constants: Vec<super::TableEntry>,
            constant_ids: BTreeMap<usize, usize>,
        }
        impl Rewrite<'_> {
            fn member(&mut self, receiver: VariableType, id: &mut usize) -> Result<VariableType> {
                let VariableType::UserData(type_id) = receiver else {
                    return Err(invalid());
                };
                if let Some(typ) = self.catalog.types.get(&type_id) {
                    let member = typ.members.get(id).ok_or_else(invalid)?;
                    self.used.insert((type_id, *id));
                    if let Some(new_id) = self.members.get(&(type_id, *id)) {
                        *id = *new_id;
                    }
                    Ok(member.result)
                } else {
                    self.records
                        .get((type_id as usize).checked_sub(FIRST_USER_TYPE_ID).ok_or_else(invalid)?)
                        .and_then(|fields| fields.get(*id))
                        .map(|field| field.variable_type)
                        .ok_or_else(invalid)
                }
            }

            fn args(&mut self, args: &mut [PPEExpr]) -> Result<()> {
                for arg in args {
                    self.expr(arg)?;
                }
                Ok(())
            }

            fn expr(&mut self, value: &mut PPEExpr) -> Result<VariableType> {
                Ok(match value {
                    PPEExpr::Value(id) | PPEExpr::Dim(id, _) => {
                        let result = self.table.try_get_entry(*id).ok_or_else(invalid)?.header.variable_type;
                        if let PPEExpr::Dim(_, args) = value {
                            self.args(args)?;
                        }
                        result
                    }
                    PPEExpr::RoutineReference(id) => self.table.try_get_entry(*id).ok_or_else(invalid)?.header.variable_type,
                    PPEExpr::RecordLiteral(id, fields) => {
                        for (_, value) in fields {
                            self.expr(value)?;
                        }
                        VariableType::UserData(*id)
                    }
                    PPEExpr::Member(base, id) | PPEExpr::IndexedMember(base, id, _) => {
                        let receiver = self.expr(base)?;
                        let result = self.member(receiver, id)?;
                        if let PPEExpr::IndexedMember(_, _, args) = value {
                            self.args(args)?;
                        }
                        result
                    }
                    PPEExpr::MemberFunctionCall(base, args, id) => {
                        let receiver = if let PPEExpr::Member(inner, member) = base.as_mut() {
                            let receiver = self.expr(inner)?;
                            let method_reference = match receiver {
                                VariableType::UserData(type_id) => self
                                    .catalog
                                    .types
                                    .get(&type_id)
                                    .and_then(|typ| typ.members.get(member))
                                    .is_some_and(|member| member.kind >= 2),
                                _ => false,
                            };
                            if method_reference {
                                if *member != *id {
                                    return Err(invalid());
                                }
                                let result = self.member(receiver, id)?;
                                *member = *id;
                                result
                            } else {
                                let receiver = self.member(receiver, member)?;
                                self.member(receiver, id)?
                            }
                        } else {
                            let receiver = self.expr(base)?;
                            self.member(receiver, id)?
                        };
                        self.args(args)?;
                        receiver
                    }
                    PPEExpr::FunctionCall(id, args) => {
                        self.args(args)?;
                        let entry = self.table.try_get_entry(*id).ok_or_else(invalid)?;
                        let result = unsafe { entry.value.data.function_value.return_var };
                        self.table
                            .try_get_entry(result as usize)
                            .map(|entry| entry.header.variable_type)
                            .unwrap_or(VariableType::None)
                    }
                    PPEExpr::PredefinedFunctionCall(definition, args) => {
                        if matches!(definition.opcode, FuncOpCode::StaticReceiver | FuncOpCode::EnumCast | FuncOpCode::EnumHas) {
                            let Some(PPEExpr::Value(id)) = args.first() else {
                                return Err(invalid());
                            };
                            let constant_id = *id;
                            let constant = self.table.try_get_entry(constant_id).ok_or_else(invalid)?;
                            if constant.header.variable_type != VariableType::Integer || constant.header.dim != 0 {
                                return Err(invalid());
                            }
                            let type_id = constant.value.as_int() as u32;
                            let mapped = self.types.get(&type_id).copied().unwrap_or(type_id);
                            let result = if definition.opcode == FuncOpCode::EnumHas {
                                VariableType::Boolean
                            } else {
                                VariableType::UserData(type_id)
                            };
                            self.args(args)?;
                            if mapped != type_id {
                                let new_id = *self.constant_ids.entry(constant_id).or_insert_with(|| {
                                    let mut entry = constant.clone();
                                    entry.header.id = self.table.len() + self.type_constants.len() + 1;
                                    entry.value = super::VariableValue::new_int(mapped as i32);
                                    let id = entry.header.id;
                                    self.type_constants.push(entry);
                                    id
                                });
                                args[0] = PPEExpr::Value(new_id);
                            }
                            result
                        } else if matches!(
                            definition.opcode,
                            FuncOpCode::ArrayValueAt | FuncOpCode::ArrayValueAt2 | FuncOpCode::ArrayValueAt3
                        ) {
                            let result = self.expr(args.first_mut().ok_or_else(invalid)?)?;
                            self.args(&mut args[1..])?;
                            result
                        } else {
                            self.args(args)?;
                            match definition.return_type {
                                VariableType::UserData(id) => {
                                    VariableType::UserData(self.types.iter().find_map(|(&old, &new)| (new == id).then_some(old)).unwrap_or(id))
                                }
                                typ => typ,
                            }
                        }
                    }
                    PPEExpr::UnaryExpression(_, value) => self.expr(value)?,
                    PPEExpr::BinaryExpression(_, left, right) => {
                        self.expr(left)?;
                        self.expr(right)?;
                        VariableType::Boolean
                    }
                    PPEExpr::Invalid => return Err(invalid()),
                })
            }
        }
        let mut rewrite = Rewrite {
            catalog: self,
            table,
            records,
            types,
            members,
            used: BTreeSet::new(),
            type_constants: Vec::new(),
            constant_ids: BTreeMap::new(),
        };
        for statement in &mut script.statements {
            match &mut statement.command {
                PPECommand::IfNot(value, _) | PPECommand::MemberCall(value) | PPECommand::ForEach(_, value, _) => {
                    rewrite.expr(value)?;
                }
                PPECommand::Let(left, right) => {
                    rewrite.expr(left)?;
                    rewrite.expr(right)?;
                }
                PPECommand::ProcedureCall(_, args) | PPECommand::PredefinedCall(_, args) => rewrite.args(args)?,
                _ => {}
            }
        }
        Ok((rewrite.used, rewrite.type_constants))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn binds_reordered_types_and_members_without_enum_domains() {
        let catalog = HostCatalog::from_registry(&UserTypeRegistry::icy_board_registry());
        let used = catalog
            .types
            .iter()
            .flat_map(|(&id, typ)| typ.members.keys().map(move |&member| (id, member)))
            .collect();
        let frozen = catalog.encode(&used).unwrap();
        let mut current = catalog.clone();
        let old_ids: Vec<_> = current.types.iter().filter(|(_, typ)| typ.kind == 1).map(|(&id, _)| id).collect();
        let remap: HashMap<_, _> = old_ids.iter().copied().zip(old_ids.iter().rev().copied()).collect();
        current.types = current
            .types
            .into_iter()
            .map(|(id, mut typ)| {
                let count = typ.members.len();
                typ.members = typ
                    .members
                    .into_iter()
                    .map(|(id, mut member)| {
                        for typ in member.parameters.iter_mut().chain(std::iter::once(&mut member.result)) {
                            if let VariableType::UserData(id) = typ {
                                *id = remap.get(id).copied().unwrap_or(*id);
                            }
                        }
                        (count - id - 1, member)
                    })
                    .collect();
                (remap.get(&id).copied().unwrap_or(id), typ)
            })
            .collect();
        let loaded = HostCatalog::decode(&frozen).unwrap();
        let (types, members) = loaded.bind(&current).unwrap();
        assert!(types.iter().any(|(old, new)| old != new));
        assert!(members.iter().any(|((_, old), new)| old != new));
        let member = current.types.values_mut().find_map(|typ| typ.members.values_mut().next()).unwrap();
        member.rank = (member.rank + 1) % 4;
        assert!(loaded.bind(&current).is_err());
    }

    /// The catalog is frozen against removal and renaming, not against growth:
    /// appending an optional parameter must keep stored programs loadable.
    #[test]
    fn binds_against_a_host_that_gained_optional_parameters() {
        let catalog = HostCatalog::from_registry(&UserTypeRegistry::icy_board_registry());
        let used = catalog
            .types
            .iter()
            .flat_map(|(&id, typ)| typ.members.keys().map(move |&member| (id, member)))
            .collect();
        let loaded = HostCatalog::decode(&catalog.encode(&used).unwrap()).unwrap();

        let with_optional = |extra: usize, required: Option<usize>| {
            let mut current = catalog.clone();
            let member = current
                .types
                .values_mut()
                .find_map(|typ| typ.members.values_mut().find(|member| member.kind >= 2))
                .unwrap();
            for _ in 0..extra {
                member.parameters.push(VariableType::Integer);
            }
            if let Some(required) = required {
                member.required = required;
            }
            current
        };

        loaded.bind(&with_optional(1, None)).expect("one appended optional parameter");
        loaded.bind(&with_optional(3, None)).expect("several appended optional parameters");
        loaded.bind(&with_optional(1, Some(0))).expect("a host that asks for fewer parameters");

        // Requiring the appended parameter would break a stored call that omits it.
        let mut demanding = with_optional(1, None);
        let member = demanding
            .types
            .values_mut()
            .find_map(|typ| typ.members.values_mut().find(|member| member.kind >= 2))
            .unwrap();
        member.required = member.parameters.len();
        assert!(loaded.bind(&demanding).is_err(), "a newly required parameter must not bind");

        // A program built against the longer signature must not load on the older host.
        let longer = with_optional(1, None);
        let used = longer
            .types
            .iter()
            .flat_map(|(&id, typ)| typ.members.keys().map(move |&member| (id, member)))
            .collect();
        let newer = HostCatalog::decode(&longer.encode(&used).unwrap()).unwrap();
        assert!(newer.bind(&catalog).is_err(), "compatibility only relaxes towards newer hosts");

        // Changing a parameter a stored call already passes stays a mismatch.
        let mut changed = catalog.clone();
        let member = changed
            .types
            .values_mut()
            .find_map(|typ| typ.members.values_mut().find(|member| member.kind >= 2 && !member.parameters.is_empty()))
            .unwrap();
        member.parameters[0] = VariableType::Double;
        assert!(loaded.bind(&changed).is_err());
    }
}
