use amethyst::core::nalgebra::{Point3, Scalar};
use noise::{NoiseFn, OpenSimplex};
use rayon::prelude::*;
use std::{
    borrow::Borrow,
    cmp::{Ord, Ordering},
    fmt,
    sync::Arc,
};

use crate::chunk::{Block, Chunk, DIRT_BLOCK};
use crate::octree::{Number, OctantDimensions, Octree};

pub struct Terrain {
    simplex: OpenSimplex,
}

// Wrapper to provide ordering for points so they can be sorted.
// This ordering is abritrary and doesn't matter so it is kept iternal to terrain generation.
#[derive(PartialEq, Eq, Clone)]
struct OrdPoint3<N: Scalar> {
    p: Point3<N>,
}
impl<N: Scalar> OrdPoint3<N> {
    fn new(p: Point3<N>) -> Self {
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
        write!(
            f,
            "{{ x: {:?}, y: {:?}, z: {:?} }}",
            self.p.x, self.p.y, self.p.z
        )
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

type ParentOctantVec = Vec<(OrdPoint3<Number>, Octree<Block>)>;
impl Terrain {
    pub fn new() -> Self {
        Terrain {
            simplex: OpenSimplex::new(),
        }
    }

    pub fn generate_chunk(&self) -> Chunk {
        let xyzs = (0u16..256)
            .into_par_iter()
            .flat_map(|x| (0u16..256).into_par_iter().map(move |y| (x, y)))
            .flat_map(|(x, y)| (0u16..256).into_par_iter().map(move |z| (x, y, z)));

        let mut intermediate_octrees: Vec<Octree<Block>> = xyzs
            .map(|(x, y, z)| {
                let pos = Point3::new(x, y, z);
                let e = self.simplex.get([x as f64, y as f64, z as f64]);
                let data = if e > 1.0 { Some(DIRT_BLOCK) } else { None };
                Octree::new(pos, data, 0)
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
