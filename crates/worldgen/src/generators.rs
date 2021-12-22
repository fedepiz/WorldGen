use noise::Perlin;
use polymap::{compute::VertexData, PolyMap};
use rand::Rng;

pub trait Field: Send + Sync {
    fn value(&self, x: f64, y: f64) -> f64;
}

pub struct Slope {
    m: f64,
    cx: f64,
    cy: f64,
}

impl Slope {
    pub fn with_rng(w: f64, h: f64, rng: &mut impl Rng) -> Self {
        Self {
            cx: w / 2.0,
            cy: h / 2.0,
            m: rng.gen_range(-100..200) as f64 / 100.0,
        }
    }
}

impl Field for Slope {
    fn value(&self, x: f64, y: f64) -> f64 {
        (x - self.cx) * self.m - (y - self.cy)
    }
}

pub struct Band {
    m: f64,
    cx: f64,
    cy: f64,
    radius: f64,
}

impl Band {
    pub fn new(cx: f64, cy: f64, m: f64, radius: f64) -> Self {
        Self { cx, cy, m, radius }
    }
}

impl Field for Band {
    fn value(&self, x: f64, y: f64) -> f64 {
        let distance = ((x - self.cx) * self.m - (y - self.cy)).abs();
        (1.0 - distance / self.radius).max(0.0)
    }
}

pub struct PerlinField {
    pub frequency: f64,
    pub x_shift: f64,
    pub y_shift: f64,
    noise: Perlin,
}

impl PerlinField {
    pub fn with_rng(frequency: f64, rng: &mut impl Rng) -> Self {
        let x_shift = rng.gen_range(0..100) as f64;
        let y_shift = rng.gen_range(0..100) as f64;

        Self::new(x_shift, y_shift, frequency)
    }

    pub fn new(x_shift: f64, y_shift: f64, frequency: f64) -> Self {
        let noise = Perlin::new();

        Self {
            noise,
            frequency,
            x_shift,
            y_shift,
        }
    }
}

impl Field for PerlinField {
    fn value(&self, x: f64, y: f64) -> f64 {
        use noise::NoiseFn;

        let px = self.x_shift + x * self.frequency;
        let py = self.y_shift + y * self.frequency;
        let noise = self.noise.get([px, py]);
        noise
    }
}

pub struct Clump {
    x: f64,
    y: f64,
    amount: f64,
    decay: f64,
}

impl Clump {
    pub fn with_rng(w: f64, h: f64, amount: f64, decay: f64, rng: &mut impl Rng) -> Self {
        Self {
            x: rng.gen_range(0.0..=w),
            y: rng.gen_range(0.0..=h),
            amount,
            decay,
        }
    }
}

impl Field for Clump {
    fn value(&self, x: f64, y: f64) -> f64 {
        let distance = ((self.x - x).powi(2) + (self.y - y).powi(2)).sqrt();
        let v = self.amount * self.decay.powf(distance);
        v.max(0.0)
    }
}

pub trait GridGenerator {
    fn grid(&self) -> &VertexData<f64>;
    fn grid_mut(&mut self) -> &mut VertexData<f64>;

    fn add_field(&mut self, poly_map: &PolyMap, field: &impl Field, intensity: f64) {
        self.grid_mut().update_each(poly_map, |_, corner, h| {
            let v = field.value(corner.x(), corner.y());
            *h += v * intensity;
        });
    }

    fn add_field_scaled(
        &mut self,
        poly_map: &PolyMap,
        field: impl Field,
        coefficients: &impl GridGenerator,
        intensity: f64,
    ) {
        let coeffs = coefficients.grid();
        self.grid_mut().update_each(poly_map, |id, corner, h| {
            let v = field.value(corner.x(), corner.y());
            let coeff = coeffs[id];
            *h += v * coeff * intensity;
        });
    }

    fn normalize(&mut self) {
        let grid = self.grid_mut();

        let min = grid.min();
        let max = grid.max();
        grid.data
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
