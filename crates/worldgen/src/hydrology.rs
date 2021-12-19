use std::{collections::HashSet};

use polymap::*;
use polymap::compute::*;

use crate::{HeightMap, TerrainType};

pub struct Hydrology {
    corner_flux: CornerData<f64>,
    edge_flux: EdgeData<f64>,
    rivers: Rivers
}

impl Hydrology {
    pub(super) fn new(poly_map: &PolyMap, height_map: &HeightMap, terrain:&CellData<TerrainType>, base_flow: f64) -> Hydrology {
        let corner_flux = {
            let mut corner_flux = CornerData::for_each(poly_map, |id, _| 
                height_map.corner_height(id) * base_flow);
          
            corner_flux.flow(height_map.downhill_flow(),|x, y| {
                *x += *y;
            });
            corner_flux
        };

        let edge_flux= EdgeData::for_each(poly_map, |_, edge| {
            let mut flux = 0.0;
            if height_map.is_descent(edge.start(), edge.end()) {
                flux += corner_flux[edge.start()]
            }
            if height_map.is_descent(edge.end(), edge.start()) {
                flux += corner_flux[edge.end()]
            }
            flux
        });

        let rivers = Rivers::new(poly_map, height_map, terrain, &edge_flux);
    
        Hydrology { corner_flux, edge_flux, rivers }
    }

    pub fn corner_flux(&self, corner: CornerId) -> f64 {
        self.corner_flux[corner]
    }

    pub fn edge_flux(&self, edge: EdgeId) -> f64 {
        self.edge_flux[edge]
    }

    pub fn rivers(&self) -> &Rivers {
        &self.rivers
    }
}

pub struct Rivers {
    edge_is_river: EdgeData<bool>,
    paths: Vec<RiverPath>
}

const MIN_RIVER: f64 = 75.0;

impl Rivers {
    fn new(poly_map: &PolyMap, height_map: &HeightMap, terrain:&CellData<TerrainType>, edge_flux:&EdgeData<f64>) -> Self {

        let edge_is_river = EdgeData::from_cell_data(poly_map, &terrain,
            |id,_,terrain| {
                let is_water = terrain.iter().any(|tt| tt.is_water());
                !is_water && edge_flux[id] > MIN_RIVER
            }
        );

        let mut river_sources= HashSet::new();
        for (id, edge) in poly_map.edges() {
            if edge_is_river[id] {
                if let Some(top) = height_map.edge_high_corner(edge) {   
                    let is_source = poly_map.corner(top).edges().iter().all(
                        |&other_id| id == other_id || !edge_is_river[other_id]
                    );
                    if is_source {
                        river_sources.insert(top);
                    }
                }
            }
        }

        let paths = river_sources.iter().filter_map(|&source| {
            let path:Vec<_> = height_map.downhill_path(source).take_while(|&corner| 
                poly_map.corner(corner).edges().iter().any(|&edge|edge_is_river[edge])
            ).collect();
            if path.is_empty() {
                None
            } else {
                Some(RiverPath { path })
            }
        }).collect();

        Self { edge_is_river, paths }
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
    path: Vec<CornerId>
}

impl RiverPath {
    pub fn source(&self) -> CornerId { *self.path.first().unwrap() }
    pub fn sink(&self) -> CornerId { *self.path.last().unwrap() }
    pub fn corners(&self) -> &[CornerId] { &self.path }
}
