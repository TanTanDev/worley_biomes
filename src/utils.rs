use std::hash::{DefaultHasher, Hash, Hasher};

pub fn hash_u64(seed: u64, x: i32, z: i32) -> u64 {
    let mut hasher = DefaultHasher::new();
    (seed, x, z).hash(&mut hasher);
    hasher.finish()
}
