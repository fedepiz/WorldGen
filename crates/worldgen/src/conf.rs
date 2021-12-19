use serde_derive::Deserialize;

#[derive(Deserialize)]
pub struct WorldGenConf {
    pub heightmap: HeightMap,
    pub hydrology: Hydrology,
}

#[derive(Deserialize)]
pub struct HeightMap {
    pub planchon_darboux: bool,
    pub slopes: NumberIntensity,
    pub clumps: NumberIntensity,
    pub depressions: NumberIntensity,
    pub perlin1: PerlinConf,
    pub perlin2: PerlinConf,
}


#[derive(Deserialize)]
pub struct Hydrology {
   pub min_river_flux: f64,
   pub rain: RainConf,
}
#[derive(Deserialize)]
pub struct RainConf {
    pub height_coeff: f64,
    pub perlin: PerlinConf,
}

#[derive(Deserialize)]
pub struct NumberIntensity {
    pub number: usize,
    pub intensity: f64,
}

#[derive(Deserialize)]
pub struct PerlinConf {
    pub frequency: f64,
    pub intensity: f64,
}

