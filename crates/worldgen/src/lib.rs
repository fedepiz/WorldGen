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
            *h += (noise + 1.0)/2.0 * intensity;
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



    fn clump(&mut self, poly_map:&PolyMap, amount: f64, decay: f64, end: f64, rng: &mut impl Rng) {
        let starting = CornerPicker::random(poly_map, rng);
        self.corners.spread(poly_map, starting, amount, 
            |accum| if accum.abs() > end.abs() { Some(accum * decay) } else { None }, 
            |_, corner_height, x| 
                *corner_height += *x
            )
    }

    fn normalize(&mut self) {
        let min = self.corners.data.iter().copied().reduce(f64::min).unwrap();
        let max = self.corners.data.iter().copied().reduce(f64::max).unwrap();
        self.corners.data.iter_mut().for_each(|x| *x = (*x - min)/(max - min));
    }

    fn relax(&mut self, poly_map:&PolyMap, t: f64) {
        self.corners.update_with_neighbors(poly_map, |x, neighborhood| {
            let average = neighborhood.iter().copied().sum::<f64>();
            let n = neighborhood.len() as f64;
            *x = t * (average/n) + (1.0 - t) * *x
        })
    }

    fn build(self, poly_map:&PolyMap) -> HeightMap {
        let cells: CellData<f64> = CellData::corner_average(poly_map, &self.corners);
        
        let descent_vector = CornerData::for_each(poly_map, |id, corner| {
            let my_elevation = self.corners[id];
            let mut slope: Option<Slope> = None;

            for &neighbor in corner.neighbors() {
                let neighbor_elevation = self.corners[neighbor];
                let diff = my_elevation - neighbor_elevation;
                if diff > 0.0 {
                    let update = match slope {
                        None => true,
                        Some(slope) => slope.intensity < diff,
                    };
                    if update {
                        slope = Some(Slope {
                            towards: neighbor,
                            intensity: diff,
                        });
                    }
                }
            }
            slope
        });

        HeightMap {
            corners: self.corners, cells, descent_vector
        }
    }
}
#[derive(Clone)]
pub struct HeightMap {
    pub corners: CornerData<f64>,
    pub cells: CellData<f64>,
    descent_vector: CornerData<Option<Slope>>,
}

#[derive(Clone, Copy)]
struct Slope {
    towards: CornerId,
    intensity: f64,
}

pub struct Rivers {
    flow_volume: CornerData<f64>,
}

impl Rivers {
    pub fn new(poly_map: &PolyMap, height_map: &HeightMap, base_flow: f64) -> Rivers {
        let mut flow_volume = CornerData::for_each(poly_map, |_,_| base_flow);


        Rivers {
            flow_volume
        }
    }
}

pub struct WorldGenerator;

impl WorldGenerator {
    pub fn new() -> Self { Self }

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

            for _ in 0 .. positive_clumps {
                hm.clump(poly_map,0.2, 0.90, 0.05, rng)
            }

            for _ in 0 .. negative_clumps {
                hm.clump(poly_map, -0.2, 0.96, 0.1, rng);
            }

            let num_relax = 3;
            for _ in 0..num_relax {
                hm.relax(poly_map, 0.2);
            }

            hm.normalize();
            hm.build(&poly_map)
        };

        let terrain_types = heightmap.cells
            .transform(|_, &x| TerrainType::from_height(x));

        WorldMap {
            heightmap,
            terrain_types,
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

        LEVELS.iter()
            .find_map(|&(tt, x)| if height <= x { Some(tt) } else { None })
            .expect("Invalid terrain type, heightmap out of bounds")
    }
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
                    TerrainType::DeepWater => Color::DARKBLUE,
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