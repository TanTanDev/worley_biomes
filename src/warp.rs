use bracket_noise::prelude::FastNoise;
use serde::{Deserialize, Serialize};

///! local definition we can serialize, to map to fastnoise
#[derive(Debug, PartialEq, Copy, Clone, Deserialize, Serialize)]
/// Type of noise to generate
pub enum NoiseType {
    Value,
    ValueFractal,
    Perlin,
    PerlinFractal,
    Simplex,
    SimplexFractal,
    Cellular,
    WhiteNoise,
    Cubic,
    CubicFractal,
}

impl NoiseType {
    pub fn to_fast_noise(self) -> bracket_noise::prelude::NoiseType {
        use bracket_noise::prelude as bn;
        match self {
            NoiseType::Value => bn::NoiseType::Value,
            NoiseType::ValueFractal => bn::NoiseType::ValueFractal,
            NoiseType::Perlin => bn::NoiseType::Perlin,
            NoiseType::PerlinFractal => bn::NoiseType::PerlinFractal,
            NoiseType::Simplex => bn::NoiseType::Simplex,
            NoiseType::SimplexFractal => bn::NoiseType::SimplexFractal,
            NoiseType::Cellular => bn::NoiseType::Cellular,
            NoiseType::WhiteNoise => bn::NoiseType::WhiteNoise,
            NoiseType::Cubic => bn::NoiseType::Cubic,
            NoiseType::CubicFractal => bn::NoiseType::CubicFractal,
        }
    }
}

#[derive(Debug, PartialEq, Copy, Clone, Deserialize, Serialize)]
/// Interpolation function to use
pub enum Interp {
    Linear,
    Hermite,
    Quintic,
}

#[derive(Debug, PartialEq, Copy, Clone, Deserialize, Serialize)]
/// Fractal function to use
pub enum FractalType {
    FBM,
    Billow,
    RigidMulti,
}

impl FractalType {
    fn to_fast_noise(&self) -> bracket_noise::prelude::FractalType {
        use bracket_noise::prelude::FractalType as ft;
        match self {
            FractalType::FBM => ft::FBM,
            FractalType::Billow => ft::Billow,
            FractalType::RigidMulti => ft::RigidMulti,
        }
    }
}

#[derive(Serialize, Deserialize, Debug)]
pub struct WarpSettings {
    pub strength: f32,
    pub noise_seed: u64,
    pub noise_frequency: f32,
    pub noise_fractal_lacunarity: f32,
    pub noise_fractal_gain: f32,
    pub noise_fractal_octaves: i32,
    pub noise_noise_type: NoiseType,
    pub noise_fractal_type: FractalType,
}

impl WarpSettings {
    pub fn make_fast_noise(&self) -> FastNoise {
        let mut noise = FastNoise::new();
        noise.set_seed(self.noise_seed);
        noise.set_frequency(self.noise_frequency);
        noise.set_fractal_lacunarity(self.noise_fractal_lacunarity);
        noise.set_fractal_gain(self.noise_fractal_gain);
        noise.set_fractal_octaves(self.noise_fractal_octaves);
        noise.set_noise_type(self.noise_noise_type.to_fast_noise());
        noise.set_fractal_type(self.noise_fractal_type.to_fast_noise());
        noise
    }
}

pub fn warp_coords(noise: &FastNoise, strength: f32, x: f32, z: f32) -> (f64, f64) {
    let nx = noise.get_noise(x, z);
    let nz = noise.get_noise(x + 103f32, z);
    ((x + nx * strength) as f64, (z + nz * strength) as f64)
}
