pub mod measure;
mod biome;

pub use biome::Ground;
use biome::Vegetation;

use std::{collections::HashSet};

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

    wind: Field<Vec2>,

    rainfall: Field<f64>,
    drainage: Field<f64>,
    rivers: Vec<Path>,
    is_river: Field<bool>,

    ground: Field<Ground>,
    vegetation: Field<Vegetation>,
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
            wind: Field::uniform(poly, Vec2::ZERO),
            rainfall: Field::uniform(poly, 0.0),
            drainage: Field::uniform(poly, 0.0),
            rivers: vec![],
            is_river: Field::uniform(poly, false),

            ground: Field::uniform(poly, Ground::default()),
            vegetation: Field::uniform(poly, Vegetation::default()),
        }
    }

    pub fn generate(&mut self, rng: &mut impl Rng) {
        let width = self.poly.width() as f64;
        let height = self.poly.height() as f64;

        self.generate_heightmap(rng);

        self.assign_terrain_types();

        self.temperature = Field::uniform(&self.poly, 0.0);
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

        self.rainfall.update(|_, x| *x = 0.00);
        self.blow_wind(rng);
        self.rainfall.smooth(&self.poly, 3);

        self.generate_rivers();

        self.ground.update(|id, ground| {
            *ground = Ground::new(
                self.terrain_category[id], 
                self.rainfall[id], 
                self.drainage[id], 
                self.heightmap[id]
            )
        });
        self.ground.smooth(&self.poly, 2);

        self.vegetation.update(|id, vegetation| {
            *vegetation = Vegetation::new(
                self.terrain_category[id], 
                self.rainfall[id], 
                self.temperature[id], 
                self.heightmap[id]
            )
        });
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
            *category = if height < 0.3 {
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

    fn blow_wind(&mut self, rng: &mut impl Rng) {
        
        let wind_direction = (rng.gen_range(0..=359) as f64).to_radians();

        // Reset the winds
        self.wind.update(|_, x| *x = Vec2::ZERO);
        
        // For each border tile, we spawn a cloud
        // TODO: Do not just pick up any border, but just the borders which are opposite to 
        // the wind-blowing direction
        for (mut cloud_cell, _) in self.poly().borders() {
            let mut vapor = 10.0;
            let mut direction = wind_direction;
            let mut stop = false;
            let mut visited = Field::uniform(self.poly(), false);
            // Randomly walk the cell through the world
            loop {
                visited[cloud_cell] = true;
                // If the cell is over water, pick up vapor, but if it's over land, drop some vapor.
                // Lose all vapour if over mountain
                let terrain_category = self.terrain_category[cloud_cell];
                match terrain_category {
                    TerrainCategory::Sea => vapor += 0.1,
                    TerrainCategory::Coast => {},
                    TerrainCategory::Land => {
                        let height = self.heightmap[cloud_cell];
                        let rain_rate = if height < 0.6 {
                            0.01
                        } else {
                            0.02
                        };
                        let rain = if height < 0.95 {
                            vapor * rain_rate
                        } else {
                            stop = true;
                            vapor
                        };
                        vapor -= rain;
                        self.rainfall[cloud_cell] += rain;
                    }
                }
                
                // Broken by a high peak
                if stop {
                    break;
                }

                // Add a random drift
                let change_magnitude = 2.5;
                let direction_change = f64::to_radians(rng.gen_range(-change_magnitude..change_magnitude));
                direction += direction_change;
                // Record the path of the cell in the wind table
                self.wind[cloud_cell] += PolarVec2 { r: vapor, theta: direction}.to_cartesian();

                match self.poly().neighbor_in_direction(cloud_cell, direction, 40.0) {
                    Some(x) => { 
                        if visited[x] {
                            break;
                        } else {
                            cloud_cell = x 
                        }
                    },
                    None => break
                }
            }
        }
    }

    fn generate_rivers(&mut self) {

        self.drainage.update(|id, drainage| {
            let mut total = 0.0;
            total += self.rainfall[id];
            if self.terrain_category[id] == TerrainCategory::Coast {
                total += 0.3;
            }
            *drainage = total;
        });

        
        for &source in self.height_sorted.iter().rev() {
            if let CellVector::Towards(target, _) = self.downhill[source] {
                let both_border = self.poly.cell(source).is_border() && self.poly.cell(target).is_border();
                if !both_border {
                    self.drainage[target] += self.drainage[source];
                }
            }
        }

        self.drainage.update(|id, drainage| {
            let is_sea = self.terrain_category[id] == TerrainCategory::Sea;
            if is_sea {
                *drainage = 0.0;
            }
        });
    
        // TODO: Detect rivers while doing drainage, detect joinpoints as well
        self.rivers = Path::paths_cascading(
            &|id| self.drainage[id] > 10.0, 
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

    pub fn wind(&self) -> &Field<Vec2> { &self.wind }

    pub fn rainfall(&self) -> &Field<f64> { &self.rainfall }

    pub fn drainage(&self) -> &Field<f64> { &self.drainage }
    pub fn rivers(&self) -> &[Path] { &self.rivers }
    pub fn is_river(&self, cell: CellId) -> bool { self.is_river[cell] }

    pub fn ground(&self) -> &Field<Ground> { &self.ground }
    pub fn vegetation(&self) -> &Field<Vegetation> { &self.vegetation }

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



#[derive(Clone, Copy)]
pub struct Vec2 {
    pub x: f64,
    pub y: f64,
}

impl Vec2 {
    const ZERO: Vec2 = Vec2 { x: 0.0, y: 0.0 };

    pub fn to_polar(&self) -> Option<PolarVec2> {
        if self.x == 0.0 {
            None
        } else {
            let r = (self.x.powi(2) + self.y.powi(2)).sqrt();
            let theta = (self.y/self.x).atan();
            Some(PolarVec2 { r, theta })
        }
    }
}

impl std::ops::AddAssign<Vec2> for Vec2{
    fn add_assign(&mut self, rhs: Vec2) {
        self.x += rhs.x;
        self.y += rhs.y;
    }
}

#[derive(Clone, Copy, Default)]
pub struct PolarVec2 {
    pub r: f64,
    pub theta: f64,
}

impl PolarVec2 {
    pub fn to_cartesian(&self) -> Vec2 {
        Vec2 {
            x: self.r * self.theta.cos(),
            y: self.r * self.theta.sin()
        }
    }
}