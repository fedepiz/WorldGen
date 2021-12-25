use std::collections::HashSet;

use polymap::*;
use polymap::field::*;

mod spatial_function;

use rand::Rng;
use spatial_function::{PerlinField, SpatialFunction, Slope};
pub struct World<'a> {
    poly: &'a PolyMap,
    heightmap: Field<f64>,
    downhill: Field<CellVector<f64>>,
    height_sorted: Vec<CellId>,
    terrain_category: Field<TerrainCategory>,
    temperature: Field<f64>,

    wind: Field<CellVector<f64>>,

    rainfall: Field<f64>,
    drainage: Field<f64>,
    rivers: Vec<Path>,
    is_river: Field<bool>,
}

impl <'a> World<'a> {
    pub fn new(poly: &'a PolyMap) -> Self {
        Self {
            poly,
            heightmap: Field::uniform(poly, 0.0),
            downhill: Field::uniform(poly, CellVector::Stationary),
            height_sorted: vec![],
            terrain_category: Field::uniform(poly, TerrainCategory::Land),
            temperature: Field::uniform(poly, 0.0),
            wind: Field::uniform(poly, CellVector::Stationary),
            rainfall: Field::uniform(poly, 0.0),
            drainage: Field::uniform(poly, 0.0),
            rivers: vec![],
            is_river: Field::uniform(poly, false)
        }
    }

    pub fn generate(&mut self, rng: &mut impl Rng) {
        let width = self.poly.width() as f64;
        let height = self.poly.height() as f64;

        self.generate_heightmap(rng);

        self.assign_terrain_types();
      
        spatial_function::Band::new(width/2.0, height/2.0, 0.0, height/2.0)
            .add_to_field(&self.poly, &mut self.temperature);

        self.temperature.update(|id, temperature| {
            // If height > 0.6, proportionally scale down the temperature
            let height = self.heightmap[id];
            if height >= 0.6 {
                let penalty = (height - 0.6)/(1.0 - 0.6);
                *temperature = *temperature * (1.2 - penalty).min(1.0);
            }
        });

       // For now, wind always blows from the east (Remember, 0.0 -> West to East)
       let tgt_angle = f64::to_radians(180.0); 
       for (cell_id, cell) in self.poly().borders() {
            let tgt_neighbor = cell.neighbors().iter().map(|&neighbor_id| {
                let angle = self.poly().angle_between_cells(cell_id, neighbor_id);
                let difference = f64::atan2((angle - tgt_angle).sin(), (angle - tgt_angle).cos()).abs();
                (neighbor_id, difference)
            }).reduce(|(id1, f1),(id2, f2)| if f1 <= f2 { (id1,f1) } else { (id2, f2) });

            if let Some((tgt_id, difference)) = tgt_neighbor {
                if difference < f64::to_radians(60.0) {
                    self.wind[cell_id] = CellVector::Towards(tgt_id, 0.1)
                }
            }
       }

        self.rainfall.update(|_, x| *x = 0.05);

        self.generate_rivers();
    }

    fn generate_heightmap(&mut self, rng: &mut impl Rng) {
        let width = self.poly.width() as f64;
        let height = self.poly.height() as f64;

        Slope::with_rng(width, height, rng)
            .scale(0.00025)
            .add_to_field(self.poly, &mut self.heightmap);
        PerlinField::with_rng(0.001, rng).scale(1.0).add_to_field(self.poly, &mut self.heightmap);
        PerlinField::with_rng(0.01, rng).scale(0.2).add_to_field(self.poly, &mut self.heightmap);
        
        planchon_darboux(&mut self.heightmap, &self.poly);
        self.heightmap.normalize();

        self.downhill.update(|id, slope| {
            let my_height = self.heightmap[id];
            // Find the neighbor with minimum height, if any
            let min_neighbor = self.poly[id].neighbors().iter()
                .map(|&id| (id, self.heightmap[id]))
                .reduce(|(id1, x), (id2, y)| if x <= y { (id1, x) } else { (id2, y)});
           
            // If the minimum neighbor is smaller then me, then that's my slope
            *slope = min_neighbor
                .filter(|&(_, x)| x < my_height)
                .map(|(id, h)| CellVector::Towards(id, my_height - h)).unwrap_or(CellVector::Stationary)
        });

        self.height_sorted = self.heightmap.ascending_order();
    }

    fn assign_terrain_types(&mut self) {
        self.terrain_category.update(|id, category| {
            let height = self.heightmap[id];
            *category = if height < 0.5 {
                TerrainCategory::Sea
            } else {
                TerrainCategory::Land
            }
        });
        
        {
            let mut out_to_be_coast = vec![];
            for (cell_id, cell) in self.poly.cells() {
                if self.terrain_category[cell_id] == TerrainCategory::Land {
                    if cell.neighbors().iter().any(|&neighbor| self.terrain_category[neighbor] == TerrainCategory::Sea) {
                        out_to_be_coast.push(cell_id)
                    }
                }
            }
            for cell in out_to_be_coast {
                self.terrain_category[cell] = TerrainCategory::Coast;
            }
        }

    }

    fn generate_rivers(&mut self) {
        self.drainage.update(|id, drainage| *drainage = self.rainfall[id]);
        
        for &cell in self.height_sorted.iter().rev() {
            if let CellVector::Towards(target, _) = self.downhill[cell] {
                self.drainage[target] += self.drainage[cell];
            }
        }

        self.drainage.update(|id, drainage| 
                if self.terrain_category[id] == TerrainCategory::Sea {
                    *drainage = 0.0;
                });
    
        // TODO: Detect rivers while doing drainage, detect joinpoints as well
        self.rivers = Path::paths_cascading(
            &|id| self.drainage[id] > 0.95, 
            &|id| match self.downhill[id] {
                CellVector::Stationary => None,
                CellVector::Towards(tgt, _) => Some(tgt),
            }, self.height_sorted.iter().rev().cloned())
            .into_iter().filter(|p| p.cells().len() > 2)
            .collect();

        self.is_river = Field::uniform(self.poly(), false);

        for river in self.rivers.iter() {
            for &cell in river.cells().iter() {
                self.is_river[cell] = true;
            }
        }
    }
    
    pub fn poly(&self) -> &'a PolyMap { self.poly }
    pub fn heightmap(&self) -> &Field<f64> { &self.heightmap }
    pub fn downhill(&self) -> &Field<CellVector<f64>> { &self.downhill }

    pub fn terrain_category(&self) -> &Field<TerrainCategory> { &self.terrain_category }
    pub fn temperature(&self) -> &Field<f64> { &self.temperature }

    pub fn wind(&self) -> &Field<CellVector<f64>> { &self.wind }
    pub fn rainfall(&self) -> &Field<f64> { &self.rainfall }
    pub fn drainage(&self) -> &Field<f64> { &self.drainage }
    pub fn rivers(&self) -> &[Path] { &self.rivers }
    pub fn is_river(&self, cell: CellId) -> bool { self.is_river[cell] }
}

 fn planchon_darboux(heightmap:&mut Field<f64>, poly_map: &PolyMap) {
    let epsilon = 0.001;
    let h = heightmap;

    let mut new_h = Field::with_fn(poly_map, |id, cell| {
        if cell.is_border() {
            h[id]
        } else {
            100.0
        }
    });

    let mut changed = true;
    while changed {
        changed = false;
        for (id, corner) in poly_map.cells() {
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

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum TerrainCategory {
    Sea,
    Coast,
    Land,
} 

#[derive(Clone, Copy)]
pub enum CellVector<T> {
    Stationary,
    Towards(CellId, T),
}

impl <T> field::Vectorial for CellVector<T> {
    fn points_to(&self) -> Option<CellId> {
        match self {
            Self::Stationary => None,
            &Self::Towards(tgt, _) => Some(tgt),
        }
    }
}

pub struct Path(Vec<CellId>);

impl Path {
    pub fn cells(&self) -> &[CellId] {
        self.0.as_slice()
    }

    pub fn paths_cascading(
            property: &impl Fn(CellId) -> bool,
            next: &impl Fn(CellId) -> Option<CellId>,
            order: impl Iterator<Item=CellId>) -> Vec<Path> {
        let mut paths = vec![];
        let mut visited = HashSet::new();

        for cell in order {
           if !visited.contains(&cell) {
               visited.insert(cell);
                if property(cell) {
                    let path = Self::path_while(cell, property, next);
                    visited.extend(path.iter().copied());
                    paths.push(Path(path))
                }
           }
        }
        paths
    } 

    pub fn path_while(mut cell: CellId, 
        property: impl Fn(CellId) -> bool,
        next: impl Fn(CellId) -> Option<CellId>) -> Vec<CellId> {
        
            let mut path = vec![];
            loop {
                path.push(cell);
                match next(cell) {
                    None => return path,
                    Some(next_cell) => {
                        if !property(next_cell) { 
                            return path
                        } else {
                            cell = next_cell
                        }
                    }
                }
            }
    }
}