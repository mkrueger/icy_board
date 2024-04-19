use crate::executable::VariableType;
use std::collections::HashMap;

pub struct UserData {
    pub name: String,
    pub fields: HashMap<String, VariableType>,
    pub functions: HashMap<String, (usize, VariableType)>,
    pub procedures: HashMap<String, usize>,
}
