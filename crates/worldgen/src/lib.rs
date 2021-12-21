use conf::WorldGenConf;
use hydrology::HydrologyBuilder;
use rand::rngs::SmallRng;
use rand::*;

use polymap::map_shader::{Color, MapShader};
use polymap::{compute::*, *};

use polymap::map_shader::colors as colors;

pub mod conf;
mod generators;
mod heightmap;
mod hydrology;
mod thermology;

pub use heightmap::HeightMap;
pub use hydrology::Hydrology;

use heightmap::*;

use generators::{Band, Clump, GridGenerator, PerlinField, Slope};
use thermology::{Thermolgoy, ThermologyBuilder};

pub struct WorldMap {
    heightmap: HeightMap,
    terrain: CellData<TerrainType>,
    hydrology: Hydrology,
    thermology: Thermolgoy,
}

impl WorldMap {
    pub fn heightmap(&self) -> &HeightMap {
        &self.heightmap
    }

    pub fn reflow_rivers(&mut self, poly_map: &PolyMap) {
        self.update_heightmap(poly_map, |hmb| {
            hmb.add_field(poly_map, &PerlinField::new(0.0, 0.0, 0.001), 0.2);
            hmb.planchon_darboux(poly_map);
        })
    }

    fn update_heightmap(&mut self, poly_map: &PolyMap, f: impl FnOnce(&mut HeightMapBuilder)) {
        let mut hmb = self.heightmap.make_builder();
        f(&mut hmb);
        self.heightmap = hmb.build(poly_map);
        self.terrain = CellData::for_each(poly_map, |id, _| {
            TerrainType::from_height(self.heightmap.cell_height(id))
        });
        self.hydrology
            .recompute(poly_map, &self.heightmap, &self.terrain);
        self.thermology
            .recompute(poly_map, &self.heightmap, &self.terrain)
    }
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
                let slope = Slope::with_rng(poly_map.width() as f64, poly_map.height() as f64, rng);
                hm.add_field(poly_map, &slope, conf.slopes.intensity);
            }

            hm.add_field(
                poly_map,
                &PerlinField::with_rng(conf.perlin1.frequency, rng),
                conf.perlin1.intensity,
            );
            hm.add_field(
                poly_map,
                &PerlinField::with_rng(conf.perlin2.frequency, rng),
                conf.perlin2.intensity,
            );

            for _ in 0..conf.clumps.number {
                let clump = Clump::with_rng(
                    poly_map.width() as f64,
                    poly_map.height() as f64,
                    conf.clumps.intensity,
                    0.90,
                    rng,
                );
                hm.add_field(poly_map, &clump, 1.0);
            }

            for _ in 0..conf.depressions.number {
                let clump = Clump::with_rng(
                    poly_map.width() as f64,
                    poly_map.height() as f64,
                    -conf.depressions.intensity,
                    0.90,
                    rng,
                );
                hm.add_field(poly_map, &clump, 1.0);
            }

            let num_relax = 3;
            for _ in 0..num_relax {
                hm.relax(poly_map, 0.2);
            }

            if conf.planchon_darboux {
                hm.planchon_darboux(poly_map)
            }

            hm.build(&poly_map)
        };

        let terrain = CellData::for_each(poly_map, |id, _| {
            TerrainType::from_height(heightmap.cell_height(id))
        });

        let hydrology = {
            let conf = &self.conf.hydrology;
            let mut hb = HydrologyBuilder::new(&poly_map);
            hb.scale_by_height(poly_map, &heightmap, conf.rain.height_coeff);

            hb.add_field(
                poly_map,
                &PerlinField::with_rng(conf.rain.perlin.frequency, rng),
                conf.rain.perlin.intensity,
            );

            hb.build(poly_map, &heightmap, &terrain, conf.min_river_flux)
        };

        let thermology = {
            let mut tb = ThermologyBuilder::new(&poly_map);

            // Some background perlin noise just to mix things up a bit
            tb.add_field(poly_map, &PerlinField::with_rng(0.0005, rng), 0.2);

            // A band along the equator
            let w = poly_map.width() as f64;
            let h = poly_map.height() as f64;
            let radius = h / 2.0;
            tb.add_field(poly_map, &Band::new(w / 2.0, h / 2.0, 0.0, radius), 0.8);
            tb.build(poly_map, &heightmap, &terrain)
        };

        WorldMap {
            heightmap,
            terrain,
            hydrology,
            thermology,
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
    Thermology,
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
                let intensity = (height as f32).max(0.0).min(1.0);
                Color::new(intensity, intensity, intensity, 1.0)
            }
            ViewMode::Terrain => {
                let terrain_color = |terrain| match terrain {
                    TerrainType::DeepWater => colors::DARKBLUE,
                    TerrainType::Water => colors::BLUE,
                    TerrainType::Land => colors::GREEN,
                    TerrainType::Hill => colors::BROWN,
                    TerrainType::Mountain => colors::WHITE,
                };

                let (tlower, theigher, t) =
                    TerrainType::from_height_range(self.world_map.heightmap.cell_height(id));
                let clower = terrain_color(tlower);
                let chigher = terrain_color(theigher);
                interpolate_colors(clower, chigher, t as f32)
            }
            ViewMode::Hydrology => {
                let rainfall = (self.world_map.hydrology.cell_rainfall(id) * 100.0)/255.0;
                Color::new(0.0, 0.0, 1.0, rainfall.min(1.0) as f32)
            }
            ViewMode::Thermology => {
                let temperature = self.world_map.thermology.cell_temperature(id);

                let t_value = temperature.max(0.0).min(1.0) as f32;
                interpolate_three_colors(colors::DARKBLUE, colors::YELLOW, colors::RED, t_value)
            }
        }
    }

    fn edge(&self, id: polymap::EdgeId, _: &Edge) -> Option<Color> {
        match self.mode {
            ViewMode::Heightmap => Some(colors::BLACK),
            ViewMode::Terrain => {
                if !self.world_map.hydrology.rivers().is_segment(id) {
                    return None;
                }
                let flow = self.world_map.hydrology.edge_flux(id)/255.0;
                Some(Color::new(0.0, 0.0, 1.0, flow.min(1.0) as f32))
            }
            ViewMode::Hydrology => {
                let flow = self.world_map.hydrology.edge_flux(id)/255.0;

                Some(Color::new(0.0, 0.0, 1.0, flow.min(1.0) as f32))
            }
            ViewMode::Thermology => None,
        }
    }

    fn draw_vertices(&self) -> bool {
        match self.mode {
            ViewMode::Heightmap => true,
            ViewMode::Terrain => false,
            ViewMode::Hydrology => true,
            ViewMode::Thermology => false,
        }
    }

    fn vertex(&self, id: VertexId, vertex: &Vertex) -> Option<Color> {
        match self.mode {
            ViewMode::Heightmap => {
                let has_slope = self.world_map.heightmap.descent_vector(id).is_some();
                if !has_slope && !vertex.is_border() {
                    Some(colors::RED)
                } else {
                    None
                }
            }
            ViewMode::Terrain => None,
            ViewMode::Hydrology => {
                let rivers = self.world_map.hydrology.rivers();

                if rivers.is_source(id) {
                    Some(colors::GREEN)
                } else if rivers.is_sink(id) {
                    Some(colors::RED)
                } else {
                    None
                }
            }
            ViewMode::Thermology => None,
        }
    }
}

fn interpolate_three_colors(c1: Color, c2: Color, c3: Color, t: f32) -> Color {
    if t <= 0.5 {
        interpolate_colors(c1, c2, 2. * t)
    } else {
        interpolate_colors(c2, c3, 2. * (t - 0.5))
    }
}

fn interpolate_colors(c1: Color, c2: Color, t: f32) -> Color {
    Color::new(
        lerp8(c1.r, c2.r, t),
        lerp8(c1.g, c2.g, t),
        lerp8(c1.b, c2.b, t),
        lerp8(c1.a, c2.a, t),
    )
}

fn lerp8(a: f32, b: f32, t: f32) -> f32 {
    ((1.0 - t) * a) + (t * b)
}
