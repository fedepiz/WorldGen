use super::*;

#[derive(Clone)]
pub struct CornerData<T> {
    pub data: Vec<T>
}

impl <T> CornerData<T> {
    pub fn for_each(poly_map: &PolyMap, mut f: impl FnMut(CornerId, &Corner) -> T) -> Self {
        Self { data: poly_map.corners().map(|(id, c)| f(id,c)).collect() }
    }

    pub fn update_each(&mut self, poly_map: &PolyMap, mut f: impl FnMut(CornerId, &Corner, &mut T)) {
        for ((corner_id, corner), data) in poly_map.corners().zip(self.data.iter_mut()) {
            f(corner_id, corner, data)
        }
    }
}

#[derive(Clone)]
pub struct CellData<T> {
    pub data:Vec<T>
}

impl <T> CellData<T> {
    pub fn from_corners_data<U>(poly_map: &PolyMap, corners_data: &CornerData<U>, mut f: impl FnMut(CellId, &Cell, &[(CornerId, &U)]) -> T) -> Self {
        let mut buf = Vec::with_capacity(10);

        Self { 
            data: poly_map.cells().map(|(id, cell)|{
                buf.clear();
                // Extract corner data
                cell.corners().iter().for_each(|&corner_id| {
                    let corner_data = &corners_data.data[corner_id.0];
                    buf.push((corner_id, corner_data));
                });
                f(id, cell, buf.as_slice())       
             }).collect()
        }
    }

    pub fn transform<U>(&self, mut f: impl FnMut(CellId, &T)->U) -> CellData<U> {
        CellData { data: self.data.iter().enumerate().map(|(idx, t)| f(CellId(idx), t)).collect() }
    }

    pub fn find_with_all_neighbors<'a>(&'a self, poly_map:&'a PolyMap, mut f: impl FnMut(CellId, &T)->bool + 'a) -> impl Iterator<Item=CellId> + 'a {
        poly_map.cells().filter_map(move |(idx, cell)| {
            let mut value = true;
            for &neighbor_id in cell.neighbors() {
                if !f(neighbor_id, &self.data[neighbor_id.0]) {
                    value = false;
                    break;
                }
            }
            if value { Some(idx) } else { None }
        })
    }
}

impl CellData<f64> {
    pub fn corner_average(poly_map: &PolyMap, corners: &CornerData<f64>) -> Self {
        CellData::from_corners_data(&poly_map, &corners, |_,_,c_data| {
            let total: f64 = c_data.iter().map(|(_, v)| **v).sum();
            let n = c_data.len() as f64;
            total/n 
        })
    }
}

impl <T> std::ops::Index<CellId> for CellData<T> {
    type Output = T;
    fn index(&self, index: CellId) -> &Self::Output {
        &self.data[index.0]
    }
}
impl <T> std::ops::IndexMut<CellId> for CellData<T> {
    fn index_mut(&mut self, index: CellId) -> &mut Self::Output {
        &mut self.data[index.0]
    }
}
