use noise::Perlin;
use rand::Rng;
use polymap::{*, field::Field};

pub trait SpatialFunction: Send + Sync + Sized {
    fn value(&self, x: f64, y: f64) -> f64;

    fn scale(self, intensity: f64) -> Scaled<Self> { Scaled(self, intensity) }

    fn add_to_field(&self, poly: &PolyMap, field: &mut Field<f64>) {
        field.update(|id, field_value| {
            let (cx, cy) = poly[id].center();
            *field_value += self.value(cx, cy)
        })
    }
}

pub struct Scaled<T:SpatialFunction>(T, f64);

impl <T:SpatialFunction> SpatialFunction for Scaled<T> {
    fn value(&self, x: f64, y: f64) -> f64 {
        self.1 * self.0.value(x,  y)
    }
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

impl SpatialFunction for Slope {
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

impl SpatialFunction for Band {
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

impl SpatialFunction for PerlinField {
    fn value(&self, x: f64, y: f64) -> f64 {
        use noise::NoiseFn;

        let px = self.x_shift + x * self.frequency;
        let py = self.y_shift + y * self.frequency;
        let noise = self.noise.get([px, py]);
        noise
    }
}

// pub struct Clump {
//     x: f64,
//     y: f64,
//     amount: f64,
//     decay: f64,
// }

// impl Clump {
//     pub fn with_rng(w: f64, h: f64, amount: f64, decay: f64, rng: &mut impl Rng) -> Self {
//         Self {
//             x: rng.gen_range(0.0..=w),
//             y: rng.gen_range(0.0..=h),
//             amount,
//             decay,
//         }
//     }
// }

// impl SpatialFunction for Clump {
//     fn value(&self, x: f64, y: f64) -> f64 {
//         let distance = ((self.x - x).powi(2) + (self.y - y).powi(2)).sqrt();
//         let v = self.amount * self.decay.powf(distance);
//         v.max(0.0)
//     }
// }