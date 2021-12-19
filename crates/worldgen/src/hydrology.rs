use std::{collections::{HashSet}};

use polymap::*;
use polymap::compute::*;

use crate::{HeightMap, TerrainType};


pub struct Hydrology {
    corner_flux: CornerData<f64>,
    edge_flux: EdgeData<f64>,
    is_river: EdgeData<bool>,
    river_sources: HashSet<CornerId>,
}


const MIN_RIVER: f64 = 75.0;

impl Hydrology {
    pub(super) fn new(poly_map: &PolyMap, height_map: &HeightMap, terrain:&CellData<TerrainType>, base_flow: f64) -> Hydrology {
        let corner_flux = {
            let mut corner_flux = CornerData::for_each(poly_map, |id, _| 
                height_map.corner_height(id) * base_flow);

            let flow_list = height_map.downhill().filter_map(|from| {
                height_map.descent_vector(from).map(|slope| (from, slope.towards))
            });
            corner_flux.flow(flow_list,|x, y| {
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

        let is_river = EdgeData::from_cell_data(poly_map, &terrain,
            |id,_,terrain| {
                let is_water = terrain.iter().any(|tt| tt.is_water());
                !is_water && edge_flux[id] > MIN_RIVER
            }
        );

        let mut river_sources= HashSet::new();
        for (id, edge) in poly_map.edges() {
            if is_river[id] {
                if let Some(top) = height_map.edge_high_corner(edge) {   
                    let is_source = poly_map.corner(top).edges().iter().all(
                        |&other_id| id == other_id || !is_river[other_id]
                    );
                    if is_source {
                        river_sources.insert(top);
                    }
                }
            }
        }
    
        Hydrology { corner_flux, edge_flux, is_river, river_sources }
    }

    pub fn edge_flux(&self, edge: EdgeId) -> f64 {
        self.edge_flux[edge]
    }

    pub fn is_river_segment(&self, edge: EdgeId) -> bool {
        self.is_river[edge]
    }

    pub fn is_river_source(&self, corner: CornerId) -> bool {
        self.river_sources.contains(&corner)
    }
}