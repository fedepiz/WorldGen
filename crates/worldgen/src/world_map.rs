use crate::defs::Defs;
use crate::hydrology::*;
use crate::thermology::*;
use crate::heightmap::*;
use crate::{TerrainType, WorldParams};
use crate::generators::*;
use polymap::compute::*;

pub struct WorldMap<'a> {
    pub(crate) defs: Defs<'a>,
    pub(crate) heightmap: HeightMap,
    pub(crate) terrain: CellData<TerrainType>,
    pub(crate) hydrology: Hydrology,
    pub(crate) thermology: Thermolgoy,
}

impl WorldMap<'_> {
    pub fn heightmap(&self) -> &HeightMap {
        &self.heightmap
    }

    pub fn reflow_rivers(&mut self, 
        params: &parameters::Parameters<WorldParams>) {

        let mut hmb = self.heightmap.make_builder();
        
        hmb.add_field(self.defs.poly, &PerlinField::new(0.0, 0.0, 0.001), 0.2);
        hmb.planchon_darboux(self.defs.poly);

        self.heightmap = hmb.build(self.defs.poly);
        self.terrain = CellData::for_each(self.defs.poly, |id, _| {
            self.defs.terrain_type.from_level(self.heightmap.cell_height(id), |x| x.height_level)
        });
        self.hydrology
            .recompute(&self.defs, params, &self.heightmap, &self.terrain);
        self.thermology
            .recompute(&self.defs, &self.heightmap, &self.terrain);
    }
}