#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Biome {
    Underwater,
    // Very Cold
    Tundra,
    // Cold
    BorealForest,
    ColdDesert,
    // Temperate
    TemperateRainforest,
    TemperateDecidiousForest,
    Shrubland,
    TemperateGrassland,
    // Tropicals
    TropicalRainforest,
    Savannah,
    SubtropicalDesert
}

impl Biome {
    const T_VLOW: f64 = 0.1;
    const T_LOW: f64 = 0.3;
    const T_HIGH: f64 = 0.8;

    const P_VLOW: f64 = 0.1;
    const P_LOW: f64 = 0.3;
    const P_HIGH: f64 = 0.8;


    pub fn whittaker(temperature: f64, rain: f64) -> Biome {
        if temperature < Self::T_VLOW {
            Biome::Tundra
        } else if temperature < Self::T_LOW {
            if rain < Self::P_VLOW {
                Biome::ColdDesert
            } else {
                Biome::BorealForest
            }
        } else if temperature < Self::T_HIGH {
            if rain < Self::P_VLOW {
                Self::TemperateGrassland
            } else if rain < Self::P_LOW {
                Self::Shrubland
            } else if rain < Self::P_HIGH {
                Self::TemperateDecidiousForest
            } else {
                Self::TemperateRainforest
            }
        } else {
            if rain < Self::P_VLOW {
                Self::SubtropicalDesert
            } else if rain < Self::P_HIGH {
                Self::Savannah
            } else {
                Self::TropicalRainforest
            }
        }
    }
}