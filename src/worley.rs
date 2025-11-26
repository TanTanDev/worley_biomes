use std::default::Default;
use std::marker::PhantomData;

use serde::{Deserialize, Serialize};
use tinyvec::TinyVec;

use crate::biome_picker::{BiomePicker, BiomeVariants};
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
    #[serde(skip, default = "default_distance_fn")]
    pub distance_fn: fn(f64, f64) -> f64,
    pub distance_fn_config: DistanceFn,
    ///! high value: sharper borders, recommended: 0.0 -> 20.0
    pub sharpness: f64,
    ///! how many k biomes to fetch closest
    pub k: usize,
    pub seed: u64,
    ///! warps coordinate for interesting shapes
    pub warp_settings: WarpSettings,
    ///! if set, biomes below this threshold, will not return from Worley::get()
    ///! recommended to be set, defaults to 0.01 = 1%
    pub kill_percent_threshold: Option<f64>,
    #[serde(skip)]
    pub _phantom: PhantomData<BiomeT>,
}

// euclidian squared
fn default_distance_fn() -> fn(f64, f64) -> f64 {
    |dx, dz| dx * dx + dz * dz
}

impl<BiomeT, Picker> Default for Worley<BiomeT, Picker>
where
    BiomeT: BiomeVariants + Serialize,
    Picker: BiomePicker<BiomeT> + Serialize + Default,
{
    fn default() -> Self {
        let distance_fn_config = DistanceFn::EuclideanSquared;
        let distance_fn = distance_fn_config.to_func();
        Self {
            distance_fn,
            distance_fn_config,
            biome_picker: Picker::default(),
            zoom: 100.0,
            sharpness: 20.0,
            k: 3,
            warp_settings: WarpSettings::default(),
            _phantom: PhantomData::default(),
            kill_percent_threshold: Some(0.01),
            seed: 0,
        }
    }
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
    pub fn get(&self, x: f64, z: f64) -> TinyVec<[(f64, BiomeT); 3]> {
        let (x, z) = (x / self.zoom, z / self.zoom);
        let (x, z) = warp_coords(
            &self.warp_settings.noise,
            self.warp_settings.strength,
            x as f32,
            z as f32,
        );

        let cell_x = x.floor() as i32;
        let cell_z = z.floor() as i32;

        let mut candidates: [(f64, BiomeT); 9] = [(0.0, BiomeT::default()); 9];
        for (i, (dx, dz)) in NEIGHBOR_OFFSETS.iter().enumerate() {
            let cx = cell_x + dx;
            let cz = cell_z + dz;
            let (fx, fz) = cell_point(self.seed, cx, cz);
            let dist = (self.distance_fn)(x - fx, z - fz);
            let biome = self.biome_picker.pick_biome(self.seed, cx, cz);
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

        for (w, _) in out.iter_mut() {
            *w /= sum;
        }

        // remove low percentage biomes
        if let Some(kill_percent_threshold) = self.kill_percent_threshold {
            let len_before = out.len();
            out.retain(|(percent, _biome)| *percent > kill_percent_threshold);
            if out.len() != len_before {
                // calculate new sum, and recalculate the percentages
                let new_sum_percent: f64 = out.iter().map(|(percent, _biome)| percent).sum();
                for (percent, _biome) in out.iter_mut() {
                    *percent /= new_sum_percent;
                }
            }
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
