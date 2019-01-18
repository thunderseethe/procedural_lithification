use amethyst::core::nalgebra::Point3;
use noise::{NoiseFn, SuperSimplex};
use rayon::prelude::*;

use crate::chunk::{Chunk, DIRT_BLOCK};
use crate::octree::Octree;

struct Terrain {
    simplex: SuperSimplex,
}

impl Terrain {
    pub fn new() -> Self {
        Terrain {
            simplex: SuperSimplex::new(),
        }
    }

    fn generate_chunk<R>(&self) -> Chunk {
        let xs = (1u16..256).into_par_iter();
        let ys = (1u16..256).into_par_iter();
        let zs = (1u16..256).into_par_iter();
        xs.zip(ys).zip(zs).map(|((x, y), z)| {
            let pos = Point3::new(x, y, z);
            let e = self.simplex.get([x as f64, y as f64, z as f64]);
            let data = if e > 0.5 { Some(DIRT_BLOCK) } else { None };
            Octree::new(pos, data, 0)
        });
        Chunk::default()
    }
}
