use std::collections::{HashMap, HashSet};

use crate::hir::SymbolId;

/// Routine reachability from top-level code; uncalled module exports are intentionally not roots.
#[derive(Debug, Default, Clone)]
pub struct CallGraph {
    roots: HashSet<SymbolId>,
    edges: HashMap<SymbolId, HashSet<SymbolId>>,
    reachable: HashSet<SymbolId>,
}

impl CallGraph {
    pub fn add_call(&mut self, caller: Option<SymbolId>, callee: SymbolId) {
        if let Some(caller) = caller {
            self.edges.entry(caller).or_default().insert(callee);
        } else {
            self.roots.insert(callee);
        }
    }

    pub fn finish(&mut self) {
        self.reachable.clear();
        let mut pending: Vec<_> = self.roots.iter().copied().collect();
        while let Some(routine) = pending.pop() {
            if !self.reachable.insert(routine) {
                continue;
            }
            if let Some(callees) = self.edges.get(&routine) {
                pending.extend(callees.iter().copied());
            }
        }
    }

    pub fn is_reachable(&self, routine: SymbolId) -> bool {
        self.reachable.contains(&routine)
    }

    pub fn callees(&self, routine: SymbolId) -> impl Iterator<Item = SymbolId> + '_ {
        self.edges.get(&routine).into_iter().flatten().copied()
    }

    pub fn reachable_routines(&self) -> impl Iterator<Item = SymbolId> + '_ {
        self.reachable.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::CallGraph;
    use crate::hir::SymbolId;

    #[test]
    fn reachability_starts_at_roots_and_follows_cycles() {
        let mut graph = CallGraph::default();
        graph.add_call(None, SymbolId(1));
        graph.add_call(Some(SymbolId(1)), SymbolId(2));
        graph.add_call(Some(SymbolId(2)), SymbolId(1));
        graph.add_call(Some(SymbolId(3)), SymbolId(4));
        graph.finish();

        assert!(graph.is_reachable(SymbolId(1)));
        assert!(graph.is_reachable(SymbolId(2)));
        assert!(!graph.is_reachable(SymbolId(3)));
        assert!(!graph.is_reachable(SymbolId(4)));
    }
}
