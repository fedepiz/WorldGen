use std::{collections::HashSet};

use polymap::*;
use polymap::compute::*;
use rand::Rng;

use crate::{HeightMap, TerrainType};
use crate::generators;

pub(crate) struct HydrologyBuilder {
    corner_rainfall: CornerData<f64>
}

impl HydrologyBuilder {
    pub fn new(poly_map: &PolyMap,) -> Self {
        Self {
            corner_rainfall: CornerData::for_each(poly_map, |_,_| 0.0)
        }
    }

    pub fn height_scaled(&mut self, poly_map:&PolyMap, hm: &HeightMap, coeff: f64) {
        self.corner_rainfall.update_each(poly_map, |id, _, h| {
            let height = hm.corner_height(id);
            *h += height * coeff 
        })
    }

    pub(crate) fn perlin_noise(
        &mut self,
        poly_map: &PolyMap,
        perlin_freq: f64,
        intensity: f64,
        rng: &mut impl Rng,
    ) {
        generators::perlin_noise(&mut self.corner_rainfall, poly_map, perlin_freq, intensity, rng)
    }

    pub fn build(mut self, poly_map: &PolyMap, height_map: &HeightMap, terrain:&CellData<TerrainType>, min_river_flux: f64) -> Hydrology {
        let cell_rainfall = CellData::corner_average(poly_map, &self.corner_rainfall);

        let corner_flux = {
            self.corner_rainfall.flow(height_map.downhill_flow(),|x, y| {
                *x += *y;
            });
            self.corner_rainfall
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

        let rivers = Rivers::new(poly_map, height_map, terrain, &edge_flux, min_river_flux);
    
        Hydrology { corner_flux, edge_flux, cell_rainfall, rivers }
    }
}

pub struct Hydrology {
    cell_rainfall: CellData<f64>,
    corner_flux: CornerData<f64>,
    edge_flux: EdgeData<f64>,
    rivers: Rivers
}

impl Hydrology {

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
    paths: Vec<RiverPath>
}


impl Rivers {
    fn new(poly_map: &PolyMap, 
           height_map: &HeightMap, 
           terrain:&CellData<TerrainType>, 
           edge_flux:&EdgeData<f64>,
           min_river_flux: f64) -> Self {

        let edge_is_river = EdgeData::from_cell_data(poly_map, &terrain,
            |id,_,terrain| {
                let is_water = terrain.iter().any(|tt| tt.is_water());
                !is_water && edge_flux[id] > min_river_flux
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
