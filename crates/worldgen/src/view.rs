use crate::{WorldMap};
use polymap::map_shader::*;
use polymap::*;



#[derive(Clone, Copy, PartialEq, Eq)]
pub enum ViewMode {
    Terrain,
    Heightmap,
    Hydrology,
    Thermology,
}

impl ViewMode {
    pub const fn name(&self) -> &'static str {
        match self {
            Self::Heightmap => "Heightmap",
            Self::Terrain => "Geology",
            Self::Hydrology => "Hydrology",
            Self::Thermology => "Temperatures",
        }
    }
}

pub struct WorldMapView<'a> {
    world_map: &'a WorldMap,
    mode: ViewMode,
}

impl<'a> WorldMapView<'a> {
    pub fn new(world_map: &'a WorldMap, mode: ViewMode) -> Self {
        Self { world_map, mode }
    }
}

impl<'a> MapShader for WorldMapView<'a> {
    fn cell(&self, id: CellId) -> Color {
        match self.mode {
            ViewMode::Heightmap => {
                let height = self.world_map.heightmap.cell_height(id);
                let intensity = (height as f32).max(0.0).min(1.0);
                Color::new(intensity, intensity, intensity, 1.0)
            }
            ViewMode::Terrain => {

                let terrain_color = |terrain| {
                    self.world_map.defs.terrain_type[terrain].color
                };

                let height = self.world_map.heightmap.cell_height(id);

                let (tlower, theigher, t) = self.world_map.defs.terrain_type
                    .from_level_range(height, |x| x.height_level);
        
                let clower = terrain_color(tlower);
                let chigher = terrain_color(theigher);
                interpolate_colors(clower, chigher, t as f32)
            }
            ViewMode::Hydrology => {
                let rainfall = self.world_map.hydrology.cell_rainfall(id);
                Color::new(0.0, 0.0, 1.0, rainfall.min(1.0) as f32)
            }
            ViewMode::Thermology => {
                let temperature = self.world_map.thermology.cell_temperature(id);

                let t_value = temperature.max(0.0).min(1.0) as f32;
                interpolate_three_colors(colors::DARKBLUE, colors::YELLOW, colors::RED, t_value)
            }
        }
    }

    fn edge(&self, id: polymap::EdgeId, _: &Edge) -> Option<Color> {
        match self.mode {
            ViewMode::Heightmap => Some(colors::BLACK),
            ViewMode::Terrain => {
                if !self.world_map.hydrology.rivers().is_segment(id) {
                    return None;
                }
                let flow = self.world_map.hydrology.edge_flux(id);
                Some(Color::new(0.0, 0.0, 1.0, flow.min(1.0) as f32))
            }
            ViewMode::Hydrology => {
                let flow = self.world_map.hydrology.edge_flux(id);
                Some(Color::new(0.0, 0.0, 0.0, flow.min(1.0) as f32))
            }
            ViewMode::Thermology => None,
        }
    }

    fn draw_vertices(&self) -> bool {
        match self.mode {
            ViewMode::Heightmap => true,
            ViewMode::Terrain => false,
            ViewMode::Hydrology => true,
            ViewMode::Thermology => false,
        }
    }

    fn vertex(&self, id: VertexId, vertex: &Vertex) -> Option<Color> {
        match self.mode {
            ViewMode::Heightmap => {
                let has_slope = self.world_map.heightmap.descent_vector(id).is_some();
                if !has_slope && !vertex.is_border() {
                    Some(colors::RED)
                } else {
                    None
                }
            }
            ViewMode::Terrain => None,
            ViewMode::Hydrology => {
                let rivers = self.world_map.hydrology.rivers();

                if rivers.is_source(id) {
                    Some(colors::GREEN)
                } else if rivers.is_sink(id) {
                    Some(colors::RED)
                } else {
                    None
                }
            }
            ViewMode::Thermology => None,
        }
    }
}

fn interpolate_three_colors(c1: Color, c2: Color, c3: Color, t: f32) -> Color {
    if t <= 0.5 {
        interpolate_colors(c1, c2, 2. * t)
    } else {
        interpolate_colors(c2, c3, 2. * (t - 0.5))
    }
}

fn interpolate_colors(c1: Color, c2: Color, t: f32) -> Color {
    Color::new(
        lerp8(c1.r, c2.r, t),
        lerp8(c1.g, c2.g, t),
        lerp8(c1.b, c2.b, t),
        lerp8(c1.a, c2.a, t),
    )
}

fn lerp8(a: f32, b: f32, t: f32) -> f32 {
    ((1.0 - t) * a) + (t * b)
}
