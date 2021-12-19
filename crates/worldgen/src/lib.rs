use rand::rngs::SmallRng;
use rand::*;

use polymap::map_shader::{Color, MapShader};
use polymap::{compute::*, *};

mod heightmap;
mod rivers;

pub use heightmap::HeightMap;
pub use rivers::Rivers;

use heightmap::*;

pub struct WorldMap {
    pub heightmap: HeightMap,
    terrain: CellData<TerrainType>,
    rivers: Rivers,
}

const RIVER_BASE_FLOW: f64 = 2.0;

pub struct WorldGenerator;

impl WorldGenerator {
    pub fn new() -> Self {
        Self
    }

    pub fn generate(&self, poly_map: &PolyMap, seed: u64) -> WorldMap {
        let mut rng = SmallRng::seed_from_u64(seed);
        let rng = &mut rng;

        let heightmap = {
            let mut hm = HeightMapBuilder::new(&poly_map, 0.);

            hm.random_slope(poly_map, 0.0005, rng);

            hm.perlin_noise(poly_map, 0.001, 1.0, rng);
            hm.perlin_noise(poly_map, 0.01, 0.2, rng);

            let positive_clumps = 2;
            let negative_clumps = 0;

            for _ in 0..positive_clumps {
                hm.clump(poly_map, 0.2, 0.90, 0.05, rng)
            }

            for _ in 0..negative_clumps {
                hm.clump(poly_map, -0.2, 0.96, 0.1, rng);
            }

            let num_relax = 3;
            for _ in 0..num_relax {
                hm.relax(poly_map, 0.2);
            }

            hm.normalize();
            hm.build(&poly_map)
        };

        let terrain = heightmap
            .cells
            .transform(|_, &x| TerrainType::from_height(x));
    

        let rivers = Rivers::new(poly_map, &heightmap, &terrain, RIVER_BASE_FLOW);

        WorldMap {
            heightmap,
            terrain,
            rivers,
        }
    }
}

#[derive(Clone, Copy)]
enum TerrainType {
    DeepWater,
    Water,
    Land,
    Hill,
    Mountain,
}

impl TerrainType {
    fn from_height(height: f64) -> TerrainType {
        const LEVELS: &'static [(TerrainType, f64)] = &[
            (TerrainType::DeepWater, 0.1),
            (TerrainType::Water, 0.5),
            (TerrainType::Land, 0.75),
            (TerrainType::Hill, 0.9),
            (TerrainType::Mountain, 1.0),
        ];

        LEVELS
            .iter()
            .find_map(|&(tt, x)| if height <= x { Some(tt) } else { None })
            .expect("Invalid terrain type, heightmap out of bounds")
    }

    fn is_water(&self) -> bool {
        match self {
            TerrainType::Water => true,
            TerrainType::DeepWater => true,
            _ => false,
        }
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Terrain,
    Heightmap,
}

pub struct WorldMapView<'a> {
    world_map: &'a WorldMap,
    mode: ViewMode,
}

impl<'a> WorldMapView<'a> {
    pub fn new(world_map: &'a WorldMap, mode: ViewMode) -> Self {
        Self { world_map, mode }
    }
}

impl<'a> MapShader for WorldMapView<'a> {
    fn cell(&self, id: CellId) -> Color {
        match self.mode {
            ViewMode::Heightmap => {
                let height = self.world_map.heightmap.cells[id];
                let intensity = (255.0 * height).max(0.0).min(255.0).round() as u8;
                Color::new(intensity, intensity, intensity, 255)
            }
            ViewMode::Terrain => match self.world_map.terrain[id] {
                TerrainType::DeepWater => Color::DARKBLUE,
                TerrainType::Water => Color::BLUE,
                TerrainType::Land => Color::GREEN,
                TerrainType::Hill => Color::BROWN,
                TerrainType::Mountain => Color::WHITE,
            },
        }
    }

    fn edge(&self, id: polymap::EdgeId, _: &Edge) -> Option<Color> {
        match self.mode {
            ViewMode::Heightmap => Some(Color::BLACK),
            ViewMode::Terrain => {
                if !self.world_map.rivers.is_river[id] {
                    return None
                }
                let flow = self.world_map.rivers.edge_flux[id];
                let max = 250.0;
                let prop = 255.0 * (flow/max);
                Some(Color::new(0, 0, 255, prop.round() as u8))
            }
        }
    }

    fn draw_corners(&self) -> bool {
        match self.mode {
            ViewMode::Heightmap => true,
            ViewMode::Terrain => false,
        }
    }
    
    fn corner(&self, id: CornerId, _:&Corner) -> Option<Color> {
        match self.mode {
            ViewMode::Heightmap => {
                let has_slope = self.world_map.heightmap.descent_vector[id].is_some();
                if !has_slope {
                    Some(Color::RED)
                } else {
                    None
                }
            }
            ViewMode::Terrain => None,
        }
    }
}
