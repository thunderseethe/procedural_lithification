use crate::octree::octree_data::OctreeData::Leaf;
use amethyst::core::nalgebra::{Point3, Scalar};
use noise::{NoiseFn, OpenSimplex, Perlin};
use rayon::prelude::*;
use std::{
    cmp::{Ord, Ordering},
    fmt, ptr,
    sync::Arc,
};

use crate::chunk::{Block, Chunk, ChunkBuilder, DIRT_BLOCK};
use crate::octree::{Number, octant_dimensions::OctantDimensions, Octree};

pub struct Terrain {
    simplex: OpenSimplex,
    perlin: Perlin,
    block_threshold: f64,
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

fn triplets(max: u16) -> impl ParallelIterator<Item = (u16, u16, u16)> {
    (0u16..max).into_par_iter().flat_map(move |x| {
        (0u16..max)
            .into_par_iter()
            .flat_map(move |y| (0u16..max).into_par_iter().map(move |z| (x, y, z)))
    })
}

#[inline(always)]
pub fn index(size: usize, x: usize, y: usize, z: usize) -> usize {
    ((x * size * size) + (y * size) + z) * 8
}

type ParentOctantVec = Vec<(OrdPoint3<Number>, Octree<Block>)>;
impl Terrain {
    pub fn new(threshold: f64) -> Self {
        Terrain {
            simplex: OpenSimplex::new(),
            perlin: Perlin::new(),
            block_threshold: threshold,
        }
    }

    pub fn generate_block(&self, x: f64, y: f64, z: f64) -> Option<Block> {
        //let p = [x / 10.0, y / 10.0, z / 10.0];
        //let e = self.perlin.get(p);
        if y < 128.0 {
            Some(DIRT_BLOCK)
        } else {
            None
        }
    }

    pub fn generate_chunk(&self) -> Chunk {
        let mut chunk_to_be = ChunkBuilder::new();
        chunk_to_be.par_iter_mut().for_each(|leaf| {
            let pos = leaf.root_point();
            self.generate_block(pos.x as f64, pos.y as f64, pos.z as f64)
                .map(|block| leaf.set_data(Leaf(Arc::new(block))));
        });
        chunk_to_be.build()
    }

    pub fn old_generate_chunk(&self) -> Chunk {
        let xyzs = triplets(256);

        let mut intermediate_octrees: Vec<Octree<Block>> = xyzs
            .map(|(x, y, z)| {
                Octree::new(
                    Point3::new(x, y, z),
                    self.generate_block(x as f64, y as f64, z as f64),
                    0,
                )
            })
            .collect();

        for height in 0..8 {
            let mut v = Terrain::find_parent_octants(intermediate_octrees.clone().into_par_iter());
            Terrain::sort_octants(&mut v);
            v.into_par_iter()
                .chunks(8)
                .map(|octants| {
                    let parent_pos: Point3<Number> = octants[0].0.clone().into();
                    let children: [Arc<Octree<Block>>; 8] =
                        array_init::array_init(|i| Arc::new(octants[i].1.clone()));
                    Octree::with_children(children, parent_pos, height + 1)
                })
                .collect_into_vec(&mut intermediate_octrees);
        }
        Chunk::new(intermediate_octrees[0].clone())
    }

    fn find_parent_octants<I>(node_iter: I) -> ParentOctantVec
    where
        I: ParallelIterator<Item = Octree<Block>>,
    {
        node_iter
            .map(|octree| {
                let nearest = OctantDimensions::nearest_octant_point(
                    octree.root_point(),
                    octree.height() + 1,
                )
                .into();
                (nearest, octree)
            })
            .collect()
    }

    fn sort_octants(nodes: &mut ParentOctantVec) {
        nodes.par_sort_by(|(octant_a, octree_a), (octant_b, octree_b)| {
            let parent_octants = octant_a.cmp(octant_b);
            if parent_octants == Ordering::Equal {
                let a = OrdPoint3::new(octree_a.root_point());
                let b = OrdPoint3::new(octree_b.root_point());
                b.cmp(&a)
            } else {
                parent_octants
            }
        })
    }
}
