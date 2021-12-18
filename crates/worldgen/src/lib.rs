
use rand::*;
use rand::rngs::SmallRng;

use polymap::{compute::*, *};
use polymap::map_shader::{MapShader, Color};

pub struct WorldMap {
    heightmap: HeightMap,
    terrain_types: CellData<TerrainType>,
}

struct HeightMap {
    corners: CornerData<f64>,
    cells: CellData<f64>,
}

impl HeightMap {
    fn new(poly_map:&PolyMap, perlin_freq: f64, rng: &mut impl Rng) -> Self {
        use noise::{NoiseFn, Perlin};

        let perlin = Perlin::new();

        let x_rand = rng.gen_range(0..100) as f64;
        let y_rand = rng.gen_range(0..100) as f64;

        let corners = CornerData::for_each(&poly_map, |_, c| {
            let p = perlin.get([ x_rand + c.x() * perlin_freq, y_rand + c.y() * perlin_freq]);
            // Normalize in [0, 1] range
            (p + 1.0)/2.0
        });

        let cells: CellData<f64> = CellData::corner_average(poly_map, &corners);
        Self { corners, cells }
    }
}

pub struct WorldGenerator {
    sea_level: f64,
    montain_level: f64,
    height_perlin_freq: f64,
    remove_land_stragglers: bool,
}

impl WorldGenerator {
    pub fn new() -> Self {
        Self {
            height_perlin_freq: 0.01,
            sea_level: 0.5,
            montain_level: 0.9,
            remove_land_stragglers: true,
        }
    }
    
    fn terrain_type(&self, height: f64) -> TerrainType {
        if height < self.sea_level { 
            TerrainType::Water 
        } else if height > self.montain_level {
            TerrainType::Mountain
        } else { 
            TerrainType::Land
        }
    }

    pub fn generate(&self, poly_map: &PolyMap, seed: u64) -> WorldMap {
        let mut rng = SmallRng::seed_from_u64(seed);
        let rng = &mut rng;

        let heightmap = HeightMap::new(&poly_map, self.height_perlin_freq, rng);

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
    Mountain,
}

impl MapShader for WorldMap {
    fn cell(&self, id: CellId) -> Color {
        match self.terrain_types[id] {
            TerrainType::Water => Color::BLUE,
            TerrainType::Land => Color::GREEN,
            TerrainType::Mountain => Color::BROWN,
        }
    }

    fn edge(&self, _: polymap::EdgeId) -> Color {
        Color::BLACK
    }
}