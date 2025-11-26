use criterion::{criterion_group, criterion_main, Criterion};
use std::hint::black_box;

use serde::{Deserialize, Serialize};
use worley_biomes::prelude::*;

#[derive(Clone, Copy, Debug, Serialize, Deserialize, Default)]
enum BiomeType {
    #[default]
    Desert,
    Forest,
    Snow,
    Plains,
}

impl BiomeVariants for BiomeType {
    fn variants() -> &'static [Self] {
        &[Self::Desert, Self::Forest, Self::Snow, Self::Plains]
    }
}

#[inline]
fn sample_worley(worley: &Worley<BiomeType, SimpleBiomePicker<BiomeType>>, x: f64, z: f64) {
    let _ = worley.get(0, x, z);
}

#[inline]
fn sample_32x32(worley: &Worley<BiomeType, SimpleBiomePicker<BiomeType>>) {
    for z in 0..32 {
        for x in 0..32 {
            let _ = worley.get(0, x as f64, z as f64);
        }
    }
}

// test how percent elimination improves performance
// by increasing the kill percent, we should get a clear increase in performance
#[inline]
fn heavy_k_post_calculation(
    worley: &Worley<BiomeType, SimpleBiomePicker<BiomeType>>,
    x: f64,
    z: f64,
) {
    let biomes = worley.get(0, x as f64, z as f64);
    for (_biome, weight) in biomes.iter() {
        std::thread::sleep(std::time::Duration::from_millis(1));
    }
}

fn default_worley() -> Worley<BiomeType, SimpleBiomePicker<BiomeType>> {
    let mut worley: Worley<BiomeType, SimpleBiomePicker<BiomeType>> = Worley::default();
    worley.zoom = 62.0;
    worley.sharpness = 20.0;
    worley.k = 3;
    worley
}

fn criterion_benchmark(c: &mut Criterion) {
    use rand::Rng;
    let worley = default_worley();
    let mut worley_k_8 = default_worley();
    worley_k_8.k = 8;
    c.bench_function("1 sample", |b| {
        b.iter_with_setup(
            || {
                let mut rng = rand::rng();
                (
                    black_box(rng.random_range(-100.0..100.0)),
                    black_box(rng.random_range(-100.0..100.0)),
                )
            },
            |(x, y)| sample_worley(&worley, x, y),
        )
    });
    c.bench_function("32x32 sample", |b| {
        b.iter(|| sample_32x32(black_box(&worley)));
    });
    c.bench_function("32x32 sample: surpass tinyvec", |b| {
        b.iter(|| sample_32x32(black_box(&worley_k_8)));
    });
    c.bench_function("heavy k post calculation", |b| {
        b.iter_with_setup(
            || {
                let mut rng = rand::rng();
                (
                    black_box(rng.random_range(-100.0..100.0)),
                    black_box(rng.random_range(-100.0..100.0)),
                )
            },
            |(x, y)| heavy_k_post_calculation(&worley, x, y),
        )
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
