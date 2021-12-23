use polymap::compute::*;
use polymap::*;

use crate::heightmap::HeightMap;
use crate::{TerrainType, Defs};

use crate::generators::GridGenerator;

pub(crate) struct ThermologyBuilder {
    vertex_temperature: VertexData<f64>,
}

impl ThermologyBuilder {
    pub fn new(poly_map: &PolyMap) -> Self {
        Self {
            vertex_temperature: VertexData::for_each(poly_map, |_, _| 0.0),
        }
    }

    pub fn build(
        self,
        defs: &Defs,
        poly_map: &PolyMap,
        heightmap: &HeightMap,
        terrain: &CellData<TerrainType>,
    ) -> Thermolgoy {
        let mut thermology = Thermolgoy::new(self.vertex_temperature);
        thermology.recompute(defs, poly_map, heightmap, terrain);
        thermology
    }
}

impl GridGenerator for ThermologyBuilder {
    fn grid(&self) -> &VertexData<f64> {
        &self.vertex_temperature
    }

    fn grid_mut(&mut self) -> &mut VertexData<f64> {
        &mut self.vertex_temperature
    }
}

pub struct Thermolgoy {
    // Core data
    vertex_innate_temperature: VertexData<f64>,

    // Derived data
    corner_temperature: VertexData<f64>,
    cell_temperature: CellData<f64>,
}

impl Thermolgoy {
    fn new(vertex_innate_temperature: VertexData<f64>) -> Self {
        Self {
            vertex_innate_temperature,
            corner_temperature: VertexData::empty_shell(),
            cell_temperature: CellData::empty_shell(),
        }
    }

    pub(crate) fn recompute(
        &mut self,
        defs: &Defs,
        poly_map: &PolyMap,
        heightmap: &HeightMap,
        terrain: &CellData<TerrainType>,
    ) {
        self.corner_temperature = self.vertex_innate_temperature.clone();
        // Higher places are cooler, and so are seas
        self.corner_temperature
            .update_each(poly_map, |id, corner, temperature| {
                // Is it a water place, or a land place?
                let is_water = corner.cells(poly_map).all(|cell| defs.terrain_type[terrain[cell]].is_water);

                *temperature = if is_water {
                    (*temperature * 0.5).min(0.4)
                } else {
                    let penalty = (1.5 - heightmap.vertex_height(id)).min(1.0);
                    *temperature * penalty
                };
            });

        self.cell_temperature = CellData::vertex_average(poly_map, &self.corner_temperature);
    }

    pub fn cell_temperature(&self, id: CellId) -> f64 {
        self.cell_temperature[id]
    }
}
