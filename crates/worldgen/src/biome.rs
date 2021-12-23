use finvec::*;

fin_idx!(pub BiomeTypeId);

#[derive(Clone)]
pub struct BiomeType {
    pub name: String,
    pub is_water: bool,
}
