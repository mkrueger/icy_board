use crate::{
    ast::{Expression, ParameterSpecifier},
    compiler::CompilationWarningType,
    hir::CallId,
};

use super::{SemanticInfo, SemanticVisitor};

#[derive(PartialEq)]
enum StorageStep {
    Member(unicase::Ascii<String>),
    Indices(Vec<Option<i32>>),
}

impl SemanticVisitor {
    fn storage_indices(&self, arguments: &[Expression]) -> StorageStep {
        StorageStep::Indices(
            arguments
                .iter()
                .map(|argument| self.enum_constant_value(argument).map(|value| value.as_int()))
                .collect(),
        )
    }

    fn storage_path(&mut self, expression: &Expression) -> Option<(usize, Vec<StorageStep>)> {
        match expression {
            Expression::Identifier(identifier) => Some((self.lookup_variable(identifier.get_identifier())?, Vec::new())),
            Expression::Parens(parens) => self.storage_path(parens.get_expression()),
            Expression::Indexer(indexer) => Some((
                self.lookup_variable(indexer.get_identifier())?,
                vec![self.storage_indices(indexer.get_arguments())],
            )),
            Expression::MemberReference(member) => {
                let (root, mut path) = self.storage_path(member.get_expression())?;
                path.push(StorageStep::Member(member.get_identifier().clone()));
                Some((root, path))
            }
            Expression::FunctionCall(call) => {
                let base = if matches!(self.function_type_lookup.get(&CallId(call.id)), Some(SemanticInfo::ArrayValueAt)) {
                    let Expression::MemberReference(member) = call.get_expression() else {
                        return None;
                    };
                    member.get_expression()
                } else {
                    call.get_expression()
                };
                let (root, mut path) = self.storage_path(base)?;
                path.push(self.storage_indices(call.get_arguments()));
                Some((root, path))
            }
            _ => None,
        }
    }

    pub(super) fn check_var_aliases(&mut self, parameters: &[ParameterSpecifier], arguments: &[Expression]) {
        if self.lang_version < 400 {
            return;
        }
        let mut targets: Vec<(usize, usize, Vec<StorageStep>)> = Vec::new();
        for (index, (parameter, argument)) in parameters.iter().zip(arguments).enumerate() {
            if !parameter.is_var() || !self.is_variable_argument(argument) {
                continue;
            }
            let Some((root, path)) = self.storage_path(argument) else {
                continue;
            };
            for (previous, previous_root, previous_path) in &targets {
                let overlap = root == *previous_root
                    && path.iter().zip(previous_path).all(|(left, right)| match (left, right) {
                        (StorageStep::Member(left), StorageStep::Member(right)) => left == right,
                        (StorageStep::Indices(left), StorageStep::Indices(right)) => {
                            left.len() == right.len() && left.iter().zip(right).all(|(left, right)| left.is_some() && left == right)
                        }
                        _ => false,
                    });
                if overlap {
                    self.errors
                        .lock()
                        .unwrap()
                        .report_warning(argument.get_span(), CompilationWarningType::AliasedVarArguments(previous + 1, index + 1));
                }
            }
            targets.push((index, root, path));
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        compiler::{PPECompiler, workspace::Workspace},
        parser::{Encoding, ErrorReporter, UserTypeRegistry, parse_ast},
    };
    use std::{
        path::PathBuf,
        sync::{Arc, Mutex},
    };

    fn aliases(source: &str, language: u16) -> Vec<(usize, usize)> {
        let mut workspace = Workspace::default();
        workspace.set_default_language_version(Some(language));
        workspace.package.runtime = Some(400);
        let registry = UserTypeRegistry::icy_board_registry();
        let errors = Arc::new(Mutex::new(ErrorReporter::default()));
        let ast = parse_ast(PathBuf::from("aliases.pps"), errors.clone(), source, &registry, Encoding::Utf8, &workspace);
        let mut compiler = PPECompiler::new(&workspace, registry, errors.clone());
        compiler.compile(&[&ast]);
        let reporter = errors.lock().unwrap();
        assert!(
            !reporter.has_errors(),
            "{:?}",
            reporter.errors.iter().map(|error| error.error.to_string()).collect::<Vec<_>>()
        );
        reporter
            .warnings
            .iter()
            .filter_map(|warning| match warning.error.downcast_ref::<CompilationWarningType>() {
                Some(CompilationWarningType::AliasedVarArguments(first, second)) => Some((*first, *second)),
                _ => None,
            })
            .collect()
    }

    #[test]
    fn s3_alias_warnings_are_precise_and_language_gated() {
        for (arguments, expected) in [
            ("number, ((number))", 1),
            ("data[1], data(1)", 1),
            ("data[1 + 1], data[2]", 1),
            ("data[0], data[1]", 0),
            ("data[cursor], data[cursor]", 0),
            ("number, other", 0),
        ] {
            let source =
                format!("INTEGER number, other, cursor, data[2]\nChange({arguments})\nPROCEDURE Change(VAR INTEGER first, VAR INTEGER second)\nENDPROC\n");
            assert_eq!(aliases(&source, 400).len(), expected, "{arguments}");
        }
        let source = "INTEGER number\nChange(number, number)\nPROCEDURE Change(VAR INTEGER first, VAR INTEGER second)\nENDPROC\n";
        assert_eq!(aliases(source, 400), [(1, 2)]);
        assert!(aliases(source, 350).is_empty());
    }

    #[test]
    fn s3_alias_warnings_cover_records_arrays_and_callbacks() {
        for (arguments, first_type, second_type, expected) in [
            ("item, item.Number", "Record", "INTEGER", 1),
            ("item.Values, item.Values[0]", "INTEGER[]", "INTEGER", 1),
            ("item.Values[0], item.Values(0)", "INTEGER", "INTEGER", 1),
            ("item.Number, item.Other", "INTEGER", "INTEGER", 0),
        ] {
            let parameter = |name: &str, kind: &str| {
                if kind == "INTEGER[]" {
                    format!("VAR INTEGER {name}[]")
                } else {
                    format!("VAR {kind} {name}")
                }
            };
            let source = format!(
                "TYPE Record\n INTEGER Number\n INTEGER Other\n INTEGER Values[]\nENDTYPE\nRecord item\nChange({arguments})\nPROCEDURE Change({}, {})\nENDPROC\n",
                parameter("first", first_type),
                parameter("second", second_type)
            );
            assert_eq!(aliases(&source, 400).len(), expected, "{arguments}");
        }
        let source = "PROCEDURE Relay(PROCEDURE callback(VAR INTEGER first, VAR INTEGER second), VAR INTEGER value)\ncallback(value, value)\nENDPROC\n";
        assert_eq!(aliases(source, 400), [(1, 2)]);
        assert!(
            aliases(
                "INTEGER number\nChange(number, number)\nPROCEDURE Change(INTEGER first, VAR INTEGER second)\nENDPROC\n",
                400
            )
            .is_empty()
        );
    }
}
