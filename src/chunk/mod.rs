use crate::mut_ptr::MultiThreadMutPtr;
use crate::octree::{octant_dimensions::*, octree_data::OctreeData, *};
use amethyst::{
    core::nalgebra::{convert, Point3, Vector2, Vector3},
    renderer::{MeshData, PosNormTex},
};
use rayon::iter::{plumbing::*, *};
use std::{borrow::Borrow, sync::Arc};

pub mod block;
pub mod chunk_builder;
pub mod mesher;
pub mod file_format;

use block::Block;
use mesher::Mesher;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Chunk {
    pub pos: Point3<i32>,
    pub octree: Octree<Block>,
}

impl Chunk {
    pub fn new(pos: Point3<i32>, octree: Octree<Block>) -> Self {
        Chunk { pos, octree }
    }

    pub fn with_block(pos: Point3<i32>, block: Block) -> Self {
        Chunk {
            pos,
            octree: Octree::with_fields(
                OctreeData::Leaf(Arc::new(block)),
                OctantDimensions::new(Point3::new(0, 0, 0), 256),
                8,
            ),
        }
    }

    pub fn with_empty(pos: Point3<i32>) -> Self {
        Chunk {
            pos,
            octree: Octree::with_uniform_dimension(8),
        }
    }

    pub fn get_block<P>(&self, pos: P) -> Option<Block>
    where
        P: Borrow<Point3<Number>>,
    {
        self.octree.get(pos).map(|arc_block| *arc_block)
    }

    pub fn place_block<P>(&mut self, pos: P, block: Block) -> &mut Self
    where
        P: Borrow<Point3<Number>>,
    {
        self.octree = self.octree.insert(pos, block);
        self
    }

    pub fn generate_mesh(&self) -> Option<Vec<(Point3<f32>, MeshData)>> {
        let chunk_render_pos: Point3<f32> = convert(self.pos * 256)/*Point3::new(
            (self.pos.x * 256) as f32,
            (self.pos.y * 256) as f32,
            (self.pos.z * 256) as f32,
        )*/;
        self.octree.map(
            || None,
            |_| {
                // Trivial cube
                let mesh = cube_mesh(256.0).into();
                Some(vec![(chunk_render_pos, mesh)])
            },
            |children| {
                Some(
                    children
                        .par_iter()
                        .flat_map(|octree| {
                            octree.map(
                                || vec![],
                                |_| {
                                    let octree_offset: Vector3<f32> = convert(octree.root_point().coords)/*Vector3::new(
                                        octree_root.x as f32,
                                        octree_root.y as f32,
                                        octree_root.z as f32,
                                    )*/;
                                    let mesh = cube_mesh(octree.bounds().diameter() as f32).into();
                                    vec![(chunk_render_pos + octree_offset, mesh)]
                                },
                                |children| {
                                    children
                                        .par_iter()
                                        .filter_map(|octree| {
                                            let octree_root_offset: Vector3<f32> = convert(octree.root_point().coords)/*Vector3::new(
                                                octree_root_point.x as f32,
                                                octree_root_point.y as f32,
                                                octree_root_point.z as f32,
                                            )*/;

                                            octree.map(
                                                || None,
                                                |_| {
                                                    Some(
                                                        (
                                                            chunk_render_pos + octree_root_offset,
                                                            cube_mesh(
                                                                octree.bounds().diameter() as f32
                                                            )
                                                            .into(),
                                                        ),
                                                    )
                                                },
                                                |_| {
                                                    let mesher = Mesher::new(&octree);
                                                    let quads = mesher.generate_quads_array();
                                                    let mut mesh_data: Vec<PosNormTex> =
                                                        Vec::with_capacity(quads.len() * 6);
                                                    mesh_data.extend(
                                                        quads
                                                            .into_iter()
                                                            .flat_map(|quad| quad.mesh_coords()),
                                                    );
                                                    Some((
                                                        chunk_render_pos + octree_root_offset,
                                                        mesh_data.into(),
                                                    ))
                                                },
                                            )
                                        })
                                        .collect::<Vec<(Point3<f32>, MeshData)>>()
                                },
                            )
                        })
                        .collect(),
                )
            },
        )
    }

    pub fn iter<'a>(&'a self) -> OctreeIterator<'a, Block> {
        self.octree.iter()
    }
}

macro_rules! node_index {
    ($node:expr, $i:expr) => {
        unsafe { $node.0.add($i).as_mut().unwrap() }
    };
}

pub fn parallel_drive_mut_node_children<'a, ITEM, E, C, F>(
    nodes: &'a mut [Arc<Octree<E>>; 8],
    consumer: C,
    handle_child: F,
) -> C::Result
where
    E: Send + Sync,
    C: UnindexedConsumer<ITEM>,
    F: Fn(&'a mut Arc<Octree<E>>, C) -> C::Result + Send + Sync,
{
    let nodes_ptr = MultiThreadMutPtr::new(nodes.as_mut_ptr());
    let reducer = consumer.to_reducer();
    let (left_half, right_half) = (consumer.split_off_left(), consumer);
    let (ll_quarter, lr_quarter, rl_quarter, rr_quarter) = (
        left_half.split_off_left(),
        left_half,
        right_half.split_off_left(),
        right_half,
    );
    let (lll_octet, llr_octet, lrl_octet, lrr_octet, rll_octet, rlr_octet, rrl_octet, rrr_octet) = (
        ll_quarter.split_off_left(),
        ll_quarter,
        lr_quarter.split_off_left(),
        lr_quarter,
        rl_quarter.split_off_left(),
        rl_quarter,
        rr_quarter.split_off_left(),
        rr_quarter,
    );
    let (left, right) = rayon::join(
        || {
            let reducer = lll_octet.to_reducer();
            let (left, right) = rayon::join(
                || {
                    let r = lll_octet.to_reducer();
                    r.reduce(
                        handle_child(node_index!(nodes_ptr, 0), lll_octet),
                        handle_child(node_index!(nodes_ptr, 1), llr_octet),
                    )
                },
                || {
                    let r = lrl_octet.to_reducer();
                    r.reduce(
                        handle_child(node_index!(nodes_ptr, 2), lrl_octet),
                        handle_child(node_index!(nodes_ptr, 3), lrr_octet),
                    )
                },
            );
            reducer.reduce(left, right)
        },
        || {
            let reducer = rll_octet.to_reducer();
            let (left, right) = rayon::join(
                || {
                    let r = rll_octet.to_reducer();
                    r.reduce(
                        handle_child(node_index!(nodes_ptr, 4), rll_octet),
                        handle_child(node_index!(nodes_ptr, 5), rlr_octet),
                    )
                },
                || {
                    let r = rrl_octet.to_reducer();
                    r.reduce(
                        handle_child(node_index!(nodes_ptr, 6), rrl_octet),
                        handle_child(node_index!(nodes_ptr, 7), rrr_octet),
                    )
                },
            );
            reducer.reduce(left, right)
        },
    );
    reducer.reduce(left, right)
}

pub fn cube_mesh(size: f32) -> Vec<PosNormTex> {
    // normal
    let n = [
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(0.0, -1.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(-1.0, 0.0, 0.0),
    ];

    let mut vec = Vec::with_capacity(36);

    // textures
    let tx = [
        Vector2::new(0.0, 0.0),
        Vector2::new(size, 0.0),
        Vector2::new(0.0, size),
        Vector2::new(size, size),
    ];
    // vertices
    let v = [
        /*base +*/ Vector3::new(0.0, 0.0, size), // 0
        /*base +*/ Vector3::new(size, 0.0, size), // 1
        /*base +*/ Vector3::new(0.0, size, size), // 2
        /*base +*/ Vector3::new(size, size, size), // 3
        /*base +*/ Vector3::new(0.0, size, 0.0), // 4
        /*base +*/ Vector3::new(size, size, 0.0), // 5
        /*base +*/ Vector3::new(0.0, 0.0, 0.0), // 6
        /*base +*/ Vector3::new(size, 0.0, 0.0), // 7
    ];
    // Back
    vec.push(pos_norm_tex(v[0], n[0], tx[0])); // (0, 0, 1)
    vec.push(pos_norm_tex(v[1], n[0], tx[1])); // (1, 0, 1)
    vec.push(pos_norm_tex(v[2], n[0], tx[2])); // (0, 1, 1)
    vec.push(pos_norm_tex(v[2], n[0], tx[2])); // (0, 1, 1)
    vec.push(pos_norm_tex(v[1], n[0], tx[1])); // (1, 0, 1)
    vec.push(pos_norm_tex(v[3], n[0], tx[3])); // (1, 1, 1)
                                               // Up
    vec.push(pos_norm_tex(v[2], n[1], tx[0])); // (0, 1, 1)
    vec.push(pos_norm_tex(v[3], n[1], tx[1])); // (1, 1, 1)
    vec.push(pos_norm_tex(v[4], n[1], tx[2])); // (0, 1, 0)
    vec.push(pos_norm_tex(v[4], n[1], tx[2])); // (0, 1, 0)
    vec.push(pos_norm_tex(v[3], n[1], tx[1])); // (1, 1, 1)
    vec.push(pos_norm_tex(v[5], n[1], tx[3])); // (1, 1, 0)
                                               // Front
    vec.push(pos_norm_tex(v[4], n[2], tx[3])); // (0, 1, 0)
    vec.push(pos_norm_tex(v[5], n[2], tx[2])); // (1, 1, 0)
    vec.push(pos_norm_tex(v[6], n[2], tx[1])); // (0, 0, 0)
    vec.push(pos_norm_tex(v[6], n[2], tx[1])); // (0, 0, 0)
    vec.push(pos_norm_tex(v[5], n[2], tx[2])); // (1, 1, 0)
    vec.push(pos_norm_tex(v[7], n[2], tx[0])); // (1, 0, 0)
                                               // Down
    vec.push(pos_norm_tex(v[6], n[3], tx[0])); // (0, 0, 0)
    vec.push(pos_norm_tex(v[7], n[3], tx[1])); // (1, 0, 0)
    vec.push(pos_norm_tex(v[0], n[3], tx[2])); // (0, 0, 1)
    vec.push(pos_norm_tex(v[0], n[3], tx[2])); // (0, 0, 1)
    vec.push(pos_norm_tex(v[7], n[3], tx[1])); // (1, 0, 0)
    vec.push(pos_norm_tex(v[1], n[3], tx[3])); // (1, 0, 1)
                                               // Right
    vec.push(pos_norm_tex(v[1], n[4], tx[0])); // (1, 0, 1)
    vec.push(pos_norm_tex(v[7], n[4], tx[1])); // (1, 0, 0)
    vec.push(pos_norm_tex(v[3], n[4], tx[2])); // (1, 1, 1)
    vec.push(pos_norm_tex(v[3], n[4], tx[2])); // (1, 1, 1)
    vec.push(pos_norm_tex(v[7], n[4], tx[1])); // (1, 0, 0)
    vec.push(pos_norm_tex(v[5], n[4], tx[3])); // (1, 1, 0)
                                               // Left
    vec.push(pos_norm_tex(v[6], n[5], tx[0])); // (0, 0, 0)
    vec.push(pos_norm_tex(v[0], n[5], tx[1])); // (0, 0, 1)
    vec.push(pos_norm_tex(v[4], n[5], tx[2])); // (0, 1, 0)
    vec.push(pos_norm_tex(v[4], n[5], tx[2])); // (0, 1, 0)
    vec.push(pos_norm_tex(v[0], n[5], tx[1])); // (0, 0, 1)
    vec.push(pos_norm_tex(v[2], n[5], tx[3])); // (0, 1, 1)
    return vec;
}

fn pos_norm_tex(
    position: Vector3<f32>,
    normal: Vector3<f32>,
    tex_coord: Vector2<f32>,
) -> PosNormTex {
    PosNormTex {
        position,
        normal,
        tex_coord,
    }
}

#[cfg(test)]
mod test {
    use super::{Chunk, Point3};
    use crate::octree::Octree;
    use std::collections::HashSet;

    #[test]
    fn test_chunk_insertions() {
        let mut points = HashSet::new();
        let mut chunk = Chunk::new(Point3::new(0, 0, 0), Octree::with_uniform_dimension(8));
        for _ in 0..1000 {
            let p = Point3::new(
                rand::random::<u8>().into(),
                rand::random::<u8>().into(),
                rand::random::<u8>().into(),
            );
            chunk.place_block(&p, 1234);
            points.insert(p);
        }

        for point in points {
            assert_eq!(chunk.get_block(point), Some(1234));
        }
    }

}
