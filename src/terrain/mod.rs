use amethyst::core::nalgebra::Point3;
use noise::{NoiseFn, SuperSimplex};
use rayon::prelude::*;

use crate::chunk::Chunk;

struct Terrain {
    simplex: SuperSimplex,
}

impl Terrain {
    pub fn new() -> Self {
        Terrain {
            simplex: SuperSimplex::new(),
        }
    }

    fn generate_chunk<R>(&self) -> Chunk,
    {
        let mut chunk = Chunk::default();
        let xs = (0i32..256).into_par_iter();
        let ys = (0i32..256).into_par_iter();
        let zs = (0i32..256).into_par_iter();
        xs.zip(ys)
            .zip(zs)
            .map(|((x, y), z)| {
                (
                    Point3::new(x, y, z),
                    self.simplex.get([x as f64, y as f64, z as f64]),
                )
            })
            .filter_map(|(pos, e)| if e > 0.5 { None } else { Some((pos, 1234)) })
            .fold(
                || Chunk::default(),
                |(chunk, (pos, block))| chunk.place_block(pos, block),
            )
    }
}
