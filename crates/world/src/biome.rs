use polymap::field::Smoothable;

use crate::TerrainCategory;
use crate::measure;

#[derive(Clone, Copy, Default)]
pub struct Ground {
    pub water: f64,
    pub sand: f64,
    pub soil: f64,
    pub rock: f64
}

impl Ground {
    pub fn new(terrain_category: TerrainCategory, rain:f64, drain: f64, height: f64) -> Self {
        if terrain_category == TerrainCategory::Sea {
            Ground {
                water: 1.0, sand: 0.0, soil: 0.0, rock: 0.0,
            }
        } else {
            let wetness = measure::DRAIN.normalize(drain).min(1.0) + measure::RAIN.normalize(rain);
            Ground {
                water: 0.0,
                sand: 0.2 * (1.0 - wetness).max(0.0) * (1.0 - height).max(0.0),
                soil: wetness,
                rock: 0.2 * height
            }.normalize()
        }
    }

    pub fn normalize(self) -> Ground {
        let total = self.water + self.sand + self.soil + self.rock;
        if total == 0.0 { self } else {
            Ground {
                water: self.water/total,
                sand: self.sand/total,
                soil: self.soil/total,
                rock: self.rock/total,
            }
        }
    }
}

impl Smoothable for Ground {
    fn add(&mut self, x:&Self) {
        self.water += x.water;
        self.sand += x.sand;
        self.soil += x.soil;
        self.rock += x.rock;
    }

    fn divide(&mut self, n: usize) {
        let n = n as f64;
        self.water /= n;
        self.sand  /= n;
        self.soil  /= n;
        self.rock  /= n;
    }
}
#[derive(Clone, Copy)]
pub struct Vegetation {
    pub none: f64,
    pub deciduous: f64,
    pub boreal: f64,
}

impl Default for Vegetation {
    fn default() -> Self {
        Vegetation {
            none: 1.0, deciduous: 0.0, boreal: 0.0, 
        }
    }
}

impl Vegetation {

    pub fn new(terrain_category: TerrainCategory, rain: f64, temperature: f64, height: f64) -> Vegetation {
        
        match terrain_category {
            TerrainCategory::Sea => Vegetation::default(),
            _ => Vegetation {
                none: (1.0 - measure::RAIN.normalize(rain)).max(0.0),
                deciduous: if height > 0.8 { 0.0 } else { 10.0 * (0.3 - (0.5 - temperature).abs().min(0.3)) },
                boreal:  if height > 0.9 { 0.0 } else { 10.0 * (0.3 - (0.2 - temperature).abs().min(0.3) * height) },
            }.normalize()
        }
    }

    pub fn normalize(self) -> Self {
        let total = self.none + self.deciduous + self.boreal;
        if total == 0.0 { self } else {
            Self {
                none: self.none/total,
                deciduous: self.deciduous/total,
                boreal: self.boreal/total,
            }
        }
    }
}