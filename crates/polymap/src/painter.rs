use raylib::prelude::*;

use super::element_set::ElementSet;
use super::map_shader::MapShader;
use super::*;

type Handle<'a> = RaylibTextureMode<'a, RaylibDrawHandle<'a>>;

pub struct Painter {
    texture: RenderTexture2D,
    validation: Validation,
}

impl Painter {
    pub fn new(
        rl: &mut RaylibHandle,
        thread: &RaylibThread,
        polymap: &PolyMap,
    ) -> Result<Self, String> {
        let texture =
            rl.load_render_texture(thread, polymap.width as u32, polymap.height as u32)?;

        Ok(Self {
            texture,
            validation: Validation::Invalid,
        })
    }

    pub fn draw(
        &mut self,
        ctx: &mut RaylibDrawHandle,
        thread: &RaylibThread,
        x: i32,
        y: i32,
        poly_map: &PolyMap,
        shader: &impl MapShader,
    ) {
        if !self.validation.is_valid() {
            self.draw_all(ctx, thread, poly_map, shader);
            self.validation = Validation::Valid;
        }

        ctx.draw_texture(&self.texture, x, y, Color::WHITE);
    }

    pub fn invalidate(&mut self, validation: Validation) {
        self.validation.join(validation)
    }

    pub fn draw_all(
        &mut self,
        ctx: &mut RaylibDrawHandle,
        thread: &RaylibThread,
        poly_map: &PolyMap,
        shader: &impl MapShader,
    ) {
        let mut tctx = ctx.begin_texture_mode(&thread, &mut self.texture);

        tctx.draw_rectangle(
            0,
            0,
            poly_map.width as i32,
            poly_map.height as i32,
            Color::WHITE,
        );

        {
            Self::draw_cells(&mut tctx, poly_map, shader, &self.validation);
            Self::draw_edges(&mut tctx, poly_map, shader);

            if shader.draw_corners() {
                Self::draw_corners(&mut tctx, poly_map, shader);
            }
        };
    }

    fn draw_cells(
        ctx: &mut Handle,
        poly_map: &PolyMap,
        shader: &impl MapShader,
        validation: &Validation,
    ) {
        use lyon::math::Point;
        use lyon::path::builder::*;
        use lyon::tessellation::geometry_builder::simple_builder;
        use lyon::tessellation::{FillOptions, FillTessellator, VertexBuffers};

        let mut geometry = VertexBuffers::<Point, u16>::new();
        {
            let options = FillOptions::tolerance(0.1);
            let mut tessellator = FillTessellator::new();
            for (id, cell) in poly_map.cells() {
                if !validation.cell_is_invalid(id) {
                    continue;
                }

                let points: Vec<_> = cell
                    .polygon
                    .exterior()
                    .points_iter()
                    .map(|p| lyon::geom::point(p.x() as f32, poly_map.height as f32 - p.y() as f32))
                    .collect();
                let polygon = lyon::path::Polygon {
                    points: points.as_slice(),
                    closed: true,
                };

                geometry.vertices.clear();
                geometry.indices.clear();
                let mut geometry_builder = simple_builder(&mut geometry);
                let mut builder = tessellator.builder(&options, &mut geometry_builder);
                builder.add_polygon(polygon);
                builder.build().unwrap();

                let color = shader.cell(id);

                for triangle in geometry.indices.chunks(3) {
                    let make_point = |idx| {
                        let v: &lyon::math::Point = &geometry.vertices[idx as usize];
                        Vector2::new(v.x, v.y)
                    };

                    ctx.draw_triangle(
                        make_point(triangle[0]),
                        make_point(triangle[1]),
                        make_point(triangle[2]),
                        color,
                    )
                }
            }
        }
    }

    fn draw_edges(ctx: &mut Handle, poly_map: &PolyMap, shader: &impl MapShader) {
        for (id, edge) in poly_map.edges() {
                if let Some(color) = shader.edge(id, edge) {
                let ((ax, ay), (bx, by)) = poly_map.edge_endpoints_coords(edge);
                let start = Vector2::new(ax as f32, poly_map.height as f32 - ay as f32);
                let end = Vector2::new(bx as f32, poly_map.height as f32 - by as f32);

                ctx.draw_line_ex(start, end, 1.0, color);
            }
        }
    }

    fn draw_corners(ctx: &mut Handle, poly_map: &PolyMap, shader: &impl MapShader) {
        for (id, corner) in poly_map.corners() {
            if let Some(color) = shader.corner(id, corner) {
                let tile_halfsize = 2.0;

                let half_size = Vector2::zero() + tile_halfsize;
                let position = Vector2::new(
                    corner.x() as f32,
                    poly_map.height as f32 - corner.y() as f32,
                ) - half_size;
                let size = half_size * 2.0;
    
                ctx.draw_rectangle_v(position, size, color);
            }            
        }
    }
}

pub enum Validation {
    Valid,
    Invalid,
    Partial(ElementSet),
}

impl Validation {
    fn is_valid(&self) -> bool {
        match self {
            Self::Valid => true,
            _ => false,
        }
    }

    fn join(&mut self, other: Validation) {
        match self {
            Self::Valid => *self = other,
            Self::Invalid => {}
            Self::Partial(partial_invalidation) => match other {
                Self::Invalid => *self = Validation::Invalid,
                Self::Valid => {}
                Self::Partial(other_invalidation) => {
                    partial_invalidation.join(&other_invalidation);
                }
            },
        }
    }

    fn cell_is_invalid(&self, id: CellId) -> bool {
        match self {
            Self::Valid => false,
            Self::Invalid => true,
            Self::Partial(elements) => elements.cells.contains(&id),
        }
    }
}
