use finvec::FinDef;
use polymap::PolyMap;

use crate::{TerrainType, TerrainTypeData};

pub(crate) struct Defs<'a> {
    pub(crate) poly: &'a PolyMap,
    pub(crate) terrain_type: FinDef<TerrainType, TerrainTypeData>,
}

impl <'a> Defs<'a> {
    pub fn new(poly: &'a PolyMap) -> Self {
        Defs {
            poly,
            terrain_type: TerrainType::default_definition()
        }
    }
}