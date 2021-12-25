use crate::*;
pub struct Field<T>(Vec<T>);

impl <T> std::ops::Index<CellId> for Field<T> {
    type Output = T;

    fn index(&self, index: CellId) -> &Self::Output {
        &self.0[index.0]
    }
}

impl <T> std::ops::IndexMut<CellId> for Field<T> {
    fn index_mut(&mut self, index: CellId) -> &mut Self::Output {
        &mut self.0[index.0]
    }
}

impl <T> Field<T> {
    pub fn with_fn(poly: &PolyMap, f: impl Fn(CellId, &Cell) -> T) -> Self {
        Self(poly.cells().map(|(id,cell)| f(id, cell)).collect())
    }

    pub fn update(&mut self, f: impl Fn(CellId, &mut T)) {
        for (idx, t) in self.0.iter_mut().enumerate() {
            f(CellId(idx), t)
        }
    }

    pub fn sorted_order(&self, compare: impl Fn (&T,&T) -> std::cmp::Ordering) -> Vec<CellId> {
        let mut values:Vec<_> = (0..self.0.len()).map(|x| CellId(x)).collect();
        values.sort_by(|&id1, &id2| {
            let t1 = &self.0[id1.0];
            let t2 = &self.0[id2.0];
            compare(t1, t2).then_with(|| { id1.cmp(&id2) })
        });
        values 
    }
}

impl <T:Copy> Field<T> {
    pub fn uniform(poly: &PolyMap, x: T) -> Self {
        Self(poly.cells().map(|(_,_)| x).collect())
    }
}

impl Field<f64> {

    pub fn smooth(&mut self, poly:&PolyMap, iterations: usize) {
        for _ in 0 .. iterations {
            self.smooth_once(poly)
        }
    }

    fn smooth_once(&mut self, poly:&PolyMap,) {
        let data = Field::with_fn(poly, |id, cell| {
            let mut count = 1;
            let mut val = self[id];
            for &neighbor in cell.neighbors() {
                val += self[neighbor];
                count += 1;
            }
            val/(count as f64)
        });
        self.0 = data.0;
    }
    
    pub fn normalize(&mut self) {
        let mut min = f64::MAX;
        let mut max = f64::MIN;

        for &x in self.0.iter() {
            min = min.min(x);
            max = max.max(x);
        }

        self.update(|_, x| *x = (*x - min)/(max - min));
    }
    
    pub fn ascending_order(&self) -> Vec<CellId> {
        self.sorted_order(|&x,&y| 
                if x < y { std::cmp::Ordering::Less } 
                else if x == y { std::cmp::Ordering::Equal } 
                else { std::cmp::Ordering::Greater }
        )
    }
}

pub trait Vectorial {
    fn points_to(&self) -> Option<CellId>;
}

impl <T:Vectorial + Clone> Field<T> {
    pub fn walk(&mut self, source: CellId, mut step: impl FnMut((CellId, &T), (CellId, &mut T)) ) {
        let mut cell = source;
        let mut current_vaue = self[cell].clone();
        loop {
            match self[cell].points_to() {
                None => return,
                Some(next) => {
                    step((cell, &current_vaue), (next, &mut self[next]));
                    cell = next;
                    current_vaue = self[next].clone();
                }
            }
        }
    }
}