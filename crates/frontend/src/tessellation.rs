use lyon::{lyon_tessellation::VertexBuffers, geom::euclid::{Point2D, UnknownUnit}};
use macroquad::prelude as mq;
use polymap::{PolyMap, CellId};


pub type Triangle = [mq::Vec2; 3];
pub struct GridTessellation {
    cells: Vec<Vec<Triangle>>,
}

impl GridTessellation {
    pub fn new(poly: &PolyMap) -> Self {
        use lyon::math::Point;
        use lyon::path::builder::*;
        use lyon::tessellation::geometry_builder::simple_builder;
        use lyon::tessellation::{FillOptions, FillTessellator};

        let mut cells = vec![];
        let mut geometry = VertexBuffers::<Point, u16>::new();
        {
            let options = FillOptions::tolerance(0.1);
            let mut tessellator = FillTessellator::new();
            for (_, cell) in poly.cells() {
                let points: Vec<_> = cell
                    .polygon()
                    .exterior()
                    .points_iter()
                    .map(|p| lyon::geom::point(p.x() as f32, poly.height() as f32 - p.y() as f32))
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
                        mq::Vec2::new(v.x, v.y)
                    };
                    triangles.push([make_vertex(0), make_vertex(1), make_vertex(2)]);
                }
                cells.push(triangles);
            }
        }
        Self { cells }
    }

    pub fn polygon_of(&self, id:CellId) -> &[Triangle] {
        self.cells[id.idx()].as_slice()
    }
}


fn geometry_to_triangles(geometry: &VertexBuffers<Point2D<f32, UnknownUnit>, u16>) -> impl Iterator<Item=[mq::Vec2;3]> + '_ {
    geometry.indices.chunks_exact(3).map(|triangle| {
        let make_vertex = |idx| {
            let v: &lyon::math::Point = &geometry.vertices[triangle[idx] as usize];
            mq::Vec2::new(v.x, v.y)
        };
        [make_vertex(0), make_vertex(1), make_vertex(2)]
    })
}

pub struct PathTessellation {
    triangles: Vec<[mq::Vec2; 3]> 
}

impl PathTessellation {
    pub fn with_points(points: &[(f64, f64)], thickness: f32, closed: bool) -> Option<Self>{
        if points.len() < 2 {
            return None
        }
    
        use lyon::math::Point;
        use lyon::path::Path;
        use lyon::tessellation::geometry_builder::simple_builder;
        use lyon::tessellation::{StrokeOptions, StrokeTessellator};
    
        let mut geometry = VertexBuffers::<Point, u16>::new();
        {
            let mut geometry_builder = simple_builder(&mut geometry);
            let mut path_builder = Path::builder();
    
            path_builder.begin(Point::new(points[0].0 as f32, points[0].1 as f32));
            
            for i in 1 .. points.len() - 1 {
                let pt = Point::new(points[i].0 as f32, points[i].1 as f32);
                let pt2 = Point::new(points[i+1].0 as f32, points[i+1].1 as f32);
    
                let xc = (pt.x + pt2.x) / 2.0;
                let yc = (pt.y + pt2.y) / 2.0;
    
                path_builder.quadratic_bezier_to(pt, Point::new(xc, yc));
            }
    
            let i = points.len()-2;
            path_builder.quadratic_bezier_to(
                Point::new(points[i].0 as f32, points[i].1 as f32), 
                Point::new(points[i+1].0 as f32, points[i+1].1 as f32));
    
            path_builder.end(closed);
            
            let path = path_builder.build();
        
            let mut tessellator = StrokeTessellator::new();
            let options = StrokeOptions::default().with_line_width(thickness);
            tessellator.tessellate_path(&path, &options, &mut geometry_builder).unwrap();
        }

        let triangles = geometry_to_triangles(&geometry).collect();
        Some(Self { triangles })
    }

    pub fn path_of_cells(poly: &PolyMap, cells: &[CellId], thickness: f32) -> Option<Self> {
        let closed = cells.first() == cells.last();
        let centers:Vec<_> = cells.iter().map(|&id| {
            let (x, y) = poly[id].center();
            (x, poly.height() as f64 - y)
        }).collect();
        Self::with_points(centers.as_slice(), thickness, closed)
    }

    pub fn polygon(&self) -> &[Triangle] {
        &self.triangles
    }
}