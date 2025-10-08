use std::marker::PhantomData;

use bracket_fast_noise::prelude::FastNoise;
use serde::{Deserialize, Serialize};

use crate::biome_picker::{BiomePicker, BiomeVariants, SimpleBiomePicker};
use crate::distance_fn::{DistanceFn, distance};
use crate::utils::hash_u64;
use crate::warp::{WarpSettings, warp_coords};

///! a biome picker based on (worley) which is offset by (noise)
#[derive(Serialize, Deserialize)]
pub struct Worley<BiomeT, Picker>
where
    BiomeT: BiomeVariants + Serialize,
    Picker: BiomePicker<BiomeT> + Serialize,
{
    ///! biome picking
    pub biome_picker: Picker,
    pub zoom: f64,
    ///!
    pub distance_fn: DistanceFn,
    ///! high value: sharper borders
    pub sharpness: f64,
    ///! how many k biomes to fetch closest
    pub k: usize,
    pub warp_settings: WarpSettings,
    #[serde(skip)]
    #[serde(default = "default_fast_noise")]
    pub cached_warp_noise: FastNoise,
    #[serde(skip)]
    pub _phantom: PhantomData<BiomeT>,
}

fn default_fast_noise() -> FastNoise {
    FastNoise::new()
}

impl<BiomeT, Picker> Worley<BiomeT, Picker>
where
    BiomeT: BiomeVariants + 'static + Serialize,
    Picker: BiomePicker<BiomeT> + Serialize,
{
    ///! returns a vec of (0: percentage) we use for (1: biome type)
    pub fn get(&self, seed: u64, x: f64, z: f64) -> Vec<(f64, BiomeT)> {
        let (x, z) = (x / self.zoom, z / self.zoom);
        let (x, z) = warp_coords(
            &self.cached_warp_noise,
            self.warp_settings.strength,
            x as f32,
            z as f32,
        );
        if !x.is_finite() || !z.is_finite() {
            panic!("finite after warp");
        }

        let cell_x = x.floor() as i32;
        let cell_z = z.floor() as i32;

        let mut candidates = Vec::new();
        for dx in -1..=1 {
            for dz in -1..=1 {
                let cx = cell_x + dx;
                let cz = cell_z + dz;
                let (fx, fz) = cell_point(seed, cx, cz);
                let dist = distance(x - fx, z - fz, self.distance_fn);
                let biome = self.biome_picker.pick_biome(seed, cx, cz);
                candidates.push((dist, biome));
            }
        }

        candidates.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());
        let nearest = candidates.into_iter().take(self.k).collect::<Vec<_>>();

        let mut sum = 0.0;
        let mut out = Vec::new();
        for (d, biome) in nearest {
            let w = if d < 1e-9 {
                1.0
            } else {
                let val = d.powf(self.sharpness);
                if val.is_finite() && val > 0.0 {
                    1.0 / val
                } else {
                    0.0
                }
            };
            sum += w;
            out.push((w, biome));
        }

        // uniform distribute weights if sum is == 0 or NaN
        if !sum.is_finite() || sum <= 0.0 {
            let uniform = 1.0 / out.len() as f64;
            for (w, _) in out.iter_mut() {
                *w /= uniform;
            }
        } else {
            for (w, _) in out.iter_mut() {
                *w /= sum;
            }
        }

        out
    }
}

// generate a random position seeded from cell position
fn cell_point(seed: u64, cell_x: i32, cell_z: i32) -> (f64, f64) {
    let h1 = hash_u64(seed.wrapping_add(1337), cell_x, cell_z);
    let h2 = hash_u64(seed.wrapping_add(7331), cell_x, cell_z);

    let fx = cell_x as f64 + ((h1 & 0xFFFF) as f64 / 65535.0);
    let fz = cell_z as f64 + ((h2 & 0xFFFF) as f64 / 65535.0);
    (fx, fz)
}
