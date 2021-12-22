use conf::WorldGenConf;
use hydrology::HydrologyBuilder;
use parameters::Parameters;
use rand::rngs::SmallRng;
use rand::*;

use polymap::{compute::*, *};

pub mod conf;
pub mod view;

mod generators;
mod heightmap;
mod hydrology;
mod thermology;

pub use heightmap::HeightMap;
pub use hydrology::Hydrology;

use heightmap::*;

use generators::{Band, Clump, GridGenerator, PerlinField, Slope};
use thermology::{Thermolgoy, ThermologyBuilder};

pub enum WorldParams {}



impl parameters::Space for WorldParams {
    type Tag = Param;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Param {
    RiverCutoff
}

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
    params: parameters::Parameters<WorldParams>,
}

impl WorldGenerator {
    pub fn new(conf: WorldGenConf, params: parameters::Parameters<WorldParams>) -> Self {
        Self { conf, params }
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

            hb.build(poly_map, &heightmap, &terrain, self.params.get(&Param::RiverCutoff))
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

    pub fn parameters(&self) -> &Parameters<WorldParams> {
        &self.params
    }

    pub fn parameters_mut(&mut self) -> &mut Parameters<WorldParams> {
        &mut self.params
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
