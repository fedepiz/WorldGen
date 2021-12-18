pub mod compute;
pub mod element_set;
pub mod map_shader;
pub mod painter;

use geo::{contains::Contains, coords_iter::CoordsIter, Polygon};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, PartialOrd)]
struct Location(f64, f64);

impl From<geo::Coordinate<f64>> for Location {
    fn from(coord: geo::Coordinate<f64>) -> Self {
        Location(coord.x, coord.y)
    }
}

impl Location {
    fn key(&self) -> (u64, u64) {
        (self.0.to_bits(), self.1.to_bits())
    }
}

impl std::hash::Hash for Location {
    fn hash<H>(&self, state: &mut H)
    where
        H: std::hash::Hasher,
    {
        self.key().hash(state)
    }
}

impl PartialEq for Location {
    fn eq(&self, other: &Location) -> bool {
        self.key() == other.key()
    }
}

impl Eq for Location {}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CellId(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct CornerId(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeId(usize);

pub struct Cell {
    polygon: Polygon<f64>,
    edges: Vec<EdgeId>,
    corners: Vec<CornerId>,
    neighbors: Vec<CellId>,
}

impl Cell {
    fn new(polygon: Polygon<f64>, edges: Vec<EdgeId>, corners: Vec<CornerId>) -> Self {
        Self {
            polygon,
            edges,
            corners,
            neighbors: vec![],
        }
    }

    pub fn corners(&self) -> &[CornerId] {
        self.corners.as_slice()
    }

    pub fn neighbors(&self) -> &[CellId] {
        self.neighbors.as_slice()
    }
}

pub struct Corner {
    coords: (f64, f64),
    edges: Vec<EdgeId>,
    neighbors: Vec<CornerId>,
}

impl Corner {
    fn new(edge: EdgeId, location: Location) -> Self {
        Self {
            coords: (location.0, location.1),
            edges: vec![edge],
            neighbors: vec![],
        }
    }

    pub fn x(&self) -> f64 {
        self.coords.0
    }
    pub fn y(&self) -> f64 {
        self.coords.1
    }

    pub fn neighbors(&self) -> &[CornerId] {
        self.neighbors.as_slice()
    }
}

pub struct Edge {
    endpoints: OrderedPair<CornerId>,
    cells: Vec<CellId>,
}

impl Edge {
    fn new(c1: CornerId, c2: CornerId) -> Self {
        Self {
            endpoints: OrderedPair::new(c1, c2),
            cells: vec![],
        }
    }

    fn add_owner(&mut self, cell: CellId) {
        self.cells.push(cell)
    }
}

#[derive(Clone, Copy, PartialEq, PartialOrd, Eq, Ord, Hash)]
pub struct OrderedPair<T> {
    min: T,
    max: T,
}

impl<T: PartialOrd> OrderedPair<T> {
    pub fn new(a: T, b: T) -> Self {
        if a <= b {
            OrderedPair { min: a, max: b }
        } else {
            OrderedPair { min: b, max: a }
        }
    }
}

pub struct PolyMap {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    edges: Vec<Edge>,
    corners: Vec<Corner>,
    cell_quadtree: quadtree_rs::Quadtree<u64, CellId>,
}

impl PolyMap {
    pub fn new(width: usize, height: usize, poisson_radius: f64) -> Self {
        let centers: Vec<_> = fast_poisson::Poisson2D::new()
            .with_dimensions([width as f64, height as f64], poisson_radius)
            .generate()
            .into_iter()
            .map(|x| (x[0], x[1]))
            .collect();

        let voronoi = voronator::VoronoiDiagram::<voronator::delaunator::Point>::from_tuple(
            &(-1.0, -1.0),
            &(width as f64, height as f64),
            &centers,
        )
        .expect("Failed to build voronoi diagram");

        let polygons: Vec<_> = voronoi
            .cells()
            .iter()
            .map(|poly| {
                let exterior: Vec<_> = poly.points().iter().map(|p| (p.x, p.y)).collect();
                geo::Polygon::new(geo::LineString::from(exterior), vec![])
            })
            .collect();

        let (mut corners, edges, mut cells) = Self::build_elements(polygons);

        // NOTE: Some indices point beyond the number of cells. These are meant
        // for the borders, I think. Skip them
        for (idx, cell) in cells.iter_mut().enumerate() {
            cell.neighbors = voronoi.neighbors[idx]
                .iter()
                .copied()
                .filter(|idx| idx < &centers.len())
                .map(|idx| CellId(idx))
                .collect();
        }

        // Use the corner edges to get their neighbors
        for (idx, corner) in corners.iter_mut().enumerate() {
            for edge in corner.edges.iter() {
                let edge = &edges[edge.0];
                if edge.endpoints.min.0 == idx {
                    corner.neighbors.push(edge.endpoints.max)
                } else {
                    corner.neighbors.push(edge.endpoints.min);
                }
            }
        }

        let cell_quadtree = {
            let mut cell_quadtree = {
                let depth = ((width.max(height) as f64).log2().round()) as usize + 1;
                quadtree_rs::Quadtree::new(depth)
            };

            for (idx, cell) in cells.iter().enumerate() {
                let area = Self::polygon_area(&cell.polygon);
                cell_quadtree.insert(area, CellId(idx));
            }
            cell_quadtree
        };

        PolyMap {
            width,
            height,
            cells,
            edges,
            corners,
            cell_quadtree,
        }
    }

    fn build_elements(polygons: Vec<Polygon<f64>>) -> (Vec<Corner>, Vec<Edge>, Vec<Cell>) {
        let mut edges: Vec<Edge> = vec![];
        let mut corners: Vec<Corner> = vec![];
        let mut cells: Vec<Cell> = vec![];

        let mut edges_lookup: HashMap<_, EdgeId> = HashMap::new();
        let mut corners_lookup: HashMap<_, CornerId> = HashMap::new();

        for (cell_id, polygon) in polygons.into_iter().enumerate() {
            let mut cell_edges = vec![];

            let mut add_corner =
                |edge_id: EdgeId, location: Location| match corners_lookup.get(&location) {
                    Some(&id) => {
                        let corner = &mut corners[id.0];
                        corner.edges.push(edge_id);
                        id
                    }
                    None => {
                        let id = CornerId(corners.len());
                        corners_lookup.insert(location, id);
                        corners.push(Corner::new(edge_id, location));
                        id
                    }
                };

            let mut cell_corners = vec![];

            let mut add_edge = |cell_id: CellId, line: &geo::Line<f64>| {
                let endpoints =
                    OrderedPair::new(Location::from(line.start), Location::from(line.end));
                match edges_lookup.get(&endpoints) {
                    Some(&edge_id) => {
                        let edge = &mut edges[edge_id.0];
                        edge.add_owner(cell_id);
                        cell_corners.push(edge.endpoints.min);
                        cell_corners.push(edge.endpoints.max);
                        edge_id
                    }
                    None => {
                        let edge_id = EdgeId(edges.len());
                        edges_lookup.insert(endpoints, edge_id);
                        let c1 = add_corner(edge_id, endpoints.min);
                        let c2 = add_corner(edge_id, endpoints.max);
                        let mut edge = Edge::new(c1, c2);
                        edge.add_owner(cell_id);
                        cell_corners.push(edge.endpoints.min);
                        cell_corners.push(edge.endpoints.max);
                        edges.push(edge);
                        edge_id
                    }
                }
            };

            let cell_id = CellId(cell_id);

            for line in polygon.exterior().lines() {
                let edge_id = add_edge(cell_id, &line);
                cell_edges.push(edge_id)
            }
            cells.push(Cell::new(polygon, cell_edges, cell_corners))
        }
        (corners, edges, cells)
    }

    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }

    pub fn polygon_at(&self, px: f32, py: f32) -> Option<CellId> {
        if px < 0.0 || py < 0.0 {
            return None;
        }

        let point = quadtree_rs::point::Point {
            x: px.round() as u64,
            y: py.round() as u64,
        };
        let area = quadtree_rs::area::AreaBuilder::default()
            .anchor(point)
            .dimensions((1, 1))
            .build()
            .unwrap();

        let mut query = self.cell_quadtree.query(area);

        let point = geo::Point::new(px as f64, py as f64);

        query
            .find(|entry| {
                let polygon = &self.cells[entry.value_ref().0].polygon;
                polygon.contains(&point)
            })
            .map(|e| *e.value_ref())
    }

    pub fn cells(&self) -> impl Iterator<Item = (CellId, &Cell)> {
        self.cells
            .iter()
            .enumerate()
            .map(|(id, cell)| (CellId(id), cell))
    }

    pub fn edges(&self) -> impl Iterator<Item = (EdgeId, &Edge)> {
        self.edges
            .iter()
            .enumerate()
            .map(|(id, edge)| (EdgeId(id), edge))
    }

    pub fn corners(&self) -> impl Iterator<Item = (CornerId, &Corner)> {
        self.corners
            .iter()
            .enumerate()
            .map(|(id, corner)| (CornerId(id), corner))
    }

    pub fn num_corners(&self) -> usize {
        self.corners.len()
    }

    pub fn edge_endpoints_coords(&self, edge: &Edge) -> ((f64, f64), (f64, f64)) {
        let c1 = self.corner(edge.endpoints.min);
        let c2 = self.corner(edge.endpoints.max);
        (c1.coords, c2.coords)
    }

    pub fn corner(&self, id: CornerId) -> &Corner {
        &self.corners[id.0]
    }

    fn polygon_area(polygon: &Polygon<f64>) -> quadtree_rs::area::Area<u64> {
        let mut min_x: f64 = 0.0;
        let mut min_y: f64 = 0.0;
        let mut max_x: f64 = 0.0;
        let mut max_y: f64 = 0.0;

        for p in polygon.exterior().coords_iter() {
            min_x = min_x.min(p.x);
            min_y = min_y.min(p.y);
            max_x = max_x.max(p.x);
            max_y = max_y.max(p.y);
        }

        let min_p = quadtree_rs::point::Point {
            x: min_x.ceil() as u64,
            y: min_y.ceil() as u64,
        };
        let max_p = (max_x.ceil() as u64, max_y.ceil() as u64);

        quadtree_rs::area::AreaBuilder::default()
            .anchor(min_p)
            .dimensions(max_p)
            .build()
            .unwrap()
    }
}
