use bracket_fast_noise::prelude::FastNoise;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize)]
pub struct WarpSettings {
    pub strength: f32,
    pub noise: FastNoise,
    // pub noise_seed: u64,
    // pub noise_frequency: f32,
    // pub noise_fractal_lacunarity: f32,
    // pub noise_fractal_gain: f32,
    // pub noise_fractal_octaves: i32,
    // pub noise_noise_type: NoiseType,
    // pub noise_fractal_type: FractalType,
}

impl WarpSettings {
    pub fn warp_coords(&self, x: f32, z: f32) -> (f64, f64) {
        let nx = self.noise.get_noise(x, z);
        let nz = self.noise.get_noise(x + 103f32, z);
        (
            (x + nx * self.strength) as f64,
            (z + nz * self.strength) as f64,
        )
    }
}

// impl WarpSettings {
//     pub fn make_fast_noise(&self) -> FastNoise {
//         let mut noise = FastNoise::new();
//         noise.set_seed(self.noise_seed);
//         noise.set_frequency(self.noise_frequency);
//         noise.set_fractal_lacunarity(self.noise_fractal_lacunarity);
//         noise.set_fractal_gain(self.noise_fractal_gain);
//         noise.set_fractal_octaves(self.noise_fractal_octaves);
//         noise.set_noise_type(self.noise_noise_type.to_fast_noise());
//         noise.set_fractal_type(self.noise_fractal_type.to_fast_noise());
//         noise
//     }
// }

pub fn warp_coords(noise: &FastNoise, strength: f32, x: f32, z: f32) -> (f64, f64) {
    let nx = noise.get_noise(x, z);
    let nz = noise.get_noise(x + 103f32, z);
    ((x + nx * strength) as f64, (z + nz * strength) as f64)
}
