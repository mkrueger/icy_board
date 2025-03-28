use std::collections::HashMap;

use crate::ast::AstVisitor;

#[derive(Default)]
pub struct RenameScanVisitor {
    pub rename_map: HashMap<unicase::Ascii<String>, unicase::Ascii<String>>,
    cur_index_var: usize,
    _file_names: usize,
    _x_coords: usize,
    _y_coords: usize,
}

const INDEX_VARS: [&str; 4] = ["i", "j", "k", "l"];

impl AstVisitor<()> for RenameScanVisitor {
    fn visit_for_statement(&mut self, for_stmt: &crate::ast::ForStatement) {
        let var_name = for_stmt.get_identifier();
        if !self.rename_map.contains_key(var_name) && self.cur_index_var < INDEX_VARS.len() {
            self.rename_map
                .insert(var_name.clone(), unicase::Ascii::new(INDEX_VARS[self.cur_index_var].to_string()));
            self.cur_index_var += 1;
        }
        crate::ast::walk_for_stmt(self, for_stmt);
    }

    fn visit_predefined_call_statement(&mut self, _call: &crate::ast::PredefinedCallStatement) {
        /*
        match &call.get_func().opcode {
            OpCode::ANSIPOS => {
                if let Expression::Identifier(id) = &call.get_arguments()[0] {
                    let var_name = id.get_identifier();
                    if !self.rename_map.contains_key(var_name) {
                        self.x_coords += 1;
                        self.rename_map.insert(var_name.clone(), unicase::Ascii::new(format!("X{}", self.x_coords)));
                    }
                }
                if let Expression::Identifier(id) = &call.get_arguments()[1] {
                    let var_name = id.get_identifier();
                    if !self.rename_map.contains_key(var_name) {
                        self.y_coords += 1;
                        self.rename_map.insert(var_name.clone(), unicase::Ascii::new(format!("Y{}", self.y_coords)));
                    }
                }
            }
            OpCode::FOPEN | OpCode::DELETE | OpCode::DISPFILE => {
                if let Expression::Identifier(id) = &call.get_arguments()[0] {
                    let var_name = id.get_identifier();
                    if !self.rename_map.contains_key(var_name) {
                        self.file_names += 1;
                        self.rename_map
                            .insert(var_name.clone(), unicase::Ascii::new(format!("fileName{}", self.file_names)));
                    }
                }
            }
            _ => {}
        }*/
    }
}
