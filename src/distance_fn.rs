use serde::{Deserialize, Serialize};

///! what distance function to use to measure distance to worlay
#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub enum DistanceFn {
    Euclidean,
    EuclideanSquared,
    Manhattan,
    Chebyshev,
    // combines euclidean with manhattan
    Hybrid,
}

pub fn distance(dx: f64, dz: f64, metric: DistanceFn) -> f64 {
    match metric {
        DistanceFn::Euclidean => (dx * dx + dz * dz).sqrt(),
        DistanceFn::EuclideanSquared => dx * dx + dz * dz,
        DistanceFn::Manhattan => dx.abs() + dz.abs(),
        DistanceFn::Chebyshev => dx.abs().max(dz.abs()),
        DistanceFn::Hybrid => ((dx * dx + dz * dz).sqrt() + dx.abs() + dz.abs()) / 2.0,
    }
}
