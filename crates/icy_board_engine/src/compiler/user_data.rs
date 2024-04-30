use std::collections::HashMap;

use async_trait::async_trait;

use crate::{
    executable::{PPEExpr, VariableType, VariableValue},
    Res,
};

pub trait UserDataMemberRegistry {
    fn get_member_id(&self, name: &unicase::Ascii<String>) -> Option<usize>;

    fn add_property(&mut self, name: unicase::Ascii<String>, var_type: VariableType, has_setter: bool);

    fn add_procedure(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>);
    fn add_function(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>, return_type: VariableType);
}

pub trait UserData: Sized + UserDataValue {
    const TYPE_NAME: &'static str;

    /// Adds custom fields specific to this userdata.
    fn register_members<F: UserDataMemberRegistry>(registry: &mut F);
}

#[async_trait]
pub trait UserDataValue: Send + Sync {
    fn get_property_value(&self, vm: &crate::vm::VirtualMachine, name: &unicase::Ascii<String>) -> Res<VariableValue>;
    fn set_property_value(&mut self, vm: &mut crate::vm::VirtualMachine, name: &unicase::Ascii<String>, val: VariableValue) -> crate::Res<()>;

    async fn call_function(&self, vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, arguments: &[PPEExpr]) -> crate::Res<VariableValue>;
    async fn call_method(&mut self, vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, arguments: &[PPEExpr]) -> crate::Res<()>;
}

pub enum UserDataEntry {
    Field(unicase::Ascii<String>),
    Getter(unicase::Ascii<String>),
    Procedure(unicase::Ascii<String>),
    Function(unicase::Ascii<String>),
}

#[derive(Default)]
pub struct UserDataRegistry {
    pub id_table: Vec<UserDataEntry>,
    pub member_id_lookup: HashMap<unicase::Ascii<String>, usize>,

    pub fields: HashMap<unicase::Ascii<String>, VariableType>,
    pub procedures: HashMap<unicase::Ascii<String>, Vec<VariableType>>,
    pub functions: HashMap<unicase::Ascii<String>, (Vec<VariableType>, VariableType)>,
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

    fn add_procedure(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>) {
        self.member_id_lookup.insert(name.clone(), self.id_table.len());
        self.id_table.push(UserDataEntry::Procedure(name.clone()));
        self.procedures.insert(name, parameters);
    }

    fn add_function(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>, return_type: VariableType) {
        self.member_id_lookup.insert(name.clone(), self.id_table.len());
        self.id_table.push(UserDataEntry::Function(name.clone()));
        self.functions.insert(name, (parameters, return_type));
    }
}
