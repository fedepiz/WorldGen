use polymap::compute::*;
use polymap::*;

use crate::generators::GridGenerator;

pub(crate) struct HeightMapBuilder {
    vertices: VertexData<f64>,
}

impl GridGenerator for HeightMapBuilder {
    fn grid(&self) -> &VertexData<f64> {
        &self.vertices
    }

    fn grid_mut(&mut self) -> &mut VertexData<f64> {
        &mut self.vertices
    }
}

impl HeightMapBuilder {
    pub(crate) fn new(poly_map: &PolyMap, default: f64) -> Self {
        let vertices = VertexData::for_each(&poly_map, |_, _| default);
        Self { vertices }
    }

    pub fn planchon_darboux(&mut self, poly_map: &PolyMap) {
        let epsilon = 0.001;
        let h = &mut self.vertices;
        let mut new_h =
            VertexData::for_each(
                poly_map,
                |id, corner| {
                    if corner.is_border() {
                        h[id]
                    } else {
                        100.0
                    }
                },
            );

        let mut changed = true;
        while changed {
            changed = false;
            for (id, corner) in poly_map.vertices() {
                if new_h[id] == h[id] {
                    continue;
                }
                for &neighbor in corner.neighbors() {
                    if h[id] >= new_h[neighbor] + epsilon {
                        new_h[id] = h[id];
                        changed = true;
                        break;
                    }
                    let oh = new_h[neighbor] + epsilon;
                    if (new_h[id] > oh) && (oh > h[id]) {
                        new_h[id] = oh;
                        changed = true;
                    }
                }
            }
        }

        std::mem::swap(&mut new_h, h);
    }

    pub(super) fn build(mut self, poly_map: &PolyMap) -> HeightMap {
        self.normalize();

        let descent_vector = VertexData::for_each(poly_map, |id, corner| {
            let my_elevation = self.vertices[id];
            let mut slope: Option<Slope> = None;

            for &neighbor in corner.neighbors() {
                let neighbor_elevation = self.vertices[neighbor];
                let diff = my_elevation - neighbor_elevation;
                if diff > 0.0 {
                    let update = match slope {
                        None => true,
                        Some(slope) => slope.intensity < diff,
                    };
                    if update {
                        slope = Some(Slope {
                            towards: neighbor,
                            intensity: diff,
                        });
                    }
                }
            }
            slope
        });

        let cells: CellData<f64> = CellData::vertex_average(poly_map, &self.vertices);

        fn descending(x: &f64, y: &f64) -> std::cmp::Ordering {
            (if x < y {
                std::cmp::Ordering::Less
            } else if x == y {
                std::cmp::Ordering::Equal
            } else {
                std::cmp::Ordering::Greater
            })
            .reverse()
        }

        let downhill = self.vertices.ordered_by(descending);

        HeightMap {
            vertices: self.vertices,
            cells,
            descent_vector,
            downhill,
        }
    }
}
#[derive(Clone)]
pub struct HeightMap {
    vertices: VertexData<f64>,
    cells: CellData<f64>,
    descent_vector: VertexData<Option<Slope>>,
    downhill: Vec<VertexId>,
}

impl HeightMap {
    pub fn vertex_height(&self, id: VertexId) -> f64 {
        self.vertices[id]
    }

    pub fn cell_height(&self, id: CellId) -> f64 {
        self.cells[id]
    }

    /// True if there is a slope going from a to b
    pub fn is_descent(&self, top: VertexId, bottom: VertexId) -> bool {
        self.descent_vector[top]
            .as_ref()
            .map(|x| x.towards == bottom)
            .unwrap_or(false)
    }

    pub fn descent_vector(&self, id: VertexId) -> Option<&Slope> {
        self.descent_vector[id].as_ref()
    }

    pub fn edge_high_corner(&self, edge: &Edge) -> Option<VertexId> {
        let s = self.vertices[edge.start()];
        let e = self.vertices[edge.end()];
        if s > e {
            Some(edge.start())
        } else if e > s {
            Some(edge.end())
        } else {
            None
        }
    }

    pub fn edge_low_corner(&self, edge: &Edge) -> Option<VertexId> {
        let s = self.vertices[edge.start()];
        let e = self.vertices[edge.end()];
        if s < e {
            Some(edge.start())
        } else if e < s {
            Some(edge.end())
        } else {
            None
        }
    }

    /// Returns an iterator over pairs of corners a -> b, which follow the downhill slope
    /// of each vector. The paths are not joind though
    pub(crate) fn downhill_flow(&self) -> impl Iterator<Item = (VertexId, VertexId)> + '_ {
        self.downhill
            .iter()
            .copied()
            .filter_map(|from| self.descent_vector(from).map(|slope| (from, slope.towards)))
    }

    // Starting from the given corner, walks downhill, recording all corner touched
    pub(crate) fn downhill_path(&self, corner: VertexId) -> DownhillPath {
        DownhillPath {
            node: corner,
            hm: self,
        }
    }

    pub(crate) fn make_builder(&self) -> HeightMapBuilder {
        HeightMapBuilder {
            vertices: self.vertices.clone(),
        }
    }
}

pub(crate) struct DownhillPath<'a> {
    node: VertexId,
    hm: &'a HeightMap,
}

impl<'a> Iterator for DownhillPath<'a> {
    type Item = VertexId;

    fn next(&mut self) -> Option<VertexId> {
        let slope = self.hm.descent_vector(self.node);
        if let Some(slope) = slope {
            let mut next = slope.towards;
            std::mem::swap(&mut self.node, &mut next);
            Some(next)
        } else {
            None
        }
    }
}

#[derive(Clone, Copy)]
pub struct Slope {
    pub towards: VertexId,
    pub intensity: f64,
}
