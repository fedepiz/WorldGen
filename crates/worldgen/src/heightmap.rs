use polymap::*;
use polymap::compute::*;

use rand::Rng;

pub(super) struct HeightMapBuilder {
    corners: CornerData<f64>,
}

impl HeightMapBuilder {
    pub(super) fn new(poly_map: &PolyMap, default: f64) -> Self {
        let corners = CornerData::for_each(&poly_map, |_, _| default);
        Self { corners }
    }

    pub(super) fn perlin_noise(
        &mut self,
        poly_map: &PolyMap,
        perlin_freq: f64,
        intensity: f64,
        rng: &mut impl Rng,
    ) {
        use noise::{NoiseFn, Perlin};

        let perlin = Perlin::new();

        let x_rand = rng.gen_range(0..100) as f64;
        let y_rand = rng.gen_range(0..100) as f64;

        self.corners.update_each(poly_map, |_, corner, h| {
            let px = x_rand + corner.x() * perlin_freq;
            let py = y_rand + corner.y() * perlin_freq;
            let noise = perlin.get([px, py]);
            *h += (noise + 1.0) / 2.0 * intensity;
        })
    }

    pub(super) fn random_slope(&mut self, poly_map: &PolyMap, steepness: f64, rng: &mut impl Rng) {
        let m = rng.gen_range(-100..200) as f64 / 100.0;

        let w = poly_map.width() as f64;
        let h = poly_map.height() as f64;
        self.corners
            .update_each(&poly_map, |_, corner, corner_height| {
                let distance = (corner.x() - w / 2.0) * m - (corner.y() - h / 2.0);
                *corner_height += distance * steepness;
            })
    }

    pub(super) fn clump(&mut self, poly_map: &PolyMap, amount: f64, decay: f64, end: f64, rng: &mut impl Rng) {
        let starting = CornerPicker::random(poly_map, rng);
        self.corners.spread(
            poly_map,
            starting,
            amount,
            |accum| {
                if accum.abs() > end.abs() {
                    Some(accum * decay)
                } else {
                    None
                }
            },
            |_, corner_height, x| *corner_height += *x,
        )
    }

    pub(super) fn normalize(&mut self) {
        let min = self.corners.data.iter().copied().reduce(f64::min).unwrap();
        let max = self.corners.data.iter().copied().reduce(f64::max).unwrap();
        self.corners
            .data
            .iter_mut()
            .for_each(|x| *x = (*x - min) / (max - min));
    }

    pub(super) fn relax(&mut self, poly_map: &PolyMap, t: f64) {
        self.corners
            .update_with_neighbors(poly_map, |x, neighborhood| {
                let average = neighborhood.iter().copied().sum::<f64>();
                let n = neighborhood.len() as f64;
                *x = t * (average / n) + (1.0 - t) * *x
            })
    }

    fn planchon_darboux(poly_map: &PolyMap, h: &mut CornerData<f64>, epsilon: f64) {
        let mut new_h = CornerData::for_each(poly_map, |id, corner| {
            if corner.is_border() {
                h[id]
            } else {
                100.0
            }
        });

        let mut changed = true;
        while changed {
            changed = false;
            for (id, corner) in poly_map.corners() {
                if new_h[id] == h[id] {
                    continue;
                }
                for &neighbor in corner.neighbors() {
                    if h[id] >= new_h[neighbor] + epsilon {
                        new_h[id] = h[id];
                        changed = true;
                        break;
                    }
                    let oh = new_h[neighbor] + epsilon;
                    if (new_h[id] > oh) && (oh > h[id]) {
                        new_h[id] = oh;
                        changed = true;
                    }
                }
            }
        }

        std::mem::swap(&mut new_h, h);
    }

    pub(super) fn build(mut self, poly_map: &PolyMap) -> HeightMap {     
        
        Self::planchon_darboux(poly_map, &mut self.corners, 0.001);

        let descent_vector = CornerData::for_each(poly_map, |id, corner| {
            let my_elevation = self.corners[id];
            let mut slope: Option<Slope> = None;

            for &neighbor in corner.neighbors() {
                let neighbor_elevation = self.corners[neighbor];
                let diff = my_elevation - neighbor_elevation;
                if diff > 0.0 {
                    let update = match slope {
                        None => true,
                        Some(slope) => slope.intensity < diff,
                    };
                    if update {
                        slope = Some(Slope {
                            towards: neighbor,
                            intensity: diff,
                        });
                    }
                }
            }
            slope
        });

        let cells: CellData<f64> = CellData::corner_average(poly_map, &self.corners);


        fn descending(x: &f64, y: &f64) -> std::cmp::Ordering {
            (if x < y {
                std::cmp::Ordering::Less
            } else if x == y {
                std::cmp::Ordering::Equal
            } else {
                std::cmp::Ordering::Greater
            }).reverse()
        }

        let downhill = self.corners.ordered_by(descending);

        HeightMap {
            corners: self.corners,
            cells,
            descent_vector,
            downhill,
        }
    }
}
#[derive(Clone)]
pub struct HeightMap {
    pub corners: CornerData<f64>,
    pub cells: CellData<f64>,
    pub descent_vector: CornerData<Option<Slope>>,
    pub downhill: Vec<CornerId>,
}

impl HeightMap {
    pub fn is_descent(&self, top: CornerId, bottom:CornerId) -> bool {
        self.descent_vector[top].as_ref().map(|x| x.towards == bottom).unwrap_or(false)
    }
}

#[derive(Clone, Copy)]
pub struct Slope {
    pub towards: CornerId,
    pub intensity: f64,
}