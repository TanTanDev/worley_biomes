use serde::{Deserialize, Serialize};

///! what distance function to use to measure distance to worlay
#[derive(Debug, Copy, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub enum DistanceFn {
    Euclidean,
    EuclideanSquared,
    Manhattan,
    Chebyshev,
    // combines euclidean with manhattan
    Hybrid,
}

impl DistanceFn {
    pub fn to_func(&self) -> fn(f64, f64) -> f64 {
        match self {
            DistanceFn::Euclidean => |dx, dz| (dx * dx + dz * dz).sqrt(),
            DistanceFn::EuclideanSquared => |dx, dz| dx * dx + dz * dz,
            DistanceFn::Manhattan => |dx, dz| dx.abs() + dz.abs(),
            DistanceFn::Chebyshev => |dx, dz| dx.abs().max(dz.abs()),
            DistanceFn::Hybrid => |dx, dz| ((dx * dx + dz * dz).sqrt() + dx.abs() + dz.abs()) / 2.0,
        }
    }
}
