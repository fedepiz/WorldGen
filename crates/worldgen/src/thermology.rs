use polymap::*;
use polymap::compute::*;

use crate::generators::GridGenerator;

pub struct ThermologyBuilder {
    corner_temperature: CornerData<f64>
}

impl ThermologyBuilder {
    pub fn new(poly_map: &PolyMap) -> Self {
        Self { corner_temperature : CornerData::for_each(poly_map, |_,_| 0.0) } 
    }


    pub fn build(self, poly_map: &PolyMap) -> Thermolgoy {
        let cell_temperature = CellData::corner_average(poly_map, &self.corner_temperature);
        Thermolgoy {
            corner_temperature: self.corner_temperature,
            cell_temperature,
        }
    }
}

impl GridGenerator for ThermologyBuilder {
    fn grid_mut(&mut self) -> &mut CornerData<f64> {
        &mut self.corner_temperature
    }
}

pub struct Thermolgoy {
    corner_temperature: CornerData<f64>,
    cell_temperature: CellData<f64>,
}