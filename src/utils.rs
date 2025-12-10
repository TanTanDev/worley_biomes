use rand::{Rng, SeedableRng, rngs::StdRng};
use std::hash::{Hash, Hasher};

pub fn hash_u64(seed: u64, x: i32, z: i32) -> u64 {
    let mut hasher = fxhash::FxHasher::default();
    (seed, x, z).hash(&mut hasher);
    hasher.finish()
}

pub fn seeded_rng(seed: u64, x: i32, z: i32) -> impl Rng {
    let combined = seed ^ ((x as u64) << 32) ^ (z as u64);
    StdRng::seed_from_u64(combined)
}
