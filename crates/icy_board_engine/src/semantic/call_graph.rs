use std::collections::{HashMap, HashSet};

/// Routine reachability from top-level code; uncalled module exports are intentionally not roots.
#[derive(Debug, Default, Clone)]
pub struct CallGraph {
    roots: HashSet<usize>,
    edges: HashMap<usize, HashSet<usize>>,
    reachable: HashSet<usize>,
}

impl CallGraph {
    pub fn add_call(&mut self, caller: Option<usize>, callee: usize) {
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

    pub fn is_reachable(&self, routine: usize) -> bool {
        self.reachable.contains(&routine)
    }

    pub fn callees(&self, routine: usize) -> impl Iterator<Item = usize> + '_ {
        self.edges.get(&routine).into_iter().flatten().copied()
    }

    pub fn reachable_routines(&self) -> impl Iterator<Item = usize> + '_ {
        self.reachable.iter().copied()
    }
}

#[cfg(test)]
mod tests {
    use super::CallGraph;

    #[test]
    fn reachability_starts_at_roots_and_follows_cycles() {
        let mut graph = CallGraph::default();
        graph.add_call(None, 1);
        graph.add_call(Some(1), 2);
        graph.add_call(Some(2), 1);
        graph.add_call(Some(3), 4);
        graph.finish();

        assert!(graph.is_reachable(1));
        assert!(graph.is_reachable(2));
        assert!(!graph.is_reachable(3));
        assert!(!graph.is_reachable(4));
    }
}
