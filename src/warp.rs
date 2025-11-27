use bracket_fast_noise::prelude::FastNoise;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Default)]
pub struct WarpSettings {
    pub strength: f32,
    pub noise: FastNoise,
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

pub fn warp_coords(noise: &FastNoise, strength: f32, x: f32, z: f32) -> (f64, f64) {
    let nx = noise.get_noise(x, z);
    let nz = noise.get_noise(x + 103f32, z);
    ((x + nx * strength) as f64, (z + nz * strength) as f64)
}
