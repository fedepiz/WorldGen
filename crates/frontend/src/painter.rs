use macroquad::prelude as mq;
use polymap::*;
use world::*;

use crate::tessellation::{GridTessellation, PathTessellation};

use strum_macros::EnumIter;

#[derive(Clone, Copy, PartialEq, Eq, EnumIter)]
pub enum ViewMode {
    Heightmap,
    Geography,
    Temperature,
    Hydrology,
}


impl ViewMode {

    pub fn name(&self) -> &'static str {
        match self {
            ViewMode::Heightmap => "Heightmap",
            ViewMode::Geography => "Geography",
            ViewMode::Temperature => "Temperature",
            ViewMode::Hydrology => "Hydrology"
        }
    }

    fn color(&self, world:&World, cell: CellId) -> mq::Color {
        match self {
            &ViewMode::Heightmap => {
                let height = world.heightmap()[cell] as f32;
                mq::Color::new(height,height, height, 1.0)
            }
            &ViewMode::Geography => {
                let terrain_category = world.terrain_category()[cell];
                match terrain_category {
                    TerrainCategory::Land => {
                        let t = (world.heightmap()[cell] - 0.5) * 2.0;
                        colors::interpolate_three_colors(mq::GREEN, mq::BROWN, mq::WHITE, t as f32)
                    }
                    TerrainCategory::Coast => mq::SKYBLUE,
                    TerrainCategory::Sea => mq::BLUE,
                }
            }
            &ViewMode::Temperature => {
                let temperature = world.temperature()[cell] as f32;
                colors::interpolate_three_colors(mq::BLUE, mq::YELLOW, mq::RED, temperature)
            }
            &ViewMode::Hydrology => {
                let drainage = world.drainage()[cell] as f32;
                mq::Color::new(0.0, 0.0, 1.0, drainage)
            }
        }
    }

    fn paths(&self, world:&World) -> Vec<(Vec<CellId>, mq::Color)> {
        match self {
            ViewMode::Geography => {
                world.rivers().iter().map(|path| 
                    (path.cells().iter().copied().collect(), mq::BLUE)
                ).collect()
            },
            &ViewMode::Hydrology => {
                world.rivers().iter().map(|path| 
                    (path.cells().iter().copied().collect(), mq::BLACK)
                ).collect()
            }
            _ => vec![]
        }
    }
}
pub struct Painter {
    target: mq::RenderTarget,
    tessellation: GridTessellation,
}

impl Painter {
    pub fn new(poly: &PolyMap) -> Self {
        Self {
            target: mq::render_target(poly.width() as u32, poly.height() as u32),
            tessellation: GridTessellation::new(poly),
        }
    }

    pub fn update(&mut self, world: &World, mode: ViewMode) {
        let display_rect = mq::Rect::new(0.0, 0.0, world.poly().width() as f32, world.poly().height() as f32);
        let mut camera = mq::Camera2D::from_display_rect(display_rect);
        camera.render_target = Some(self.target);
        mq::push_camera_state();
        mq::set_camera(&camera);

        mq::draw_rectangle(0.0,0.0, world.poly().width() as f32, world.poly().height() as f32, mq::BLACK);
        
        for (cell, _) in world.poly().cells() {
            let triangles = self.tessellation.polygon_of(cell);
            let color = mode.color(world, cell);
            for triangle in triangles {
                mq::draw_triangle(triangle[0], triangle[1], triangle[2], color);
            }
        }
      
        for (path, color) in mode.paths(world) {
            let tess = PathTessellation::path_of_cells(world.poly(), path.as_slice(), 2.0).unwrap();
            for triangle in tess.polygon() {
                mq::draw_triangle(triangle[0], triangle[1], triangle[2], color)
            }
        }
    
        mq::pop_camera_state();
    }

    pub fn draw(&mut self) {
        let mut params = mq::DrawTextureParams::default();
        params.dest_size = Some(mq::Vec2::new(mq::screen_width(), mq::screen_height()));
        mq::draw_texture_ex(self.target.texture, 0.0, 0.0, mq::WHITE, params);
    }
}


 
mod colors {
    use macroquad::prelude::*;

    pub fn interpolate_three_colors(c1: Color, c2: Color, c3: Color, t: f32) -> Color {
        if t <= 0.5 {
            interpolate_colors(c1, c2, 2. * t)
        } else {
            interpolate_colors(c2, c3, 2. * (t - 0.5))
        }
    }

    pub fn interpolate_colors(c1: Color, c2: Color, t: f32) -> Color {
        Color::new(
            lerp8(c1.r, c2.r, t),
            lerp8(c1.g, c2.g, t),
            lerp8(c1.b, c2.b, t),
            lerp8(c1.a, c2.a, t),
        )
    }

    pub fn lerp8(a: f32, b: f32, t: f32) -> f32 {
        ((1.0 - t) * a) + (t * b)
    }
}
