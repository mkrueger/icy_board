//! Shared source-analysis boundary for the compiler and language server.
//!
//! Binding resolves module names and legacy declaration kinds, but does not
//! desugar control flow, substitute constants or optimize expressions. The
//! checked high-level representation retains that source structure and its
//! provenance alongside the authoritative semantic annotations.

use std::{
    collections::{HashMap, HashSet},
    sync::Arc,
};

use crate::{
    ast::Ast,
    executable::{FuncOpCode, VariableType},
};

use super::SemanticVisitor;

#[derive(Default)]
pub(crate) struct SourceAnnotations {
    pub user_types: HashMap<usize, u32>,
    pub receiver_types: HashMap<usize, VariableType>,
    pub instance_providers: HashMap<usize, FuncOpCode>,
    pub static_receivers: HashMap<usize, u32>,
    pub compound_targets: HashMap<usize, VariableType>,
    routine_references: HashSet<usize>,
    function_results: HashSet<usize>,
}

impl SourceAnnotations {
    fn take(visitor: &mut SemanticVisitor) -> Self {
        Self {
            user_types: std::mem::take(&mut visitor.user_type_lookup),
            receiver_types: std::mem::take(&mut visitor.member_receiver_type_lookup),
            instance_providers: std::mem::take(&mut visitor.instance_provider_lookup),
            static_receivers: std::mem::take(&mut visitor.static_receiver_lookup),
            compound_targets: std::mem::take(&mut visitor.compound_target_types),
            routine_references: std::mem::take(&mut visitor.allowed_routine_reference_spans),
            function_results: std::mem::take(&mut visitor.function_return_value_spans),
        }
    }

    pub(crate) fn restore(&self, visitor: &mut SemanticVisitor) {
        visitor.user_type_lookup.clone_from(&self.user_types);
        visitor.member_receiver_type_lookup.clone_from(&self.receiver_types);
        visitor.instance_provider_lookup.clone_from(&self.instance_providers);
        visitor.static_receiver_lookup.clone_from(&self.static_receivers);
        visitor.compound_target_types.clone_from(&self.compound_targets);
        visitor.allowed_routine_reference_spans.clone_from(&self.routine_references);
        visitor.function_return_value_spans.clone_from(&self.function_results);
    }
}

/// Source-shaped, bound and checked HIR. Only source analysis constructs it;
/// backend lowering consumes it. Invalid editor input still produces semantic
/// information, but cannot cross the executable-lowering boundary.
pub struct CheckedProgram {
    pub(crate) files: Vec<CheckedSource>,
    valid: bool,
}

pub(crate) struct CheckedSource {
    pub ast: Ast,
    pub annotations: Arc<SourceAnnotations>,
}

impl CheckedProgram {
    pub fn is_valid(&self) -> bool {
        self.valid
    }
}

impl SemanticVisitor {
    /// Perform the single authoritative source analysis, shared with the editor.
    /// Original ASTs remain untouched. Calls retain their parse-assigned IDs;
    /// member/receiver annotations are isolated by file rather than offset alone.
    pub fn analyze_sources(&mut self, sources: &[&Ast]) -> CheckedProgram {
        self.set_modules(sources);
        let bound = crate::compiler::modules::bind_sources(sources, self.errors.clone(), &self.type_registry);
        self.prepare_legacy_call_signatures(&bound.iter().collect::<Vec<_>>());
        self.source_annotations.clear();
        let mut files: Vec<_> = bound
            .into_iter()
            .map(|ast| CheckedSource {
                ast,
                annotations: Arc::default(),
            })
            .collect();
        for file in files.iter_mut().filter(|file| file.ast.module.is_some()) {
            self.set_file_name(&file.ast.file_name);
            file.ast.visit(self);
            file.annotations = Arc::new(SourceAnnotations::take(self));
            self.source_annotations.insert(file.ast.file_name.clone(), file.annotations.clone());
        }
        for file in files.iter_mut().filter(|file| file.ast.module.is_none()) {
            self.set_file_name(&file.ast.file_name);
            file.ast.visit(self);
            file.annotations = Arc::new(SourceAnnotations::take(self));
            self.source_annotations.insert(file.ast.file_name.clone(), file.annotations.clone());
        }
        self.finish();
        let valid = self.errors.lock().unwrap().errors.is_empty();
        CheckedProgram { files, valid }
    }

    /// Select file-local source annotations for editor queries. Offsets in two
    /// package files may coincide and must never share a lookup map.
    pub fn select_source_file(&mut self, file: &std::path::Path) {
        if let Some(annotations) = self.source_annotations.get(file).cloned() {
            annotations.restore(self);
        }
    }
}
