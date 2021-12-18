use rand::*;
use rand::rngs::SmallRng;

use polymap::{compute::*, *};
use polymap::map_shader::{MapShader, Color};

#[derive(Clone)]
pub struct WorldMap {
    pub heightmap: HeightMap,
    terrain_types: CellData<TerrainType>,
}

struct HeightMapBuilder {
    corners: CornerData<f64>,
}

impl HeightMapBuilder {
    fn new( poly_map:&PolyMap, default: f64) -> Self {
        let corners = CornerData::for_each(&poly_map, |_, _| {
            default
        });
        Self {
            corners
        }
    }

    fn perlin_noise(&mut self, poly_map:&PolyMap, perlin_freq: f64, intensity: f64, rng: &mut impl Rng) {
        use noise::{NoiseFn, Perlin};

        let perlin = Perlin::new();

        let x_rand = rng.gen_range(0..100) as f64;
        let y_rand = rng.gen_range(0..100) as f64;

        self.corners.update_each(poly_map, |_, corner, h| {
            let px =  x_rand + corner.x() * perlin_freq;
            let py = y_rand + corner.y() * perlin_freq;
            let noise = perlin.get([px, py]);
            *h += noise * intensity;
        })
    }

    fn random_slope(&mut self, poly_map:&PolyMap, steepness: f64, rng: &mut impl Rng) {
        let m = rng.gen_range(-100..200) as f64/100.0;
        
        let w = poly_map.width() as f64;
        let h = poly_map.height() as f64;
        self.corners.update_each(&poly_map, |_, corner, corner_height| {
            let distance = (corner.x() - w/2.0) * m  - (corner.y() - h/2.0);
            *corner_height += distance * steepness;
        })
    }

    fn normalize(&mut self) {
        let min = self.corners.data.iter().copied().reduce(f64::min).unwrap();
        let max = self.corners.data.iter().copied().reduce(f64::max).unwrap();
        self.corners.data.iter_mut().for_each(|x| *x = (*x - min)/(max - min));
    }

    fn build(self, poly_map:&PolyMap) -> HeightMap {
        let cells: CellData<f64> = CellData::corner_average(poly_map, &self.corners);
        
        HeightMap {
            corners: self.corners, cells
        }
    }
}
#[derive(Clone)]
pub struct HeightMap {
    pub corners: CornerData<f64>,
    pub cells: CellData<f64>,
}


pub struct WorldGenerator {
    height_perlin_freq: f64,
    remove_land_stragglers: bool,
}

impl WorldGenerator {
    pub fn new() -> Self {
        Self {
            height_perlin_freq: 0.005,
            remove_land_stragglers: true,
        }
    }
    
    fn terrain_type(&self, height: f64) -> TerrainType {
        const LEVELS: &'static [(TerrainType, f64)] = &[
            (TerrainType::Water, 0.4),
            (TerrainType::Land, 0.6),
            (TerrainType::Hill, 0.8),
            (TerrainType::Mountain, 1.0),
        ];

        LEVELS.iter()
            .find_map(|&(tt, x)| if height <= x { Some(tt) } else { None })
            .expect("Invalid terrain type, heightmap out of bounds")
    }

    pub fn generate(&self, poly_map: &PolyMap, seed: u64) -> WorldMap {
        let mut rng = SmallRng::seed_from_u64(seed);
        let rng = &mut rng;

        let heightmap = {
            let mut hm = HeightMapBuilder::new(&poly_map, 0.5);

            let num_slopes = rng.gen_range(1..=2);
            for _ in 0..num_slopes {
                hm.random_slope(poly_map, 0.001, rng);
            }
            hm.perlin_noise(poly_map, self.height_perlin_freq, 0.5, rng);
            hm.normalize();
            hm.build(&poly_map)
        };

        let mut terrain_types = heightmap.cells
            .transform(|_, &x| self.terrain_type(x));

        if self.remove_land_stragglers {
            let found:Vec<_> = terrain_types.find_with_all_neighbors(poly_map, |_, terrain| {
                match terrain {
                    TerrainType::Water => true,
                    _ => false,
                }
            }).collect();

            for cell in found {
                terrain_types[cell] = TerrainType::Water;
            }
        }

        WorldMap {
            heightmap,
            terrain_types,
        }
    }
}


#[derive(Clone, Copy)]
enum TerrainType {
    Water,
    Land,
    Hill,
    Mountain,
}


#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Terrain,
    Heightmap,
}

pub struct WorldMapView<'a> { 
    world_map: &'a WorldMap, 
    mode: ViewMode 
}

impl <'a> WorldMapView <'a>{
    pub fn new(world_map: &'a WorldMap, mode: ViewMode) -> Self {
        Self { world_map, mode }
    }
}

impl <'a> MapShader for WorldMapView<'a> {
    fn cell(&self, id: CellId) -> Color {
        match self.mode {
            ViewMode::Heightmap => {
                let height = self.world_map.heightmap.cells[id];
                let intensity = (255.0 * height).max(0.0).min(255.0).round() as u8;
                Color::new(intensity, intensity, intensity, 255)
            }
            ViewMode::Terrain => {
                match self.world_map.terrain_types[id] {
                    TerrainType::Water => Color::BLUE,
                    TerrainType::Land => Color::GREEN,
                    TerrainType::Hill => Color::BROWN,
                    TerrainType::Mountain => Color::WHITE,
                }
            }
        }

   
    }

    fn edge(&self, _: polymap::EdgeId) -> Color {
        Color::BLACK
    }
}
