use crate::octree::octree_data::OctreeData::Leaf;
use amethyst::core::nalgebra::{Point3, Scalar};
use noise::{NoiseFn, Perlin};
use rayon::prelude::*;
use std::{
    borrow::Borrow,
    cmp::{Ord, Ordering},
    fmt,
    sync::Arc,
};

use crate::chunk::{block::DIRT_BLOCK, chunk_builder::ChunkBuilder, Chunk};
use crate::octree::Number;

pub struct Terrain {
    perlin: Perlin,
}

// Wrapper to provide ordering for points so they can be sorted.
// This ordering is abritrary and doesn't matter so it is kept iternal to terrain generation.
#[derive(PartialEq, Eq, Clone)]
pub struct OrdPoint3<N: Scalar> {
    p: Point3<N>,
}
impl<N: Scalar> OrdPoint3<N> {
    pub fn new(p: Point3<N>) -> Self {
        OrdPoint3 { p }
    }
}
impl<N: Ord + PartialEq + Scalar> PartialOrd for OrdPoint3<N> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
impl<N: Ord + Eq + Scalar> Ord for OrdPoint3<N> {
    fn cmp(&self, other: &Self) -> Ordering {
        use std::cmp::Ordering::*;
        let cmps = (
            self.p.x.cmp(&other.p.x),
            self.p.y.cmp(&other.p.y),
            self.p.z.cmp(&other.p.z),
        );
        match cmps {
            (Greater, _, _) => Greater,
            (Equal, Greater, _) => Greater,
            (Equal, Equal, Greater) => Greater,
            (Equal, Equal, Equal) => Equal,
            (_, _, _) => Less,
        }
    }
}
impl<N: Scalar> fmt::Debug for OrdPoint3<N> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("Point3")
            .field("x", &self.p.x)
            .field("y", &self.p.y)
            .field("z", &self.p.z)
            .finish()
    }
}
impl<N: Scalar> Into<Point3<N>> for OrdPoint3<N> {
    fn into(self) -> Point3<N> {
        self.p
    }
}
impl<N: Scalar> From<Point3<N>> for OrdPoint3<N> {
    fn from(p: Point3<N>) -> Self {
        OrdPoint3::new(p)
    }
}

impl Terrain {
    pub fn new() -> Self {
        Terrain {
            perlin: Perlin::new(),
        }
    }

    pub fn generate_chunk<P>(&self, chunk_pos_ref: P) -> Chunk
    where
        P: Borrow<Point3<i32>>,
    {
        let chunk_pos = chunk_pos_ref.borrow();
        if chunk_pos.y > 0 {
            Chunk::with_empty(*chunk_pos)
        } else if chunk_pos.y < 0 {
            Chunk::with_block(*chunk_pos, DIRT_BLOCK)
        } else {
            self.y_zero_chunk_generator(chunk_pos)
        }
    }

    fn y_zero_chunk_generator<P>(&self, chunk_pos_ref: P) -> Chunk
    where
        P: Borrow<Point3<i32>>,
    {
        let chunk_pos = chunk_pos_ref.borrow();
        let height_map: [[u8; 256]; 256] = array_init::array_init(|x| {
            array_init::array_init(|z| {
                let nx = (chunk_pos.x as f64) + (x as f64 / 256.0) - 0.5;
                let nz = (chunk_pos.z as f64) + (z as f64 / 256.0) - 0.5;
                let noise = self.perlin.get([nx, nz])
                    + 0.5 * self.perlin.get([2.0 * nx, 2.0 * nz])
                    + 0.25 * self.perlin.get([4.0 * nx, 4.0 * nz])
                    + 0.13 * self.perlin.get([8.0 * nx, 8.0 * nz])
                    + 0.06 * self.perlin.get([16.0 * nx, 16.0 * nz])
                    + 0.03 * self.perlin.get([32.0 * nx, 32.0 * nz]);
                let noise = noise / (1.0 + 0.5 + 0.25 + 0.13 + 0.06 + 0.03);
                ((noise / 2.0 + 0.5) * 256.0).ceil() as u8
            })
        });
        let generate_block = |p: Point3<Number>| {
            let subarray: [u8; 256] = height_map[p.x as usize];
            let height: u8 = subarray[p.z as usize];
            if p.y <= height {
                Some(DIRT_BLOCK)
            } else {
                None
            }
        };
        let mut chunk_to_be = ChunkBuilder::new(*chunk_pos);
        chunk_to_be.par_iter_mut().for_each(|leaf| {
            let pos = leaf.root_point();
            generate_block(pos).map(|block| leaf.set_data(Leaf(Arc::new(block))));
        });
        chunk_to_be.build()
    }
}
