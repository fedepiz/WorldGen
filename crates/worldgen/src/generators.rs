use polymap::{compute::{CornerData, CornerPicker}, PolyMap};
use rand::Rng;


pub trait GridGenerator {
    fn grid_mut(&mut self) -> &mut CornerData<f64>;

    fn random_slope(&mut self, poly_map: &PolyMap, steepness: f64, rng: &mut impl Rng) {
        let m = rng.gen_range(-100..200) as f64 / 100.0;

        let w = poly_map.width() as f64;
        let h = poly_map.height() as f64;
        self.grid_mut()
            .update_each(&poly_map, |_, corner, corner_height| {
                let distance = (corner.x() - w / 2.0) * m - (corner.y() - h / 2.0);
                *corner_height += distance * steepness;
            })
    }

    fn perlin_noise(
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
    
        self.grid_mut().update_each(poly_map, |_, corner, h| {
            let px = x_rand + corner.x() * perlin_freq;
            let py = y_rand + corner.y() * perlin_freq;
            let noise = perlin.get([px, py]);
            *h += (noise + 1.0) / 2.0 * intensity;
        });
    }

    fn clump(
        &mut self,
        poly_map: &PolyMap,
        amount: f64,
        decay: f64,
        end: f64,
        rng: &mut impl Rng,
    ) {
        let starting = CornerPicker::random(poly_map, rng);
        self.grid_mut().spread(
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

    fn normalize(&mut self) {
        let grid = self.grid_mut();

        let min = grid.min();
        let max = grid.max();
        grid
            .data
            .iter_mut()
            .for_each(|x| *x = (*x - min) / (max - min));
    }

    fn relax(&mut self, poly_map: &PolyMap, t: f64) {
        self.grid_mut()
            .update_with_neighbors(poly_map, |x, neighborhood| {
                let average = neighborhood.iter().copied().sum::<f64>();
                let n = neighborhood.len() as f64;
                *x = t * (average / n) + (1.0 - t) * *x
            })
    }
}

