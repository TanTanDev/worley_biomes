use std::default::Default;
use std::marker::PhantomData;

use bracket_fast_noise::prelude::FastNoise;
use serde::{Deserialize, Serialize};
use tinyvec::TinyVec;

use crate::biome_picker::{BiomePicker, BiomeVariants, SimpleBiomePicker};
use crate::distance_fn::DistanceFn;
use crate::utils::hash_u64;
use crate::warp::{WarpSettings, warp_coords};

///! a biome picker based on (worley) which is offset by (noise)
#[derive(Serialize, Deserialize)]
pub struct Worley<BiomeT, Picker>
where
    BiomeT: BiomeVariants + Serialize,
    Picker: BiomePicker<BiomeT> + Serialize + Default,
{
    ///! biome picking
    pub biome_picker: Picker,
    pub zoom: f64,
    ///!
    #[serde(skip, default = "default_distance_fn")]
    pub distance_fn: fn(f64, f64) -> f64,
    pub distance_fn_config: DistanceFn,
    ///! high value: sharper borders, recommended: 0.0 -> 20.0
    pub sharpness: f64,
    ///! how many k biomes to fetch closest
    pub k: usize,
    pub warp_settings: WarpSettings,
    #[serde(skip)]
    pub _phantom: PhantomData<BiomeT>,
}

fn default_distance_fn() -> fn(f64, f64) -> f64 {
    |dx, dz| (dx * dx + dz * dz).sqrt()
}

impl<BiomeT, Picker> Default for Worley<BiomeT, Picker>
where
    BiomeT: BiomeVariants + Serialize,
    Picker: BiomePicker<BiomeT> + Serialize + Default,
{
    fn default() -> Self {
        let distance_fn_config = DistanceFn::Euclidean;
        let distance_fn = distance_fn_config.to_func();
        // let distance_fn = |dx, dz| (dx * dx + dz * dz).sqrt();
        Self {
            distance_fn,
            distance_fn_config,
            biome_picker: Picker::default(),
            zoom: 100.0,
            sharpness: 20.0,
            k: 1,
            warp_settings: WarpSettings::default(),
            _phantom: PhantomData::default(),
            // ... other defaults ...
        }
    }
}
fn default_fast_noise() -> FastNoise {
    FastNoise::new()
}

const NEIGHBOR_OFFSETS: [(i32, i32); 9] = [
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, -1),
    (0, 0),
    (0, 1),
    (1, -1),
    (1, 0),
    (1, 1),
];

impl<BiomeT, Picker> Worley<BiomeT, Picker>
where
    BiomeT: BiomeVariants + 'static + Serialize + Default,
    Picker: BiomePicker<BiomeT> + Serialize + Default,
{
    pub fn set_distance_fn(&mut self, distance_fn: DistanceFn) {
        self.distance_fn = distance_fn.to_func();
        self.distance_fn_config = distance_fn;
    }
    pub fn get_distance_fn(&mut self) -> DistanceFn {
        self.distance_fn_config
    }

    ///! returns a vec of (0: percentage) we use for (1: biome type)
    pub fn get(&self, seed: u64, x: f64, z: f64) -> TinyVec<[(f64, BiomeT); 3]> {
        let (x, z) = (x / self.zoom, z / self.zoom);
        let (x, z) = warp_coords(
            &self.warp_settings.noise,
            self.warp_settings.strength,
            x as f32,
            z as f32,
        );
        if !x.is_finite() || !z.is_finite() {
            panic!("finite after warp");
        }

        let cell_x = x.floor() as i32;
        let cell_z = z.floor() as i32;

        // let mut candidates = Vec::new();
        let mut candidates: [(f64, BiomeT); 9] = [(0.0, BiomeT::default()); 9];
        for (i, (dx, dz)) in NEIGHBOR_OFFSETS.iter().enumerate() {
            let cx = cell_x + dx;
            let cz = cell_z + dz;
            let (fx, fz) = cell_point(seed, cx, cz);
            // let dist = distance(x - fx, z - fz, self.distance_fn);
            let dist = (self.distance_fn)(x - fx, z - fz);
            let biome = self.biome_picker.pick_biome(seed, cx, cz);
            // candidates.push((dist, biome));
            candidates[i] = (dist, biome);
        }

        let k = self.k.min(candidates.len());
        // select the 3 lowest
        candidates.select_nth_unstable_by(k, |a, b| a.0.total_cmp(&b.0));

        let mut sum = 0.0;
        let mut out = TinyVec::with_capacity(self.k);
        for (d, biome) in candidates.iter().take(self.k) {
            // very close, high value
            let w = if *d < 1e-9 {
                100.0
            } else {
                // closer to 0, higher weight value
                1.0 / d.powf(self.sharpness)
            };
            sum += w;
            out.push((w, *biome));
        }

        debug_assert!(sum.is_finite() && sum > 0.0, "invalid weight sum: {}", sum);
        for (w, _) in out.iter_mut() {
            *w /= sum;
        }

        out
    }
}

// generate a random position seeded from cell position
#[inline(always)]
fn cell_point(seed: u64, cell_x: i32, cell_z: i32) -> (f64, f64) {
    let h1 = hash_u64(seed.wrapping_add(1337), cell_x, cell_z);
    let h2 = hash_u64(seed.wrapping_add(7331), cell_x, cell_z);

    let fx = cell_x as f64 + ((h1 & 0xFFFF) as f64 / 65535.0);
    let fz = cell_z as f64 + ((h2 & 0xFFFF) as f64 / 65535.0);
    (fx, fz)
}
