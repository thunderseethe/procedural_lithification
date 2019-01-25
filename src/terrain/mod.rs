use amethyst::core::nalgebra::{Point3, Scalar};
use noise::{NoiseFn, OpenSimplex, Perlin};
use rayon::prelude::*;
use std::{
    cmp::{Ord, Ordering},
    fmt, ptr,
    sync::Arc,
};

use crate::chunk::{Block, Chunk, DIRT_BLOCK};
use crate::octree::{Number, OctantDimensions, Octree};

pub struct Terrain {
    simplex: OpenSimplex,
    perlin: Perlin,
    block_threshold: f64,
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

struct MultiThreadGeneration<T> {
    ptr: *const T,
}
impl<T> MultiThreadGeneration<T> {
    pub fn new(ptr: *const T) -> Self {
        MultiThreadGeneration { ptr }
    }
}
unsafe impl<T> Send for MultiThreadGeneration<T> {}
unsafe impl<T> Sync for MultiThreadGeneration<T> {}

struct MutMultiThreadGeneration<T> {
    ptr: *mut T,
}
impl<T> MutMultiThreadGeneration<T> {
    pub fn new(ptr: *mut T) -> Self {
        MutMultiThreadGeneration { ptr }
    }
}
unsafe impl<T> Send for MutMultiThreadGeneration<T> {}
unsafe impl<T> Sync for MutMultiThreadGeneration<T> {}

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
        if y < 64.0 {
            Some(DIRT_BLOCK)
        } else {
            None
        }
    }

    pub fn generate_depth0(&self) -> Vec<Octree<Block>> {
        let depth0_size = 2097152 * 8;
        let mut depth0: Vec<Octree<Block>> =
            vec![Octree::new(Point3::new(0, 0, 0), None, 0); depth0_size];
        let depth0_ptr = MutMultiThreadGeneration::new(depth0.as_mut_ptr());
        triplets(256).for_each(|(x, y, z)| {
            let data = self.generate_block(x as f64, y as f64, z as f64);
            let pos = Point3::new(x, y, z);
            let (parent_index, sub_octant_index) = Terrain::parent_indices(pos, 0);
            unsafe {
                let p = depth0_ptr.ptr.add(parent_index + sub_octant_index);
                //match ptr.as_mut() {
                //    Some(octant) => {
                //        *octant = Octree::new(pos, data, 0);
                //    }
                //    None => panic!("unexpected null array"),
                //}
                ptr::write(p, Octree::new(pos, data, 0));
            }
        });
        depth0
    }

    pub fn generate_chunk(&self) -> Chunk {
        let mut depth_n = self.generate_depth0();
        for height in 1..8 {
            let inv_height = 8 - height;
            let inv_multiple = usize::pow(2, inv_height);
            let multiple = u16::pow(2, height);
            // For capacity we want to remove a factor of 2
            // Basically we're calculating (2^(8 - height - 1))^3 which is the number of parent nodes we'll have
            // If we just calculated (2^(8 - height))^3 we would have far too many nodes for each height and waste allocations
            let capacity = usize::pow(2, (inv_height - 1) * 3) * 8;
            let mut depth_n1: Vec<Octree<Block>> =
                vec![Octree::new(Point3::new(0, 0, 0), None, 0); capacity];
            let depth_n_ptr = MultiThreadGeneration::new(depth_n.as_ptr());
            let depth_n1_ptr = MutMultiThreadGeneration::new(depth_n1.as_mut_ptr());
            triplets(inv_multiple as u16).for_each(|(x, y, z)| {
                let child_index = index(inv_multiple, x.into(), y.into(), z.into());
                let children: [Arc<Octree<Block>>; 8] =
                    array_init::array_init(|i| Arc::new(depth_n[child_index + i].clone()));
                let pos = Point3::new(x * multiple, y * multiple, z * multiple);
                let (parent_index, sub_octant_index) = Terrain::parent_indices(pos, height);
                unsafe {
                    ptr::write(
                        depth_n1_ptr.ptr.add(parent_index + sub_octant_index),
                        Octree::with_children(children, pos, height),
                    );
                }
            });
            depth_n = depth_n1;
        }
        let children: [Arc<Octree<Block>>; 8] =
            array_init::array_init(|i| Arc::new(depth_n[i].clone()));
        Chunk::new(Octree::with_children(children, Point3::new(0, 0, 0), 8))
    }

    #[inline]
    fn parent_indices(pos: Point3<u16>, height: u32) -> (usize, usize) {
        let parent_height = height + 1;
        let parent_pos = OctantDimensions::nearest_octant_point(pos, parent_height);
        let parent_octant = OctantDimensions::new(parent_pos, 1 << parent_height);
        let parent_index = index(
            usize::pow(2, 8 - parent_height),
            (parent_pos.x >> parent_height).into(),
            (parent_pos.y >> parent_height).into(),
            (parent_pos.z >> parent_height).into(),
        );
        let sub_octant_index: usize = parent_octant.get_octant(pos).into();
        (parent_index, sub_octant_index)
    }

    pub fn old_generate_chunk(&self) -> Chunk {
        let xyzs = triplets(256);

        let mut intermediate_octrees: Vec<Octree<Block>> = xyzs
            .map(|(x, y, z)| {
                let pos = Point3::new(x, y, z);
                let p = [x as f64 / 10.0, y as f64 / 10.0, z as f64 / 10.0];
                let e = self.perlin.get(p);
                let data = if e > self.block_threshold {
                    Some(DIRT_BLOCK)
                } else {
                    None
                };
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

#[cfg(test)]
mod test {
    use test::Bencher;
}
