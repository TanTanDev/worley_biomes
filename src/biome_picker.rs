use rand::Rng;
use serde::{Deserialize, Serialize};

use crate::utils::hash_u64;

///! trait needed to know what variants are available
pub trait Biome: Copy {
    fn variants() -> &'static [Self]; // list of all variants
}

///! used to generates a biome VARIANT, based upon a "cell" position
#[derive(Serialize, Deserialize)]
pub enum BiomeGenerator<BiomeT: Biome> {
    // all variants have same chance of being selected
    UniformDistribution,
    // weighted odds for biomes to be selected
    Weighted(Vec<(BiomeT, f32)>),
}

impl<BiomeT: Biome + 'static> BiomeGenerator<BiomeT> {
    pub fn pick_biome(&self, seed: u64, cell_x: i32, cell_z: i32) -> BiomeT {
        match self {
            BiomeGenerator::UniformDistribution => {
                let variants = BiomeT::variants();
                let idx = (hash_u64(seed, cell_x, cell_z) % variants.len() as u64) as usize;
                variants[idx]
            }
            BiomeGenerator::Weighted(weights) => {
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

fn seeded_rng(seed: u64, x: i32, z: i32) -> impl Rng {
    use rand::SeedableRng;
    let combined = seed ^ ((x as u64) << 32) ^ (z as u64);
    rand::rngs::StdRng::seed_from_u64(combined)
}
