use polymap::*;
use polymap::compute::*;

use crate::TerrainType;
use crate::heightmap::HeightMap;

use crate::generators::GridGenerator;

pub(crate) struct ThermologyBuilder {
    corner_temperature: CornerData<f64>
}

impl ThermologyBuilder {
    pub fn new(poly_map: &PolyMap) -> Self {
        Self { corner_temperature : CornerData::for_each(poly_map, |_,_| 0.0) } 
    }

    pub fn build(self, poly_map: &PolyMap, heightmap: &HeightMap, terrain: &CellData<TerrainType>) -> Thermolgoy {
        let mut corner_temperature = self.corner_temperature;

        // Higher places are cooler, and so are seas
        corner_temperature.update_each(poly_map, |id, corner, temperature| {
            // Is it a water place, or a land place?
            let is_water = corner.cells(poly_map).all(|cell| terrain[cell].is_water());
            
            *temperature = if is_water {
                (*temperature * 0.5).min(40.0)
            } else {
                let penalty = (1.5 - heightmap.corner_height(id)).min(1.0);
                *temperature * penalty
            };

        });

        let cell_temperature = CellData::corner_average(poly_map, &corner_temperature);
        Thermolgoy {
            corner_temperature,
            cell_temperature,
        }
    }
}

impl GridGenerator for ThermologyBuilder {
    fn grid(&self) -> &CornerData<f64> { &self.corner_temperature }

    fn grid_mut(&mut self) -> &mut CornerData<f64> {
        &mut self.corner_temperature
    }
}

pub struct Thermolgoy {
    corner_temperature: CornerData<f64>,
    cell_temperature: CellData<f64>,
}

impl Thermolgoy {
    pub fn cell_temperature(&self, id: CellId) -> f64 { 
        self.cell_temperature[id]
    }
    pub fn corner_temperature(&self, id: CornerId) -> f64 { 
        self.corner_temperature[id]
    }
}