use super::*;
use std::collections::HashSet;

#[derive(Default)]
pub struct ElementSet {
    pub cells: HashSet<CellId>,
    pub edges: HashSet<EdgeId>,
    pub corners: HashSet<CornerId>,
}

impl ElementSet {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn join(&mut self, other: &ElementSet) {
        join_set(&mut self.cells, &other.cells);
        join_set(&mut self.edges, &other.edges);
        join_set(&mut self.corners, &other.corners)
    }

    pub fn clear(&mut self) {
        self.cells.clear();
        self.edges.clear();
        self.corners.clear();
    }

    pub fn add_cell(&mut self, cell: CellId, poly_map: &PolyMap) {
        self.cells.insert(cell);
        let cell = &poly_map.cells[cell.0];
        for &edge_id in cell.edges.iter() {
            let edge = &poly_map.edges[edge_id.0];
            self.corners.insert(edge.endpoints.min);
            self.corners.insert(edge.endpoints.max);
            self.edges.insert(edge_id);
        }
    }
}

fn join_set<T: Copy + Eq + std::hash::Hash>(s1: &mut HashSet<T>, s2: &HashSet<T>) {
    for &x in s2 {
        s1.insert(x);
    }
}
