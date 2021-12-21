pub mod compute;
pub mod map_shader;
pub mod painter;

use arrayvec::ArrayVec;
use geo::{contains::Contains, Polygon};
use std::collections::HashMap;

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
pub struct VertexId(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeId(usize);

pub struct Cell {
    polygon: Polygon<f64>,
    edges: Vec<EdgeId>,
    corners: Vec<VertexId>,
    neighbors: Vec<CellId>,
}

impl Cell {
    fn new(polygon: Polygon<f64>, edges: Vec<EdgeId>, corners: Vec<VertexId>) -> Self {
        Self {
            polygon,
            edges,
            corners,
            neighbors: vec![],
        }
    }

    pub fn corners(&self) -> &[VertexId] {
        self.corners.as_slice()
    }

    pub fn neighbors(&self) -> &[CellId] {
        self.neighbors.as_slice()
    }

    fn fix(&mut self) {
        self.corners.sort();
        self.neighbors.sort();
        self.edges.sort();
    }
}

pub struct Vertex {
    coords: (f64, f64),
    edges: ArrayVec<[EdgeId; 5]>,
    neighbors: ArrayVec<[VertexId; 5]>,
    is_border: bool,
}

impl Vertex {
    fn new(edge: EdgeId, location: Location, is_border: bool) -> Self {
        Self {
            coords: (location.0, location.1),
            edges: { let mut v = ArrayVec::new(); v.push(edge); v},
            neighbors: ArrayVec::new(),
            is_border,
        }
    }

    fn add_edge(&mut self, id: EdgeId) {
        if !self.edges.contains(&id) {
            self.edges.push (id);
            self.edges.sort();
        }
    }

    pub fn x(&self) -> f64 {
        self.coords.0
    }
    pub fn y(&self) -> f64 {
        self.coords.1
    }

    pub fn neighbors(&self) -> &[VertexId] {
        self.neighbors.as_slice()
    }

    pub fn edges(&self) -> &[EdgeId] {
        self.edges.as_slice()
    }

    pub fn cells<'a>(&'a self, poly_map: &'a PolyMap) -> impl Iterator<Item = CellId> + 'a {
        self.edges()
            .iter()
            .flat_map(|&edge| poly_map.edge(edge).cells().iter().copied())
    }

    pub fn other_edge(&self, id: EdgeId) -> Option<EdgeId> {
        self.edges().iter().find(|&&other| other != id).copied()
    }

    pub fn is_border(&self) -> bool {
        self.is_border
    }

    fn fix(&mut self) {
        self.neighbors.sort();
        self.edges.sort();
    }
}

pub struct Edge {
    endpoints: OrderedPair<VertexId>,
    cells: ArrayVec<[CellId;2]>,
}

impl Edge {
    fn new(c1: VertexId, c2: VertexId) -> Self {
        Self {
            endpoints: OrderedPair::new(c1, c2),
            cells: ArrayVec::new(),
        }
    }

    pub fn start(&self) -> VertexId {
        self.endpoints.min
    }
    pub fn end(&self) -> VertexId {
        self.endpoints.max
    }

    pub fn cells(&self) -> &[CellId] {
        self.cells.as_slice()
    }

    fn fix(&mut self) {
        self.cells.sort();
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
    vertices: Vec<Vertex>,
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
            &(0.0, 0.0),
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

        let (mut vertices, mut edges, mut cells) =
            Self::build_elements(polygons, width as f64, height as f64);

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
        for (idx, vertex) in vertices.iter_mut().enumerate() {
            for edge in vertex.edges.iter() {
                let edge = &edges[edge.0];
                if edge.endpoints.min.0 == idx {
                    if !vertex.neighbors.contains(&edge.endpoints.max) {
                        vertex.neighbors.push(edge.endpoints.max)
                    }
                } else {
                    if !vertex.neighbors.contains(&edge.endpoints.min) {
                        vertex.neighbors.push(edge.endpoints.min);
                    }
                }
            }
        }        


        vertices.iter_mut().for_each(|x| x.fix());
        edges.iter_mut().for_each(|x| x.fix());
        cells.iter_mut().for_each(|x| x.fix());

        PolyMap {
            width,
            height,
            cells,
            edges,
            vertices,
        }
    }

    fn build_elements(
        polygons: Vec<Polygon<f64>>,
        width: f64,
        height: f64,
    ) -> (Vec<Vertex>, Vec<Edge>, Vec<Cell>) {
        let mut edges: Vec<Edge> = vec![];
        let mut vertices: Vec<Vertex> = vec![];
        let mut cells: Vec<Cell> = vec![];

        let mut edges_lookup: HashMap<_, EdgeId> = HashMap::new();
        let mut vertices_by_position: HashMap<_, VertexId> = HashMap::new();

        for (cell_id, polygon) in polygons.into_iter().enumerate() {
            let mut cell_edges = vec![];

            let mut add_vertex =
                |edge_id: EdgeId, location: Location| match vertices_by_position.get(&location) {
                    Some(&id) => {
                        let vertex = &mut vertices[id.0];
                        vertex.add_edge(edge_id);
                        id
                    }
                    None => {
                        let id = VertexId(vertices.len());
                        vertices_by_position.insert(location, id);
                        let is_border = location.0 <= 0.
                            || location.0 >= width
                            || location.1 <= 0.
                            || location.1 >= height;
                        vertices.push(Vertex::new(edge_id, location, is_border));
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
                        edge.cells.push(cell_id);
                        cell_corners.push(edge.endpoints.min);
                        cell_corners.push(edge.endpoints.max);
                        edge_id
                    }
                    None => {
                        let edge_id = EdgeId(edges.len());
                        edges_lookup.insert(endpoints, edge_id);
                        let c1 = add_vertex(edge_id, endpoints.min);
                        let c2 = add_vertex(edge_id, endpoints.max);
                        let mut edge = Edge::new(c1, c2);
                        edge.cells.push(cell_id);
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

        (vertices, edges, cells)
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


        let point = geo::Point::new(px as f64, py as f64);

        self.cells()
            .find(|&(_, cell)| cell.polygon.contains(&point))
            .map(|(id, _)| id)
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

    pub fn vertices(&self) -> impl Iterator<Item = (VertexId, &Vertex)> {
        self.vertices
            .iter()
            .enumerate()
            .map(|(id, corner)| (VertexId(id), corner))
    }

    pub fn num_vertices(&self) -> usize {
        self.vertices.len()
    }

    pub fn edge_endpoints_coords(&self, edge: &Edge) -> ((f64, f64), (f64, f64)) {
        let c1 = self.vertex(edge.endpoints.min);
        let c2 = self.vertex(edge.endpoints.max);
        (c1.coords, c2.coords)
    }

    pub fn edge_between(&self, c1: VertexId, c2: VertexId) -> Option<EdgeId> {
        let op = OrderedPair::new(c1, c2);
        self.vertices[op.min.0]
            .edges()
            .iter()
            .copied()
            .find(|&edge_id| self.edges[edge_id.0].endpoints.max == op.max)
    }

    pub fn vertex(&self, id: VertexId) -> &Vertex {
        &self.vertices[id.0]
    }

    pub fn edge(&self, id: EdgeId) -> &Edge {
        &self.edges[id.0]
    }
}
