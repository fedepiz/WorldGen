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
    Precipitation,
    Drainage,
    Biome,
}


impl ViewMode {

    pub fn name(&self) -> &'static str {
        match self {
            ViewMode::Heightmap => "Heightmap",
            ViewMode::Geography => "Geography",
            ViewMode::Temperature => "Temperature",
            ViewMode::Precipitation => "Precipitation",
            ViewMode::Drainage => "Drainage",
            ViewMode::Biome => "Biome",
        }
    }

    fn draw_cell(&self, world:&World, cell: CellId) -> DrawCell {
        match self {
            &ViewMode::Heightmap => {
                let height = world.heightmap()[cell] as f32;
                
                let color = mq::Color::new(height,height, height, 1.0);

                DrawCell {
                    color,
                    direction: None,
                }
            }
            &ViewMode::Geography => {
                let terrain_category = world.terrain_category()[cell];
                let color = match terrain_category {
                    TerrainCategory::Land => {
                        let t = (world.heightmap()[cell] - 0.5) * 2.0;
                        colors::interpolate_three_colors(mq::GREEN, mq::BROWN, mq::WHITE, t as f32)
                    }
                    TerrainCategory::Coast => mq::SKYBLUE,
                    TerrainCategory::Sea => mq::BLUE,
                };
                DrawCell {
                    color,
                    direction: None,
                }
            }
            &ViewMode::Temperature => {
                let temperature = world.temperature()[cell] as f32;
                let color = colors::interpolate_three_colors(mq::BLUE, mq::YELLOW, mq::RED, temperature);
                DrawCell {
                    color,
                    direction: None,
                }
            }
            &ViewMode::Precipitation => {
                let rain = world::measure::DRAIN.normalize(world.rainfall()[cell]);
                
                let color = mq::Color::new(0.0, 0.0, 1.0, rain as f32);

                let wind_vector = &world.wind()[cell].to_polar().unwrap_or_default();
                let direction = if wind_vector.r == 0.0 { None} else{ 
                    let rain = world::measure::RAIN.normalize(wind_vector.r);
                    let color = mq::Color::new(rain as f32, 0.0, 0.0, rain as f32);
                    Some((color, wind_vector.theta)) 
                };

                DrawCell {
                    color,
                    direction,
                }
            }
            &ViewMode::Drainage => {
                let drainage = world::measure::DRAIN.normalize(world.drainage()[cell]) as f32;
                let color = mq::Color::new(0.0, 0.0, 1.0, drainage);

                let direction = if world.is_river(cell) {
                    match world.downhill()[cell] {
                        CellVector::Stationary => None,
                        CellVector::Towards(tgt, _) => {
                            let angle = world.poly().angle_between_cells(cell, tgt);
                            Some((mq::WHITE, angle))
                        }
    
                    }
                } else {
                    None
                };

                DrawCell {
                    color,
                    direction,
                }
            }
            &ViewMode::Biome => {
                let color = biome_color(world.biome()[cell]);
                DrawCell {
                    color,
                    direction: None,
                }
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
            &ViewMode::Drainage => {
                world.rivers().iter().map(|path| 
                    (path.cells().iter().copied().collect(), mq::BLACK)
                ).collect()
            }
            _ => vec![]
        }
    }
}

struct DrawCell {
    color: mq::Color,
    direction: Option<(mq::Color, f64)>,
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
        
        for (cell_id, cell) in world.poly().cells() {
            let triangles = self.tessellation.polygon_of(cell_id);
            let drawing = mode.draw_cell(world, cell_id);
            for triangle in triangles {
                mq::draw_triangle(triangle[0], triangle[1], triangle[2], drawing.color);
            }

            if let Some((color, direction)) = drawing.direction {
                let (cx, cy) = cell.center();
                let triangle = rotated_triangle((cx, world.poly().height() as f64 - cy), 5.0, direction);

                mq::draw_triangle(triangle[0], triangle[1], triangle[2], color)
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

fn rotated_triangle(center:(f64, f64), height: f64, direction: f64) -> [mq::Vec2; 3] {
    let (cx, cy) = center;
    let h = height;
    let t = direction;
    let p_top = mq::Vec2::new((cx + h * t.cos()) as f32, (cy + h * t.sin()) as f32);

    let tr = t +  f64::to_radians(90.0);
    let p_right = mq::Vec2::new((cx + h * tr.cos()) as f32, (cy + h * tr.sin()) as f32);

    let tl = t +  f64::to_radians(-90.0);
    let p_left = mq::Vec2::new((cx + h * tl.cos()) as f32, (cy + h * tl.sin()) as f32);

    [p_top, p_left, p_right]
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

fn biome_color(biome: Biome) -> mq::Color {
    match biome {
        Biome::Underwater => mq::BLUE,
        // Very Cold
        Biome::Tundra => mq::BEIGE,
        // Cold
        Biome::BorealForest => mq::DARKGREEN,
        Biome::ColdDesert => mq::GRAY,
        // Temperate
        Biome::TemperateRainforest => mq::GREEN,
        Biome::TemperateDecidiousForest => mq::GREEN,
        Biome::Shrubland => mq::MAROON,
        Biome::TemperateGrassland => mq::GREEN,
        // Tropicals
        Biome::TropicalRainforest => mq::DARKGREEN,
        Biome::Savannah => mq::MAROON,
        Biome::SubtropicalDesert => mq::YELLOW,
    }
}