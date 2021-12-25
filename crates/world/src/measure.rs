pub struct Measure {
    pub name: &'static str,
    pub symbol: &'static str,
    pub min: f64,
    pub max: f64,
}

impl Measure {
    pub fn normalize(&self, x: f64) -> f64 {
        (x - self.min)/(self.max - self.min)
    }
}

pub const RAIN: Measure = Measure {
    name: "Rainfall",
    symbol: "rain",
    min: 0.0,
    max: 50.0
};

pub const DRAIN: Measure = Measure {
    name: "Drainage",
    symbol: "drainage",
    min: 0.0,
    max: 1.0
};