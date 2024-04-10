pub mod deserializer;
pub mod disassembler;
pub use deserializer::*;

pub mod commands;
pub use commands::*;

pub mod smt_op_codes;
pub use smt_op_codes::*;

pub mod func_op_codes;
pub use func_op_codes::*;

pub mod variable_value;
pub use variable_value::*;

pub mod variable_table;
pub use variable_table::*;

pub mod exec;
pub use exec::*;

#[cfg(test)]
pub mod expr_tests;
#[cfg(test)]
pub mod stmt_tests;

pub const LAST_PPLC: u16 = 340;

#[derive(Clone, Debug, Default)]
pub struct VariableNameGenerator {
    _version: u16,
    user_vars_version: u16,

    string_vars: usize,
    int_vars: usize,
    bool_vars: usize,
    money_vars: usize,
    byte_vars: usize,
    time_vars: usize,
    date_vars: usize,
    generic_vars: usize,

    function_vars: usize,
    procedure_vars: usize,
    constants: usize,
}

impl VariableNameGenerator {
    pub fn get_next_name(&mut self, decl: &TableEntry) -> (String, bool) {
        let var_idx = decl.header.id - 1;
        if var_idx < USER_VARIABLES.len()
            && self.user_vars_version >= USER_VARIABLES[var_idx].version
        {
            return (USER_VARIABLES[var_idx].name.to_string(), true);
        }

        if decl.get_type() == EntryType::Constant {
            self.constants += 1;
            return (format!("CONST{:>03}", self.constants), false);
        }

        let name = match decl.header.variable_type {
            VariableType::String => {
                self.string_vars += 1;
                format!("STR{:>03}", self.string_vars)
            }
            VariableType::BigStr => {
                self.string_vars += 1;
                format!("BSTR{:>03}", self.string_vars)
            }
            VariableType::Integer => {
                self.int_vars += 1;
                format!("INT{:>03}", self.int_vars)
            }
            VariableType::Boolean => {
                self.bool_vars += 1;
                format!("BOOL{:>03}", self.bool_vars)
            }
            VariableType::Byte => {
                self.byte_vars += 1;
                format!("BYTE{:>03}", self.byte_vars)
            }
            VariableType::Money => {
                self.money_vars += 1;
                format!("MONEY{:>03}", self.money_vars)
            }
            VariableType::Time => {
                self.time_vars += 1;
                format!("TIME{:>03}", self.time_vars)
            }
            VariableType::Date => {
                self.date_vars += 1;
                format!("DATE{:>03}", self.date_vars)
            }
            VariableType::Function => {
                self.function_vars += 1;
                format!("FUNC{:>03}", self.function_vars)
            }
            VariableType::Procedure => {
                self.procedure_vars += 1;
                format!("PROC{:>03}", self.procedure_vars)
            }
            _ => {
                self.generic_vars += 1;
                format!("VAR{:>03}", self.generic_vars)
            }
        };
        (name, false)
    }

    pub fn new(version: u16, user_vars_version: u16) -> Self {
        Self {
            _version: version,
            user_vars_version,
            ..Default::default()
        }
    }
}
