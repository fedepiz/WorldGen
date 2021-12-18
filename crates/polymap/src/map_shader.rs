use super::*;
use super::element_set::ElementSet;

pub use raylib::color::Color;

pub trait MapShader {
    fn cell(&self, id: CellId) -> Color;
    fn edge(&self, id: EdgeId) -> Color;
}

use rand::{prelude::SmallRng, SeedableRng};
use rand::Rng;

#[allow(unused)]
pub struct RandomColorShader {
    colors: Vec<Color>,
}

#[allow(unused)]
impl RandomColorShader {
    pub fn new(poly: &PolyMap) -> Self {
        let mut rng = SmallRng::from_entropy();
        let colors:Vec<_> = poly.cells.iter().map(|_| Color::new(rng.gen(),rng.gen(),rng.gen(),u8::MAX)).collect();
        Self {
            colors,
        }
    }
}

impl MapShader for RandomColorShader {
    fn cell(&self, id: CellId) -> Color {
        self.colors.get(id.0).copied().unwrap_or_else(|| Color::WHITE)
    }

    fn edge(&self, _: EdgeId) -> Color {
        Color::WHITE
    }
}

pub struct HighlightShader(pub ElementSet);

impl HighlightShader {
    #[allow(unused)]
    pub fn new() -> Self {
        Self(ElementSet::new())
    }
}

impl MapShader for HighlightShader {
    fn cell(&self, id: CellId) -> Color {
        if self.0.cells.contains(&id) { Color::RED } else { Color::WHITE }
    }

    fn edge(&self, id: EdgeId) -> Color {
        if self.0.edges.contains(&id) { Color::YELLOW } else { Color::BLACK }
    }
}