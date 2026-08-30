use std::collections::HashMap;

use async_trait::async_trait;

use crate::{
    Res,
    executable::{GenericVariableData, VariableData, VariableType, VariableValue},
};

pub trait UserDataMemberRegistry {
    fn get_member_id(&self, name: &unicase::Ascii<String>) -> Option<usize>;

    fn add_property(&mut self, name: unicase::Ascii<String>, var_type: VariableType, has_setter: bool);
    fn add_array_property(&mut self, name: unicase::Ascii<String>, var_type: VariableType, rank: u8);

    /// `required` is how many leading parameters a caller may not leave out.
    fn add_procedure_with(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>, required: usize);
    fn add_function_with(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>, required: usize, return_type: VariableType);
    fn add_array_function_with(
        &mut self,
        name: unicase::Ascii<String>,
        parameters: Vec<VariableType>,
        required: usize,
        return_type: VariableType,
        return_rank: u8,
    );

    /// A function called on the type rather than on one of its values, such as
    /// `Surface.New`. It is an ordinary member otherwise, so adding one needs
    /// nothing from the runtime.
    fn add_static_function_with(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>, required: usize, return_type: VariableType);

    fn set_parameter_names(&mut self, name: &unicase::Ascii<String>, names: Vec<String>);

    fn add_named_function_with(
        &mut self,
        name: unicase::Ascii<String>,
        parameters: Vec<(&str, VariableType)>,
        required: usize,
        return_type: VariableType,
    ) {
        let (names, types) = split_named_parameters(parameters);
        self.add_function_with(name.clone(), types, required, return_type);
        self.set_parameter_names(&name, names);
    }

    fn add_named_array_function_with(
        &mut self,
        name: unicase::Ascii<String>,
        parameters: Vec<(&str, VariableType)>,
        required: usize,
        return_type: VariableType,
        return_rank: u8,
    ) {
        let (names, types) = split_named_parameters(parameters);
        self.add_array_function_with(name.clone(), types, required, return_type, return_rank);
        self.set_parameter_names(&name, names);
    }

    fn add_named_static_function_with(
        &mut self,
        name: unicase::Ascii<String>,
        parameters: Vec<(&str, VariableType)>,
        required: usize,
        return_type: VariableType,
    ) {
        let (names, types) = split_named_parameters(parameters);
        self.add_static_function_with(name.clone(), types, required, return_type);
        self.set_parameter_names(&name, names);
    }

    fn add_named_function(&mut self, name: unicase::Ascii<String>, parameters: Vec<(&str, VariableType)>, return_type: VariableType) {
        let required = parameters.len();
        self.add_named_function_with(name, parameters, required, return_type);
    }

    fn add_named_static_function(&mut self, name: unicase::Ascii<String>, parameters: Vec<(&str, VariableType)>, return_type: VariableType) {
        let required = parameters.len();
        self.add_named_static_function_with(name, parameters, required, return_type);
    }

    fn add_procedure(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>) {
        let required = parameters.len();
        self.add_procedure_with(name, parameters, required);
    }

    fn add_function(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>, return_type: VariableType) {
        let required = parameters.len();
        self.add_function_with(name, parameters, required, return_type);
    }

    fn add_static_function(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>, return_type: VariableType) {
        let required = parameters.len();
        self.add_static_function_with(name, parameters, required, return_type);
    }
}

fn split_named_parameters(parameters: Vec<(&str, VariableType)>) -> (Vec<String>, Vec<VariableType>) {
    parameters.into_iter().map(|(name, var_type)| (name.to_string(), var_type)).unzip()
}

pub trait UserData: Sized + UserDataValue {
    const TYPE_NAME: &'static str;
    const EMPTY_VALUE: Option<fn() -> VariableValue> = None;

    /// The zero-argument builtin that hands back the one instance of this object, which is
    /// what lets `Terminal.Info` start at the caller's one `Terminal` value.
    const INSTANCE_PROVIDER: Option<crate::executable::FuncOpCode> = None;

    /// What a static member is called on. It carries no state of its own; it is only
    /// what gives the call somewhere to dispatch from.
    const STATIC_RECEIVER: Option<fn() -> VariableValue> = None;

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
    async fn set_property_value(&self, vm: &mut crate::vm::VirtualMachine<'_>, name: &unicase::Ascii<String>, val: VariableValue) -> crate::Res<()>;

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
    pub parameter_names: Vec<String>,
    pub required: usize,
    pub return_type: VariableType,
    pub return_rank: u8,
}

pub struct MemberProcedure {
    pub parameters: Vec<VariableType>,
    pub parameter_names: Vec<String>,
    pub required: usize,
}

#[derive(Default)]
pub struct UserDataRegistry {
    pub id_table: Vec<UserDataEntry>,
    pub member_id_lookup: HashMap<unicase::Ascii<String>, usize>,

    pub fields: HashMap<unicase::Ascii<String>, VariableType>,
    pub field_ranks: HashMap<unicase::Ascii<String>, u8>,
    pub procedures: HashMap<unicase::Ascii<String>, MemberProcedure>,
    pub functions: HashMap<unicase::Ascii<String>, MemberFunction>,

    /// The members that belong to the type rather than to one of its values.
    pub statics: std::collections::HashSet<unicase::Ascii<String>>,

    pub instance_provider: Option<crate::executable::FuncOpCode>,
    pub static_receiver: Option<fn() -> VariableValue>,
    pub empty_value: Option<fn() -> VariableValue>,
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

    fn add_array_property(&mut self, name: unicase::Ascii<String>, var_type: VariableType, rank: u8) {
        self.member_id_lookup.insert(name.clone(), self.id_table.len());
        self.id_table.push(UserDataEntry::Getter(name.clone()));
        self.field_ranks.insert(name.clone(), rank);
        self.fields.insert(name, var_type);
    }

    fn add_procedure_with(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>, required: usize) {
        self.member_id_lookup.insert(name.clone(), self.id_table.len());
        self.id_table.push(UserDataEntry::Procedure(name.clone()));
        self.procedures.insert(
            name,
            MemberProcedure {
                parameters,
                parameter_names: Vec::new(),
                required,
            },
        );
    }

    fn add_function_with(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>, required: usize, return_type: VariableType) {
        self.member_id_lookup.insert(name.clone(), self.id_table.len());
        self.id_table.push(UserDataEntry::Function(name.clone()));
        self.functions.insert(
            name,
            MemberFunction {
                parameters,
                parameter_names: Vec::new(),
                required,
                return_type,
                return_rank: 0,
            },
        );
    }

    fn add_array_function_with(
        &mut self,
        name: unicase::Ascii<String>,
        parameters: Vec<VariableType>,
        required: usize,
        return_type: VariableType,
        return_rank: u8,
    ) {
        self.member_id_lookup.insert(name.clone(), self.id_table.len());
        self.id_table.push(UserDataEntry::Function(name.clone()));
        self.functions.insert(
            name,
            MemberFunction {
                parameters,
                parameter_names: Vec::new(),
                required,
                return_type,
                return_rank,
            },
        );
    }

    fn add_static_function_with(&mut self, name: unicase::Ascii<String>, parameters: Vec<VariableType>, required: usize, return_type: VariableType) {
        self.statics.insert(name.clone());
        self.add_function_with(name, parameters, required, return_type);
    }

    fn set_parameter_names(&mut self, name: &unicase::Ascii<String>, names: Vec<String>) {
        if let Some(function) = self.functions.get_mut(name) {
            function.parameter_names = names;
        } else if let Some(procedure) = self.procedures.get_mut(name) {
            procedure.parameter_names = names;
        }
    }
}
