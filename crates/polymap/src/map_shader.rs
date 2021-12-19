use super::*;

pub use raylib::color::Color;

pub trait MapShader {
    fn cell(&self, id: CellId) -> Color;
    fn edge(&self, id: EdgeId, edge: &Edge) -> Option<Color>;
    fn draw_corners(&self) -> bool;
    fn corner(&self, id: CornerId, corner: &Corner) -> Option<Color>;
}

use rand::Rng;
use rand::{prelude::SmallRng, SeedableRng};

#[allow(unused)]
pub struct RandomColorShader {
    colors: Vec<Color>,
}

#[allow(unused)]
impl RandomColorShader {
    pub fn new(poly: &PolyMap) -> Self {
        let mut rng = SmallRng::from_entropy();
        let colors: Vec<_> = poly
            .cells
            .iter()
            .map(|_| Color::new(rng.gen(), rng.gen(), rng.gen(), u8::MAX))
            .collect();
        Self { colors }
    }
}
