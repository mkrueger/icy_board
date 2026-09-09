use crate::{
    compiler::CompilationErrorType,
    executable::VariableType,
    hir::SymbolId,
    parser::lexer::{Spanned, Token},
};

use super::{ReferenceType, References, SemanticVisitor};

impl SemanticVisitor {
    pub(super) fn add_declaration(&mut self, variable_type: VariableType, identifier_token: &Spanned<Token>) -> usize {
        let id = self.references.len();
        let reference_type = match variable_type {
            VariableType::Function => ReferenceType::Function(self.function_containers.len()),
            VariableType::Procedure => ReferenceType::Procedure(self.function_containers.len()),
            _ => ReferenceType::Variable(id),
        };

        self.references.push((
            reference_type,
            References {
                variable_type,
                variable_table_index: 0,
                implementation: None,
                header: None,
                return_types: vec![],
                declaration: Some((
                    self.current_file.clone(),
                    Spanned::new(identifier_token.token.to_string(), identifier_token.span.clone()),
                )),
                usages: vec![],
            },
        ));
        id
    }

    pub(super) fn add_reference(&mut self, reference_type: ReferenceType, variable_type: VariableType, identifier_token: &Spanned<Token>) {
        if let Some(index) = self.reference_index(&reference_type) {
            if self.references_are_reachable {
                self.reference_owners.entry(index).or_default().insert(self.cur_func_impl);
            }
            self.references[index].1.usages.push((
                self.current_file.clone(),
                Spanned::new(identifier_token.token.to_string(), identifier_token.span.clone()),
            ));
            return;
        }
        let index = self.references.len();
        self.references.push((
            reference_type.clone(),
            References {
                declaration: None,
                implementation: None,
                header: None,
                return_types: vec![],
                variable_type,
                variable_table_index: 0,
                usages: vec![(
                    self.current_file.clone(),
                    Spanned::new(identifier_token.token.to_string(), identifier_token.span.clone()),
                )],
            },
        ));
        self.index_reference(&reference_type, index);
        if self.references_are_reachable {
            self.reference_owners.entry(index).or_default().insert(self.cur_func_impl);
        }
    }

    fn reference_index(&self, reference_type: &ReferenceType) -> Option<usize> {
        match reference_type {
            ReferenceType::Label(id) => self.label_reference_lookup.get(id).copied(),
            ReferenceType::PredefinedFunc(opcode) => self.predefined_function_reference_lookup.get(&(*opcode as i16)).copied(),
            ReferenceType::PredefinedProc(opcode) => self.predefined_procedure_reference_lookup.get(&(*opcode as i16)).copied(),
            _ => None,
        }
    }

    fn index_reference(&mut self, reference_type: &ReferenceType, index: usize) {
        match reference_type {
            ReferenceType::Label(id) => {
                self.label_reference_lookup.insert(*id, index);
            }
            ReferenceType::PredefinedFunc(opcode) => {
                self.predefined_function_reference_lookup.insert(*opcode as i16, index);
            }
            ReferenceType::PredefinedProc(opcode) => {
                self.predefined_procedure_reference_lookup.insert(*opcode as i16, index);
            }
            _ => {}
        }
    }

    pub(super) fn add_label_usage(&mut self, label_token: &Spanned<Token>) {
        let Token::Identifier(identifier) = &label_token.token else {
            log::error!("Invalid label token {label_token:?}");
            return;
        };
        let index = if let Some(index) = self.label_lookup_table.get_mut(identifier) {
            *index
        } else {
            self.label_count += 1;
            self.label_lookup_table.insert(identifier.clone(), self.label_count);
            self.label_count
        };

        self.add_reference(ReferenceType::Label(index), VariableType::UserData(255), label_token);
    }

    pub(super) fn set_label_declaration(&mut self, label_token: &Spanned<Token>) {
        let Token::Label(identifier) = &label_token.token else {
            log::error!("Invalid label token {label_token:?}");
            return;
        };
        if *identifier == "~BEGIN~" {
            return;
        }

        let index = if let Some(index) = self.label_lookup_table.get_mut(identifier) {
            if let Some(reference) = self.label_reference_lookup.get(index).copied()
                && self.references[reference].1.declaration.is_some()
            {
                self.errors
                    .lock()
                    .unwrap()
                    .report_error(label_token.span.clone(), CompilationErrorType::LabelAlreadyDefined(identifier.to_string()));
                return;
            }
            *index
        } else {
            self.label_count += 1;
            self.label_lookup_table.insert(identifier.clone(), self.label_count);
            self.label_count
        };
        let reference_type = ReferenceType::Label(index);
        let span = label_token.span.start + 1..label_token.span.end;

        if let Some(reference) = self.label_reference_lookup.get(&index).copied() {
            self.references[reference].1.declaration = Some((self.current_file.clone(), Spanned::new(label_token.token.to_string(), span)));
            return;
        }

        let reference_index = self.references.len();
        self.references.push((
            reference_type,
            References {
                variable_type: VariableType::Integer,
                variable_table_index: 0,
                implementation: None,
                header: None,
                return_types: vec![],
                declaration: Some((self.current_file.clone(), Spanned::new(label_token.token.to_string(), span))),
                usages: vec![],
            },
        ));
        self.label_reference_lookup.insert(index, reference_index);
    }

    pub(super) fn add_reference_to(&mut self, identifier: &Spanned<Token>, index: usize) {
        if self.references_are_reachable {
            self.reference_owners.entry(index).or_default().insert(self.cur_func_impl);
            if matches!(self.references[index].0, ReferenceType::Function(_) | ReferenceType::Procedure(_)) {
                self.call_graph.add_call(self.cur_func_impl.map(SymbolId), SymbolId(index));
            }
        }
        self.references[index]
            .1
            .usages
            .push((self.current_file.clone(), Spanned::new(identifier.token.to_string(), identifier.span.clone())));
    }

    pub fn reference_is_live(&self, reference: usize) -> bool {
        self.reference_owners.get(&reference).is_some_and(|owners| {
            owners
                .iter()
                .any(|owner| owner.is_none_or(|routine| self.call_graph.is_reachable(SymbolId(routine))))
        })
    }

    pub fn routine_is_reachable(&self, reference: usize) -> bool {
        self.call_graph.is_reachable(SymbolId(reference))
    }
}
