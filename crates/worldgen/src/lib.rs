use rand::rngs::SmallRng;
use rand::*;

use polymap::map_shader::{Color, MapShader};
use polymap::{compute::*, *};

mod heightmap;
mod hydrology;

pub use heightmap::HeightMap;
pub use hydrology::Hydrology;

use heightmap::*;

pub struct WorldMap {
    pub heightmap: HeightMap,
    terrain: CellData<TerrainType>,
    hydrology: Hydrology,
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

        let terrain = CellData::for_each(
            poly_map, |id, _| TerrainType::from_height(heightmap.cell_height(id))
        );
    

        let rivers = Hydrology::new(poly_map, &heightmap, &terrain, RIVER_BASE_FLOW);

        WorldMap {
            heightmap,
            terrain,
            hydrology: rivers,
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
            .unwrap_or_else(|| TerrainType::DeepWater)
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
    Hydrology,
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
                let height = self.world_map.heightmap.cell_height(id);
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
            ViewMode::Hydrology => Color::WHITE,
        }
    }

    fn edge(&self, id: polymap::EdgeId, _: &Edge) -> Option<Color> {
        match self.mode {
            ViewMode::Heightmap => Some(Color::BLACK),
            ViewMode::Terrain => {
                if !self.world_map.hydrology.rivers().is_segment(id) {
                    return None
                }
                let flow = self.world_map.hydrology.edge_flux(id);
                Some(Color::new(0, 0, 255, flow.round().min(255.0) as u8))
            }
            ViewMode::Hydrology => {                
                let flow = self.world_map.hydrology.edge_flux(id);
                
                Some(Color::new(0, 0, 255, flow.round().min(255.0) as u8))
            }
        }
    }

    fn draw_corners(&self) -> bool {
        match self.mode {
            ViewMode::Heightmap => true,
            ViewMode::Terrain => false,
            ViewMode::Hydrology => true,
        }
    }
    
    fn corner(&self, id: CornerId, corner:&Corner) -> Option<Color> {
        match self.mode {
            ViewMode::Heightmap => {
                let has_slope = self.world_map.heightmap.descent_vector(id).is_some();
                if !has_slope && !corner.is_border() {
                    Some(Color::RED)
                } else {
                    None
                }
            }
            ViewMode::Terrain => None,
            ViewMode::Hydrology => {
                let rivers = self.world_map.hydrology.rivers();

                if rivers.is_source(id) {
                    Some(Color::GREEN)
                } else if rivers.is_sink(id) {
                    Some(Color::RED)
                } else {
                    None
                }
            },
        }
    }
}
