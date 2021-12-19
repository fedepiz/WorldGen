use conf::WorldGenConf;
use hydrology::HydrologyBuilder;
use rand::rngs::SmallRng;
use rand::*;

use polymap::map_shader::{Color, MapShader};
use polymap::{compute::*, *};

pub mod conf;
mod generators;
mod heightmap;
mod hydrology;
mod thermology;

pub use heightmap::HeightMap;
pub use hydrology::Hydrology;

use heightmap::*;

use generators::GridGenerator;

pub struct WorldMap {
    pub heightmap: HeightMap,
    hydrology: Hydrology,
}

pub struct WorldGenerator {
    conf: WorldGenConf,
}

impl WorldGenerator {
    pub fn new(conf: WorldGenConf) -> Self {
        Self { conf }
    }

    pub fn generate(&self, poly_map: &PolyMap, seed: u64) -> WorldMap {
        let mut rng = SmallRng::seed_from_u64(seed);
        let rng = &mut rng;

        let heightmap = {
            let conf = &self.conf.heightmap;
            let mut hm = HeightMapBuilder::new(&poly_map, 0.);

            for _ in 0..conf.slopes.number {
                hm.random_slope(poly_map, conf.slopes.intensity, rng);
            }
            hm.perlin_noise(
                poly_map,
                conf.perlin1.frequency,
                conf.perlin1.intensity,
                rng,
            );
            hm.perlin_noise(
                poly_map,
                conf.perlin2.frequency,
                conf.perlin2.intensity,
                rng,
            );

            for _ in 0..conf.clumps.number {
                hm.clump(poly_map, conf.clumps.intensity, 0.90, 0.05, rng)
            }

            for _ in 0..conf.depressions.number {
                hm.clump(poly_map, -conf.depressions.intensity, 0.96, 0.1, rng);
            }

            let num_relax = 3;
            for _ in 0..num_relax {
                hm.relax(poly_map, 0.2);
            }

            if conf.planchon_darboux {
                hm.planchon_darboux(poly_map)
            }

            hm.normalize();
            hm.build(&poly_map)
        };

        let terrain = CellData::for_each(poly_map, |id, _| {
            TerrainType::from_height(heightmap.cell_height(id))
        });

        let hydrology = {
            let conf = &self.conf.hydrology;
            let mut hb = HydrologyBuilder::new(&poly_map);
            hb.height_scaled(poly_map, &heightmap, conf.rain.height_coeff);
            hb.perlin_noise(
                poly_map,
                conf.rain.perlin.frequency,
                conf.rain.perlin.intensity,
                rng,
            );

            hb.build(poly_map, &heightmap, &terrain, conf.min_river_flux)
        };

        WorldMap {
            heightmap,
            hydrology,
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
    const LEVELS: &'static [(TerrainType, f64)] = &[
        (TerrainType::DeepWater, 0.0),
        (TerrainType::Water, 0.5),
        (TerrainType::Land, 0.5),
        (TerrainType::Hill, 0.75),
        (TerrainType::Mountain, 1.00001),
    ];

    fn idx_from_height(height: f64) -> usize {
        Self::LEVELS
            .iter()
            .enumerate()
            .find_map(|(idx, &(_, x))| if height < x { Some(idx) } else { None })
            .unwrap()
    }

    fn from_height(height: f64) -> TerrainType {
        Self::LEVELS[Self::idx_from_height(height)].0
    }

    fn from_height_range(height: f64) -> (TerrainType, TerrainType, f64) {
        let high_idx = Self::idx_from_height(height);

        let low = Self::LEVELS
            .get(high_idx - 1)
            .copied()
            .unwrap_or(Self::LEVELS[0]);
        let high = Self::LEVELS[high_idx];

        // Don't mix land and sea
        let t = if low.0.is_water() != high.0.is_water() {
            1.0
        } else {
            let n = high.1 - low.1;
            // Same height -> only one terrain
            if n == 0.0 {
                1.0
            } else {
                (height - low.1) / (high.1 - low.1)
            }
        };
        (low.0, high.0, t)
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
            ViewMode::Terrain => {
                let terrain_color = |terrain| match terrain {
                    TerrainType::DeepWater => Color::DARKBLUE,
                    TerrainType::Water => Color::BLUE,
                    TerrainType::Land => Color::GREEN,
                    TerrainType::Hill => Color::BROWN,
                    TerrainType::Mountain => Color::WHITE,
                };

                let (tlower, theigher, t) =
                    TerrainType::from_height_range(self.world_map.heightmap.cell_height(id));
                let clower = terrain_color(tlower);
                let chigher = terrain_color(theigher);
                interpolate_colors(clower, chigher, t)
            }
            ViewMode::Hydrology => {
                let rainfall = 100.0 * self.world_map.hydrology.cell_rainfall(id);
                Color::new(0, 0, 255, rainfall.round().min(255.0) as u8)
            }
        }
    }

    fn edge(&self, id: polymap::EdgeId, _: &Edge) -> Option<Color> {
        match self.mode {
            ViewMode::Heightmap => Some(Color::BLACK),
            ViewMode::Terrain => {
                if !self.world_map.hydrology.rivers().is_segment(id) {
                    return None;
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

    fn corner(&self, id: CornerId, corner: &Corner) -> Option<Color> {
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
            }
        }
    }
}

fn interpolate_colors(c1: Color, c2: Color, t: f64) -> Color {
    Color::new(
        lerp8(c1.r, c2.r, t),
        lerp8(c1.g, c2.g, t),
        lerp8(c1.b, c2.b, t),
        lerp8(c1.a, c2.a, t),
    )
}

fn lerp8(a: u8, b: u8, t: f64) -> u8 {
    (((1.0 - t) * a as f64) + (t * b as f64)).round() as u8
}
