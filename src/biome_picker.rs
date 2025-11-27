use rand::Rng;
#[cfg(feature = "serde")]
use serde::{Deserialize, Serialize};

use crate::utils::{hash_u64, seeded_rng};

pub trait BiomePicker<BiomeT> {
    fn pick_biome(&self, seed: u64, cell_x: i32, cell_z: i32) -> BiomeT;
}

///! trait needed to know what variants are available
pub trait BiomeVariants: Copy {
    fn variants() -> &'static [Self]; // list of all variants
}

///! used to generates a biome VARIANT, based upon a "cell" position
#[cfg_attr(feature = "serde", derive(Serialize, Deserialize))]
#[derive(Default)]
pub enum SimpleBiomePicker<BiomeT: BiomeVariants> {
    // all variants have same chance of being selected
    #[default]
    Any,
    // all variants have same chance of being selected
    AnyOf(Vec<BiomeT>),
    // weighted odds for biomes to be selected
    Weighted(Vec<(BiomeT, f32)>),
}

impl<BiomeT: BiomeVariants + 'static> BiomePicker<BiomeT> for SimpleBiomePicker<BiomeT> {
    #[inline(always)]
    fn pick_biome(&self, seed: u64, cell_x: i32, cell_z: i32) -> BiomeT {
        match self {
            SimpleBiomePicker::Any => {
                let variants = BiomeT::variants();
                let idx = (hash_u64(seed, cell_x, cell_z) % variants.len() as u64) as usize;
                variants[idx]
            }
            SimpleBiomePicker::AnyOf(variants) => {
                let idx = (hash_u64(seed, cell_x, cell_z) % variants.len() as u64) as usize;
                variants[idx]
            }
            SimpleBiomePicker::Weighted(weights) => {
                // turn hash into rng
                let mut rng = seeded_rng(seed, cell_x, cell_z);
                let roll: f32 = rng.random();

                let mut cumulative = 0.0;
                for (biome, weight) in weights {
                    cumulative += weight;
                    if roll < cumulative {
                        return *biome;
                    }
                }
                // fallback (shouldn’t happen if weights sum to 1.0)
                weights.last().unwrap().0
            }
        }
    }
}

// impl<BiomeT: Biome + 'static> SimpleBiomePicker<BiomeT> {
//     pub fn pick_biome(&self, seed: u64, cell_x: i32, cell_z: i32) -> BiomeT {
//         match self {
//             SimpleBiomePicker::UniformDistribution => {
//                 let variants = BiomeT::variants();
//                 let idx = (hash_u64(seed, cell_x, cell_z) % variants.len() as u64) as usize;
//                 variants[idx]
//             }
//             SimpleBiomePicker::Weighted(weights) => {
//                 // turn hash into rng
//                 let mut rng = seeded_rng(seed, cell_x, cell_z);
//                 let roll: f32 = rng.random();

//                 let mut cumulative = 0.0;
//                 for (biome, weight) in weights {
//                     cumulative += weight;
//                     if roll < cumulative {
//                         return *biome;
//                     }
//                 }
//                 // fallback (shouldn’t happen if weights sum to 1.0)
//                 weights.last().unwrap().0
//             }
//         }
//     }
// }
