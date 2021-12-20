use std::collections::HashSet;

use polymap::compute::*;
use polymap::*;

use crate::generators::GridGenerator;
use crate::{HeightMap, TerrainType};

pub(crate) struct HydrologyBuilder {
    corner_rainfall: CornerData<f64>,
}

impl HydrologyBuilder {
    pub fn new(poly_map: &PolyMap) -> Self {
        Self {
            corner_rainfall: CornerData::for_each(poly_map, |_, _| 0.0),
        }
    }

    pub fn scale_by_height(&mut self, poly_map: &PolyMap, hm: &HeightMap, coeff: f64) {
        self.corner_rainfall.update_each(poly_map, |id, _, h| {
            let height = hm.corner_height(id);
            *h += height * coeff
        })
    }

    pub fn build(
        self,
        poly_map: &PolyMap,
        height_map: &HeightMap,
        terrain: &CellData<TerrainType>,
        min_river_flux: f64,
    ) -> Hydrology {
        let mut hydrology = Hydrology::new(min_river_flux, self.corner_rainfall);
        hydrology.recompute(poly_map, height_map, terrain);
        hydrology
    }
}

impl GridGenerator for HydrologyBuilder {
    fn grid(&self) -> &CornerData<f64> {
        &self.corner_rainfall
    }

    fn grid_mut(&mut self) -> &mut CornerData<f64> {
        &mut self.corner_rainfall
    }
}

pub struct Hydrology {
    // Innate data
    min_river_flux: f64,
    corner_rainfall: CornerData<f64>,

    // Computed data
    cell_rainfall: CellData<f64>,
    corner_flux: CornerData<f64>,
    edge_flux: EdgeData<f64>,
    rivers: Rivers,
}

impl Hydrology {
    fn new(min_river_flux: f64, corner_rainfall: CornerData<f64>) -> Self {
        Self {
            min_river_flux,
            corner_rainfall,
            cell_rainfall: CellData::empty_shell(),
            corner_flux: CornerData::empty_shell(),
            edge_flux: EdgeData::empty_shell(),
            rivers: Rivers::new(),
        }
    }

    pub(crate) fn recompute(
        &mut self,
        poly_map: &PolyMap,
        height_map: &HeightMap,
        terrain: &CellData<TerrainType>,
    ) {
        self.cell_rainfall = CellData::corner_average(poly_map, &self.corner_rainfall);

        self.corner_flux = {
            let mut corner_flux = self.corner_rainfall.clone();
            corner_flux.flow(height_map.downhill_flow(), |x, y| {
                *x += *y;
            });
            corner_flux
        };

        self.edge_flux = EdgeData::for_each(poly_map, |_, edge| {
            let mut flux = 0.0;
            if height_map.is_descent(edge.start(), edge.end()) {
                flux += self.corner_flux[edge.start()]
            }
            if height_map.is_descent(edge.end(), edge.start()) {
                flux += self.corner_flux[edge.end()]
            }
            flux
        });

        self.rivers = Rivers::compute(
            poly_map,
            height_map,
            terrain,
            &self.edge_flux,
            self.min_river_flux,
        );
    }

    pub fn corner_flux(&self, corner: CornerId) -> f64 {
        self.corner_flux[corner]
    }

    pub fn edge_flux(&self, edge: EdgeId) -> f64 {
        self.edge_flux[edge]
    }

    pub fn cell_rainfall(&self, cell: CellId) -> f64 {
        self.cell_rainfall[cell]
    }

    pub fn rivers(&self) -> &Rivers {
        &self.rivers
    }
}

pub struct Rivers {
    edge_is_river: EdgeData<bool>,
    paths: Vec<RiverPath>,
}

impl Rivers {
    fn new() -> Self {
        Self {
            edge_is_river: EdgeData::empty_shell(),
            paths: vec![],
        }
    }

    fn compute(
        poly_map: &PolyMap,
        height_map: &HeightMap,
        terrain: &CellData<TerrainType>,
        edge_flux: &EdgeData<f64>,
        min_river_flux: f64,
    ) -> Self {
        let edge_is_river = EdgeData::from_cell_data(poly_map, &terrain, |id, _, terrain| {
            let is_water = terrain.iter().any(|tt| tt.is_water());
            !is_water && edge_flux[id] > min_river_flux
        });

        let mut river_sources = HashSet::new();
        for (id, edge) in poly_map.edges() {
            if edge_is_river[id] {
                if let Some(top) = height_map.edge_high_corner(edge) {
                    let is_source = poly_map
                        .corner(top)
                        .edges()
                        .iter()
                        .all(|&other_id| id == other_id || !edge_is_river[other_id]);
                    if is_source {
                        river_sources.insert(top);
                    }
                }
            }
        }

        let paths = river_sources
            .iter()
            .filter_map(|&source| {
                let path: Vec<_> = height_map
                    .downhill_path(source)
                    .take_while(|&corner| {
                        poly_map
                            .corner(corner)
                            .edges()
                            .iter()
                            .any(|&edge| edge_is_river[edge])
                    })
                    .collect();
                if path.is_empty() {
                    None
                } else {
                    Some(RiverPath { path })
                }
            })
            .collect();

        Self {
            edge_is_river,
            paths,
        }
    }

    pub fn is_segment(&self, edge: EdgeId) -> bool {
        self.edge_is_river[edge]
    }

    pub fn is_source(&self, corner: CornerId) -> bool {
        self.paths.iter().any(|path| path.source() == corner)
    }

    pub fn is_sink(&self, corner: CornerId) -> bool {
        self.paths.iter().any(|path| path.sink() == corner)
    }

    pub fn paths(&self) -> &[RiverPath] {
        self.paths.as_slice()
    }
}

pub struct RiverPath {
    path: Vec<CornerId>,
}

impl RiverPath {
    pub fn source(&self) -> CornerId {
        *self.path.first().unwrap()
    }
    pub fn sink(&self) -> CornerId {
        *self.path.last().unwrap()
    }
    pub fn corners(&self) -> &[CornerId] {
        &self.path
    }
}
