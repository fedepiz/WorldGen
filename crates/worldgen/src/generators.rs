use polymap::{compute::CornerData, PolyMap};
use rand::Rng;

pub fn perlin_noise(
    grid: &mut CornerData<f64>,
    poly_map: &PolyMap,
    perlin_freq: f64,
    intensity: f64,
    rng: &mut impl Rng,
) {
    use noise::{NoiseFn, Perlin};

    let perlin = Perlin::new();

    let x_rand = rng.gen_range(0..100) as f64;
    let y_rand = rng.gen_range(0..100) as f64;

    grid.update_each(poly_map, |_, corner, h| {
        let px = x_rand + corner.x() * perlin_freq;
        let py = y_rand + corner.y() * perlin_freq;
        let noise = perlin.get([px, py]);
        *h += (noise + 1.0) / 2.0 * intensity;
    });
}
