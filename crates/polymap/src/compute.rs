use super::*;
use rand::Rng;
use std::collections::HashSet;

pub struct CornerPicker;

impl CornerPicker {
    pub fn random(poly_map: &PolyMap, rng: &mut impl Rng) -> CornerId {
        CornerId(rng.gen_range(0..poly_map.corners.len()))
    }
}

#[derive(Clone)]
pub struct CornerData<T> {
    pub data: Vec<T>,
}

impl<T> CornerData<T> {

    pub fn empty_shell() -> Self {
        Self { data: vec![] }
    }

    pub fn for_each(poly_map: &PolyMap, mut f: impl FnMut(CornerId, &Corner) -> T) -> Self {
        Self {
            data: poly_map.corners().map(|(id, c)| f(id, c)).collect(),
        }
    }

    pub fn update_each(
        &mut self,
        poly_map: &PolyMap,
        mut f: impl FnMut(CornerId, &Corner, &mut T),
    ) {
        for ((corner_id, corner), data) in poly_map.corners().zip(self.data.iter_mut()) {
            f(corner_id, corner, data)
        }
    }

    pub fn spread<U>(
        &mut self,
        poly_map: &PolyMap,
        starting: CornerId,
        mut accum: U,
        mut next: impl FnMut(U) -> Option<U>,
        mut update: impl FnMut(CornerId, &mut T, &U),
    ) {
        let mut visited = HashSet::new();
        let mut queue = vec![starting];
        let mut next_iteration = vec![];

        visited.insert(starting);

        loop {
            while !queue.is_empty() {
                let node = queue.pop().unwrap();
                update(node, &mut self.data[node.0], &accum);

                for &neighbor in poly_map.corner(node).neighbors() {
                    if !visited.contains(&neighbor) {
                        next_iteration.push(neighbor);
                        visited.insert(neighbor);
                    }
                }
            }
            if next_iteration.is_empty() {
                return;
            }
            accum = match next(accum) {
                None => return,
                Some(x) => x,
            };
            std::mem::swap(&mut queue, &mut next_iteration)
        }
    }

    pub fn ordered_by(
        &self,
        mut compare: impl FnMut(&T, &T) -> std::cmp::Ordering,
    ) -> Vec<CornerId> {
        let mut temporary: Vec<_> = (0..self.data.len()).map(CornerId).collect();
        temporary.sort_by(|&id1, &id2| {
            let t1 = &self.data[id1.0];
            let t2 = &self.data[id2.0];
            let ord = compare(t1, t2);
            ord.then_with(|| id1.cmp(&id2))
        });
        temporary
    }
}

impl<T: Clone> CornerData<T> {
    pub fn update_with_neighbors(&mut self, poly_map: &PolyMap, mut f: impl FnMut(&mut T, &[&T])) {
        let mut buf = vec![];
        let read_data = self.data.clone();
        for (id, corner) in poly_map.corners() {
            buf.clear();
            for &neighbor in corner.neighbors() {
                buf.push(&read_data[neighbor.0]);
            }
            f(&mut self.data[id.0], buf.as_slice())
        }
    }

    pub fn flow(
        &mut self,
        order: impl Iterator<Item = (CornerId, CornerId)>,
        mut update: impl FnMut(&mut T, &T),
    ) {
        for (from, to) in order {
            let source = &self.data[from.0].clone();
            let value = &mut self.data[to.0];
            update(value, source)
        }
    }
}

impl CornerData<f64> {
    pub fn max(&self) -> f64 {
        self.data.iter().copied().reduce(f64::max).unwrap()
    }

    pub fn min(&self) -> f64 {
        self.data.iter().copied().reduce(f64::min).unwrap()
    }
}

impl<T> std::ops::Index<CornerId> for CornerData<T> {
    type Output = T;
    fn index(&self, index: CornerId) -> &Self::Output {
        &self.data[index.0]
    }
}
impl<T> std::ops::IndexMut<CornerId> for CornerData<T> {
    fn index_mut(&mut self, index: CornerId) -> &mut Self::Output {
        &mut self.data[index.0]
    }
}

#[derive(Clone)]
pub struct EdgeData<T> {
    pub data: Vec<T>,
}

impl<T> EdgeData<T> {
    pub fn for_each(poly_map: &PolyMap, mut combine: impl FnMut(EdgeId, &Edge) -> T) -> Self {
        let data: Vec<_> = poly_map
            .edges()
            .map(|(id, edge)| combine(id, edge))
            .collect();

        Self { data }
    }

    pub fn from_corners_data<U>(
        poly_map: &PolyMap,
        corners_data: &CornerData<U>,
        mut combine: impl FnMut(EdgeId, &Edge, &U, &U) -> T,
    ) -> Self {
        let data: Vec<_> = poly_map
            .edges()
            .map(|(id, edge)| {
                let c1 = &corners_data[edge.endpoints.min];
                let c2 = &corners_data[edge.endpoints.max];
                combine(id, edge, c1, c2)
            })
            .collect();

        Self { data }
    }

    pub fn from_cell_data<U>(
        poly_map: &PolyMap,
        cell_data: &CellData<U>,
        mut combine: impl FnMut(EdgeId, &Edge, &[&U]) -> T,
    ) -> Self {
        let mut buf = vec![];
        let data: Vec<_> = poly_map
            .edges()
            .map(|(id, edge)| {
                buf.clear();
                for &cell in edge.cells() {
                    buf.push(&cell_data[cell]);
                }

                combine(id, edge, buf.as_slice())
            })
            .collect();

        Self { data }
    }
}

impl<T> std::ops::Index<EdgeId> for EdgeData<T> {
    type Output = T;
    fn index(&self, index: EdgeId) -> &Self::Output {
        &self.data[index.0]
    }
}
impl<T> std::ops::IndexMut<EdgeId> for EdgeData<T> {
    fn index_mut(&mut self, index: EdgeId) -> &mut Self::Output {
        &mut self.data[index.0]
    }
}

#[derive(Clone)]
pub struct CellData<T> {
    pub data: Vec<T>,
}

impl<T> CellData<T> {
    pub fn for_each(poly_map: &PolyMap, mut f: impl FnMut(CellId, &Cell) -> T) -> Self {
        Self {
            data: poly_map.cells().map(|(id, cell)| f(id, cell)).collect(),
        }
    }

    pub fn from_corners_data<U>(
        poly_map: &PolyMap,
        corners_data: &CornerData<U>,
        mut f: impl FnMut(CellId, &Cell, &[(CornerId, &U)]) -> T,
    ) -> Self {
        let mut buf = Vec::with_capacity(10);

        Self {
            data: poly_map
                .cells()
                .map(|(id, cell)| {
                    buf.clear();
                    // Extract corner data
                    cell.corners().iter().for_each(|&corner_id| {
                        let corner_data = &corners_data.data[corner_id.0];
                        buf.push((corner_id, corner_data));
                    });
                    f(id, cell, buf.as_slice())
                })
                .collect(),
        }
    }

    pub fn transform<U>(&self, mut f: impl FnMut(CellId, &T) -> U) -> CellData<U> {
        CellData {
            data: self
                .data
                .iter()
                .enumerate()
                .map(|(idx, t)| f(CellId(idx), t))
                .collect(),
        }
    }

    pub fn find_with_all_neighbors<'a>(
        &'a self,
        poly_map: &'a PolyMap,
        mut f: impl FnMut(CellId, &T) -> bool + 'a,
    ) -> impl Iterator<Item = CellId> + 'a {
        poly_map.cells().filter_map(move |(idx, cell)| {
            let mut value = true;
            for &neighbor_id in cell.neighbors() {
                if !f(neighbor_id, &self.data[neighbor_id.0]) {
                    value = false;
                    break;
                }
            }
            if value {
                Some(idx)
            } else {
                None
            }
        })
    }
}

impl CellData<f64> {
    pub fn corner_average(poly_map: &PolyMap, corners: &CornerData<f64>) -> Self {
        CellData::from_corners_data(&poly_map, &corners, |_, _, c_data| {
            let total: f64 = c_data.iter().map(|(_, v)| **v).sum();
            let n = c_data.len() as f64;
            total / n
        })
    }
}

impl<T> std::ops::Index<CellId> for CellData<T> {
    type Output = T;
    fn index(&self, index: CellId) -> &Self::Output {
        &self.data[index.0]
    }
}
impl<T> std::ops::IndexMut<CellId> for CellData<T> {
    fn index_mut(&mut self, index: CellId) -> &mut Self::Output {
        &mut self.data[index.0]
    }
}
