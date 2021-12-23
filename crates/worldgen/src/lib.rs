use conf::WorldGenConf;
use finvec::FinDef;
use hydrology::HydrologyBuilder;
use parameters::Parameters;
use rand::rngs::SmallRng;
use rand::*;
use world_map::WorldMap;
use crate::defs::Defs;

use polymap::{compute::*, *, map_shader::colors::colors};

pub mod conf;
pub mod view;
pub mod world_map;

mod defs;
mod generators;
mod heightmap;
mod hydrology;
mod thermology;
mod biome;

pub use heightmap::HeightMap;
pub use hydrology::Hydrology;

use heightmap::*;

use generators::{Band, Clump, GridGenerator, PerlinField, Slope};
use thermology::{ThermologyBuilder};

pub enum WorldParams {}



impl parameters::Space for WorldParams {
    type Tag = Param;
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Param {
    RainToRiver,
    RiverCutoff
}


pub struct WorldGenerator {
    conf: WorldGenConf,
    params: parameters::Parameters<WorldParams>,
}

impl WorldGenerator {
    pub fn new(conf: WorldGenConf, params: parameters::Parameters<WorldParams>) -> Self {
        Self { conf, params }
    }

    pub fn generate<'a>(&self, poly_map: &'a PolyMap, seed: u64) -> WorldMap<'a> {
        let defs = Defs::new(poly_map);

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
            defs.terrain_type.from_level(heightmap.cell_height(id), |x| x.height_level)
        });

        let hydrology = {
            let conf = &self.conf.hydrology;
            let mut hb = HydrologyBuilder::new(&poly_map);
            hb.scale_by_height(poly_map, &heightmap, conf.rain.height_coeff);

            hb.add_field(
                poly_map,
                &PerlinField::with_rng(conf.rain.perlin.frequency, rng),
                conf.rain.perlin.intensity
            );

            hb.build(&defs, &self.params, &heightmap, &terrain, self.params.get(&Param::RiverCutoff))
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
            tb.build(&defs, &heightmap, &terrain)
        };

        WorldMap {
            defs,
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

finvec::fin_idx!(pub TerrainType);


impl TerrainType {
    pub fn default_definition() -> FinDef<TerrainType, TerrainTypeData> {
        FinDef::new(vec![
            TerrainTypeData {
                name: "Deep Water".to_string(),
                is_water: true,
                height_level: 0.0,
                color: colors::DARKBLUE,
            },
            TerrainTypeData {
                name: "Water".to_string(),
                is_water: true,
                height_level: 0.5,
                color: colors::BLUE,
            },
            TerrainTypeData {
                name: "Land".to_string(),
                is_water: false,
                height_level: 0.5,
                color: colors::GREEN,
            },
            TerrainTypeData {
                name: "Hill".to_string(),
                is_water: false,
                height_level: 0.75,
                color: colors::BROWN,
            },
            TerrainTypeData {
                name: "Mountain".to_string(),
                is_water: false,
                height_level: 1.0001,
                color: colors::WHITE
            },
        ])
    }
}

pub struct TerrainTypeData {
    pub name: String,
    pub is_water: bool,
    pub height_level: f64,
    pub color: polymap::map_shader::Color,
}
