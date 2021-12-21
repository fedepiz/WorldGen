use super::*;

pub use macroquad::prelude::Color;

pub mod colors {
    pub use macroquad::color::*;
}

pub trait MapShader {
    fn cell(&self, id: CellId) -> Color;
    fn edge(&self, id: EdgeId, edge: &Edge) -> Option<Color>;
    fn draw_vertices(&self) -> bool;
    fn vertex(&self, id: VertexId, corner: &Vertex) -> Option<Color>;
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
            .map(|_| Color::new(rng.gen(), rng.gen(), rng.gen(), 1.0))
            .collect();
        Self { colors }
    }
}
