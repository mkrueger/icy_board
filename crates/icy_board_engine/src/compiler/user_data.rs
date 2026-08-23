use std::collections::HashMap;

use async_trait::async_trait;

use crate::{
    Res,
    executable::{GenericVariableData, VariableData, VariableType, VariableValue},
};

pub trait UserDataMemberRegistry {
    fn get_member_id(&self, name: &unicase::Ascii<String>) -> Option<usize>;

    fn add_property(&mut self, name: unicase::Ascii<String>, var_type: VariableType, has_setter: bool);

    /// `required` is how many leading parameters a caller may not leave out.
    fn add_procedure_with(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>, required: usize);
    fn add_function_with(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>, required: usize, return_type: VariableType);

    fn add_procedure(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>) {
        let required = parameters.len();
        self.add_procedure_with(name, parameters, required);
    }

    fn add_function(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>, return_type: VariableType) {
        let required = parameters.len();
        self.add_function_with(name, parameters, required, return_type);
    }
}

pub trait UserData: Sized + UserDataValue {
    const TYPE_NAME: &'static str;

    /// Adds custom fields specific to this userdata.
    fn register_members<F: UserDataMemberRegistry>(registry: &mut F);
}

/// The value a member answers with when it hands back another object. The object
/// lives as long as the values that name it, so nothing has to free it.
pub fn user_data_value<T: UserDataValue + 'static>(value: T, type_id: usize) -> VariableValue {
    VariableValue {
        data: VariableData::default(),
        generic_data: GenericVariableData::UserData(std::sync::Arc::new(value)),
        vtype: VariableType::UserData(type_id as u8),
    }
}

#[async_trait(?Send)]
pub trait UserDataValue: Send + Sync {
    fn get_property_value(&self, vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> Res<VariableValue>;
    fn set_property_value(&mut self, vm: &mut crate::vm::VirtualMachine, name: &unicase::Ascii<String>, val: VariableValue) -> crate::Res<()>;

    async fn call_function(
        &self,
        vm: &mut crate::vm::VirtualMachine<'_>,
        name: &unicase::Ascii<String>,
        arguments: &[VariableValue],
    ) -> crate::Res<VariableValue>;
    async fn call_method(&mut self, vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, arguments: &[VariableValue]) -> crate::Res<()>;
}

pub enum UserDataEntry {
    Field(unicase::Ascii<String>),
    Getter(unicase::Ascii<String>),
    Procedure(unicase::Ascii<String>),
    Function(unicase::Ascii<String>),
}

pub struct MemberFunction {
    pub parameters: Vec<VariableType>,
    pub required: usize,
    pub return_type: VariableType,
}

pub struct MemberProcedure {
    pub parameters: Vec<VariableType>,
    pub required: usize,
}

#[derive(Default)]
pub struct UserDataRegistry {
    pub id_table: Vec<UserDataEntry>,
    pub member_id_lookup: HashMap<unicase::Ascii<String>, usize>,

    pub fields: HashMap<unicase::Ascii<String>, VariableType>,
    pub procedures: HashMap<unicase::Ascii<String>, MemberProcedure>,
    pub functions: HashMap<unicase::Ascii<String>, MemberFunction>,
}

impl UserDataMemberRegistry for UserDataRegistry {
    fn get_member_id(&self, name: &unicase::Ascii<String>) -> Option<usize> {
        self.member_id_lookup.get(name).copied()
    }

    fn add_property(&mut self, name: unicase::Ascii<String>, var_type: VariableType, has_setter: bool) {
        self.member_id_lookup.insert(name.clone(), self.id_table.len());
        if has_setter {
            self.id_table.push(UserDataEntry::Field(name.clone()));
        } else {
            self.id_table.push(UserDataEntry::Getter(name.clone()));
        }
        self.fields.insert(name, var_type);
    }

    fn add_procedure_with(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>, required: usize) {
        self.member_id_lookup.insert(name.clone(), self.id_table.len());
        self.id_table.push(UserDataEntry::Procedure(name.clone()));
        self.procedures.insert(name, MemberProcedure { parameters, required });
    }

    fn add_function_with(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>, required: usize, return_type: VariableType) {
        self.member_id_lookup.insert(name.clone(), self.id_table.len());
        self.id_table.push(UserDataEntry::Function(name.clone()));
        self.functions.insert(
            name,
            MemberFunction {
                parameters,
                required,
                return_type,
            },
        );
    }
}
