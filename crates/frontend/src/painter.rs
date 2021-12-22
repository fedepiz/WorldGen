use macroquad::prelude::{self as mq, Vec2};

use polymap::map_shader::MapShader;
use polymap::*;

pub struct Painter {
    render_target: mq::RenderTarget,
    validation: Validation,
    tessellation: Tessellation,
}

impl Painter {
    pub fn new(polymap: &PolyMap) -> Result<Self, String> {
        let texture = mq::render_target(polymap.width() as u32, polymap.height() as u32);

        let tessellation = Tessellation::new(polymap);

        Ok(Self {
            render_target: texture,
            tessellation,
            validation: Validation::Invalid,
        })
    }

    pub fn draw(&mut self, x: f32, y: f32, poly_map: &PolyMap, shader: &impl MapShader) {
        if !self.validation.is_valid() {
            self.draw_all(poly_map, shader);
            self.validation = Validation::Valid;
        }

        let mut params = mq::DrawTextureParams::default();
        params.dest_size = Some(Vec2::new(mq::screen_width(), mq::screen_height()));
        mq::draw_texture_ex(self.render_target.texture, x, y, mq::WHITE, params);
    }

    pub fn invalidate(&mut self, validation: Validation) {
        self.validation.join(validation)
    }

    pub fn draw_all(&mut self, poly_map: &PolyMap, shader: &impl MapShader) {
        let mut camera = mq::Camera2D::from_display_rect(mq::Rect::new(0.0, 0.0, 1600.0, 900.0));
        camera.render_target = Some(self.render_target);
        mq::set_camera(&camera);

        mq::draw_rectangle(
            0.0,
            0.0,
            poly_map.width() as f32,
            poly_map.height() as f32,
            mq::WHITE,
        );

        {
            self.tessellation.draw(&poly_map, shader);
            Self::draw_edges(poly_map, shader);

            if shader.draw_vertices() {
                Self::draw_vertices(poly_map, shader);
            }
        };

        mq::set_default_camera()
    }

    fn draw_edges(poly_map: &PolyMap, shader: &impl MapShader) {
        for (id, edge) in poly_map.edges() {
            if let Some(color) = shader.edge(id, edge) {
                let ((ax, ay), (bx, by)) = poly_map.edge_endpoints_coords(edge);
                let start = Vec2::new(ax as f32, poly_map.height() as f32 - ay as f32);
                let end = Vec2::new(bx as f32, poly_map.height() as f32 - by as f32);

                mq::draw_line(start.x, start.y, end.x, end.y, 1.0, color);
            }
        }
    }

    fn draw_vertices(poly_map: &PolyMap, shader: &impl MapShader) {
        for (id, corner) in poly_map.vertices() {
            if let Some(color) = shader.vertex(id, corner) {
                let tile_halfsize = 2.0;

                let half_size = Vec2::ZERO + Vec2::new(tile_halfsize, tile_halfsize);
                let position = Vec2::new(
                    corner.x() as f32,
                    poly_map.height() as f32 - corner.y() as f32,
                ) - half_size;
                let size = half_size * 2.0;

                mq::draw_rectangle(position.x, position.y, size.x, size.y, color);
            }
        }
    }
}

pub enum Validation {
    Valid,
    Invalid,
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
        }
    }
}

struct Tessellation {
    cells: Vec<Vec<[Vec2; 3]>>,
}

impl Tessellation {
    pub fn new(poly_map: &PolyMap) -> Self {
        use lyon::math::Point;
        use lyon::path::builder::*;
        use lyon::tessellation::geometry_builder::simple_builder;
        use lyon::tessellation::{FillOptions, FillTessellator, VertexBuffers};

        let mut cells = vec![];
        let mut geometry = VertexBuffers::<Point, u16>::new();
        {
            let options = FillOptions::tolerance(0.1);
            let mut tessellator = FillTessellator::new();
            for (_, cell) in poly_map.cells() {
                let points: Vec<_> = cell
                    .polygon()
                    .exterior()
                    .points_iter()
                    .map(|p| lyon::geom::point(p.x() as f32, poly_map.height() as f32 - p.y() as f32))
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

                let mut triangles = vec![];
                for triangle in geometry.indices.chunks(3) {
                    let make_vertex = |idx| {
                        let v: &lyon::math::Point = &geometry.vertices[triangle[idx] as usize];
                        Vec2::new(v.x, v.y)
                    };
                    triangles.push([make_vertex(0), make_vertex(1), make_vertex(2)]);
                }
                cells.push(triangles);
            }
        }
        Self { cells }
    }

    pub fn draw<'a>(&self, poly_map: &PolyMap, shader: &impl MapShader) {
        for ((id, _), triangles) in poly_map.cells().zip(self.cells.iter()) {
            for triangle in triangles {
                let color = shader.cell(id);
                mq::draw_triangle(triangle[0], triangle[1], triangle[2], color);
            }
        }
    }
}
