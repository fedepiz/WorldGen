use geo::{contains::Contains, Polygon};

pub mod field;

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

impl CellId {
    pub fn idx(&self) -> usize { self.0 }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct VertexId(usize);

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct EdgeId(usize);

pub struct Cell {
    center: (f64, f64),
    polygon: Polygon<f64>,
    neighbors: Vec<CellId>,
    is_border:bool
}

impl Cell {

    pub fn center(&self) -> (f64, f64) { self.center }

    pub fn polygon(&self) -> &Polygon<f64> { &self.polygon }

    pub fn neighbors(&self) -> &[CellId] {
        self.neighbors.as_slice()
    }

    pub fn is_border(&self) -> bool { self.is_border }
}

pub struct PolyMap {
    width: usize,
    height: usize,
    cells: Vec<Cell>,
    borders: Vec<CellId>,
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


        let cells:Vec<_> = voronoi.cells().iter().enumerate()
            .map(|(idx, polygon)| {
                let exterior: Vec<_> = polygon.points().iter().map(|p| (p.x, p.y)).collect();
                let polygon = geo::Polygon::new(geo::LineString::from(exterior), vec![]);
                let center = &voronoi.sites[idx];
                
                let neighbors_idxs = voronoi.neighbors[idx].as_slice();
                let mut is_border = false;
                let mut neighbors:Vec<_> = neighbors_idxs.iter().filter_map(|&neighbor_idx| {
                    // Borders are represented as out-of-bounds neighbors
                    if neighbor_idx < voronoi.cells().len() {
                        Some(CellId(neighbor_idx))
                    } else {
                        is_border = true;
                        None
                    }
                }).collect();

                neighbors.sort_by_key(|x| x.0);

                Cell {
                    center: (center.x, center.y),
                    polygon,
                    neighbors,
                    is_border
                }
            }).collect();
            
        let mut borders:Vec<_> = cells.iter().enumerate()
            .filter_map(|(idx, cell)| if cell.is_border { Some(CellId(idx))} else { None })
            .collect();
        borders.sort();

        PolyMap {
            width,
            height,
            cells,
            borders
        }
    }

    pub fn width(&self) -> usize {
        self.width
    }
    pub fn height(&self) -> usize {
        self.height
    }

    pub fn cell_at(&self, px: f64, py: f64) -> Option<CellId> {
        if px < 0.0 || py < 0.0 {
            return None;
        }

        let point = geo::Point::new(px, py);

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

    pub fn borders(&self) -> impl Iterator<Item = (CellId,&Cell)> {
        self.borders.iter().map(|&id| (id, &self.cells[id.0]))
    }

    pub fn angle_between_cells(&self, from: CellId, to: CellId) -> f64 {
        let (fx, fy) = self.cells[from.0].center();
        let (tx, ty) = self.cells[to.0].center();

        f64::atan2(-(ty-fy), tx-fx)
    }
}

impl std::ops::Index<CellId> for PolyMap {
    type Output = Cell;

    fn index(&self, index: CellId) -> &Self::Output {
        &self.cells[index.0]
    }
}
