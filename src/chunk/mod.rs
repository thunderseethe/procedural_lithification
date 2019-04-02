use crate::mut_ptr::MultiThreadMutPtr;
use crate::octree::{octant_dimensions::*, octant_face::OctantFace, octree_data::OctreeData, *};
use amethyst::{
    core::nalgebra::{convert, Point3, Scalar, Unit, Vector2, Vector3},
    renderer::{MeshData, PosNormTangTex, PosNormTex},
};
use array_init::array_init;
use num_traits::One;
use num_traits::{FromPrimitive, ToPrimitive, Zero};
use rayon::iter::{plumbing::*, *};
use std::{borrow::Borrow, sync::Arc};

pub mod block;
pub mod chunk_builder;
pub mod mesher;

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
        let root_octree = &self.octree;
        let chunk_render_pos: Point3<f32> = Point3::new(
            (self.pos.x * 256) as f32,
            (self.pos.y * 256) as f32,
            (self.pos.z * 256) as f32,
        );
        self.octree.map(
            || None,
            |_| {
                // Trivial cube
                let mesh = cube_mesh(
                    Point3::new(0.0, 0.0, 0.0),
                    256.0,
                    &[true, true, true, true, true, true],
                )
                .into();
                Some(vec![(chunk_render_pos, mesh)])
            },
            |children| {
                Some(
                    children
                        .iter()
                        .filter_map(|octree| {
                            if octree.is_empty() {
                                return None;
                            }
                            //let mesh: MeshData = octree
                            //    .par_iter()
                            //    .map(|(dim, _)| {
                            //        let faces: [bool; 6] = array_init(|i| {
                            //            let face = OctantFace::from_usize(i).unwrap();
                            //            if root_octree.face_boundary_adjacent(&dim, face) {
                            //                true
                            //            } else {
                            //                root_octree.check_octant_face_visible(
                            //                    dim.face_adjacent_point(face),
                            //                    dim.diameter(),
                            //                )
                            //            }
                            //        });
                            //        cube_mesh(
                            //            convert(dim.bottom_left()),
                            //            dim.diameter() as f32,
                            //            &faces,
                            //        )
                            //    })
                            //    .reduce(
                            //        || Vec::new(),
                            //        |mut vec1, vec2| {
                            //            vec1.extend(vec2);
                            //            vec1
                            //        },
                            //    )
                            //    .into();
                            let mesher = Mesher::new(&octree);
                            let quads = mesher.generate_quads_array();
                            let mut mesh_data: Vec<PosNormTex> =
                                Vec::with_capacity(quads.len() * 6);
                            mesh_data.extend(
                                quads
                                    .into_iter()
                                    .flat_map(|quad| quad.mesh_coords(&self.pos)),
                            );
                            Some((chunk_render_pos, mesh_data.into()))
                        })
                        .collect(),
                )
            },
        )
    }

    pub fn block_iter<'a>(&'a self) -> SingleBlockIterator<'a> {
        SingleBlockIterator {
            iter: self.octree.iter(),
            state: None,
        }
    }

    pub fn iter<'a>(&'a self) -> OctreeIterator<'a, Block> {
        self.octree.iter()
    }
}

pub struct SingleBlockIterator<'a> {
    iter: OctreeIterator<'a, Block>,
    state: Option<(&'a OctantDimensions, &'a Block, Point3<Number>)>,
}

impl<'a> SingleBlockIterator<'a> {
    fn increment(&self, dim: &'a OctantDimensions, point: Point3<Number>) -> Point3<Number> {
        let mut result = Point3::new(point.x + 1, point.y, point.z);
        if result.x > dim.x_max() {
            result.x = dim.x_min();
            result.y += 1;
        }
        if result.y > dim.y_max() {
            result.y = dim.y_min();
            result.z += 1;
        }
        if result.z > dim.z_max() {
            panic!("Iter should have stopped before leaving dimension bounds");
        }
        return result;
    }
}

impl<'a> Iterator for SingleBlockIterator<'a> {
    type Item = (Point3<Number>, &'a Block);

    fn next(&mut self) -> Option<Self::Item> {
        self.state
            .and_then(|(dim, block, point)| {
                if convert::<Point3<u8>, Point3<u16>>(point) == dim.top_right() {
                    self.state = None;
                    self.next()
                } else {
                    let new_point = self.increment(dim, point);
                    self.state = Some((dim, block, new_point));
                    Some((new_point, block))
                }
            })
            .or_else(|| {
                self.iter.next().map(|(dim, block)| {
                    let point = Point3::new(dim.x_min(), dim.y_min(), dim.z_min());
                    self.state = Some((dim, block, point));
                    (point, block)
                })
            })
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

pub fn cube_mesh(pos: Point3<f32>, size: f32, faces: &[bool; 6]) -> Vec<PosNormTangTex> {
    // normal
    let n = [
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(0.0, -1.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(-1.0, 0.0, 0.0),
    ];
    // tangent
    let t = [
        Vector3::new(-1.0, 0.0, 0.0),
        Vector3::new(0.0, -1.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(0.0, 0.0, 1.0),
    ];

    let vertex_count = faces.iter().map(|f| if *f { 6 } else { 0 }).sum();
    let mut vec = Vec::with_capacity(vertex_count);

    // textures
    let tx = [
        Vector2::new(0.0, 0.0),
        Vector2::new(size, 0.0),
        Vector2::new(0.0, size),
        Vector2::new(size, size),
    ];
    // vertices
    let base = Vector3::new(pos.x, pos.y, pos.z);
    let v = [
        base + Vector3::new(0.0, 0.0, size),   // 0
        base + Vector3::new(size, 0.0, size),  // 1
        base + Vector3::new(0.0, size, size),  // 2
        base + Vector3::new(size, size, size), // 3
        base + Vector3::new(0.0, size, 0.0),   // 4
        base + Vector3::new(size, size, 0.0),  // 5
        base + Vector3::new(0.0, 0.0, 0.0),    // 6
        base + Vector3::new(size, 0.0, 0.0),   // 7
    ];
    // Back
    if faces[0] {
        vec.push(pos_norm_tang_tex(v[0], n[0], t[0], tx[0])); // (0, 0, 1)
        vec.push(pos_norm_tang_tex(v[1], n[0], t[0], tx[1])); // (1, 0, 1)
        vec.push(pos_norm_tang_tex(v[2], n[0], t[0], tx[2])); // (0, 1, 1)
        vec.push(pos_norm_tang_tex(v[2], n[0], t[0], tx[2])); // (0, 1, 1)
        vec.push(pos_norm_tang_tex(v[1], n[0], t[0], tx[1])); // (1, 0, 1)
        vec.push(pos_norm_tang_tex(v[3], n[0], t[0], tx[3])); // (1, 1, 1)
    }
    // Up
    if faces[1] {
        vec.push(pos_norm_tang_tex(v[2], t[1], n[1], tx[0])); // (0, 1, 1)
        vec.push(pos_norm_tang_tex(v[3], t[1], n[1], tx[1])); // (1, 1, 1)
        vec.push(pos_norm_tang_tex(v[4], t[1], n[1], tx[2])); // (0, 1, 0)
        vec.push(pos_norm_tang_tex(v[4], t[1], n[1], tx[2])); // (0, 1, 0)
        vec.push(pos_norm_tang_tex(v[3], t[1], n[1], tx[1])); // (1, 1, 1)
        vec.push(pos_norm_tang_tex(v[5], t[1], n[1], tx[3])); // (1, 1, 0)
    }
    // Front
    if faces[2] {
        vec.push(pos_norm_tang_tex(v[4], t[2], n[2], tx[3])); // (0, 1, 0)
        vec.push(pos_norm_tang_tex(v[5], t[2], n[2], tx[2])); // (1, 1, 0)
        vec.push(pos_norm_tang_tex(v[6], t[2], n[2], tx[1])); // (0, 0, 0)
        vec.push(pos_norm_tang_tex(v[6], t[2], n[2], tx[1])); // (0, 0, 0)
        vec.push(pos_norm_tang_tex(v[5], t[2], n[2], tx[2])); // (1, 1, 0)
        vec.push(pos_norm_tang_tex(v[7], t[2], n[2], tx[0])); // (1, 0, 0)
    }
    // Down
    if faces[3] {
        vec.push(pos_norm_tang_tex(v[6], t[3], n[3], tx[0])); // (0, 0, 0)
        vec.push(pos_norm_tang_tex(v[7], t[3], n[3], tx[1])); // (1, 0, 0)
        vec.push(pos_norm_tang_tex(v[0], t[3], n[3], tx[2])); // (0, 0, 1)
        vec.push(pos_norm_tang_tex(v[0], t[3], n[3], tx[2])); // (0, 0, 1)
        vec.push(pos_norm_tang_tex(v[7], t[3], n[3], tx[1])); // (1, 0, 0)
        vec.push(pos_norm_tang_tex(v[1], t[3], n[3], tx[3])); // (1, 0, 1)
    }
    // Right
    if faces[4] {
        vec.push(pos_norm_tang_tex(v[1], t[4], n[4], tx[0])); // (1, 0, 1)
        vec.push(pos_norm_tang_tex(v[7], t[4], n[4], tx[1])); // (1, 0, 0)
        vec.push(pos_norm_tang_tex(v[3], t[4], n[4], tx[2])); // (1, 1, 1)
        vec.push(pos_norm_tang_tex(v[3], t[4], n[4], tx[2])); // (1, 1, 1)
        vec.push(pos_norm_tang_tex(v[7], t[4], n[4], tx[1])); // (1, 0, 0)
        vec.push(pos_norm_tang_tex(v[5], t[4], n[4], tx[3])); // (1, 1, 0)
    }
    // Left
    if faces[5] {
        vec.push(pos_norm_tang_tex(v[6], t[5], n[5], tx[0])); // (0, 0, 0)
        vec.push(pos_norm_tang_tex(v[0], t[5], n[5], tx[1])); // (0, 0, 1)
        vec.push(pos_norm_tang_tex(v[4], t[5], n[5], tx[2])); // (0, 1, 0)
        vec.push(pos_norm_tang_tex(v[4], t[5], n[5], tx[2])); // (0, 1, 0)
        vec.push(pos_norm_tang_tex(v[0], t[5], n[5], tx[1])); // (0, 0, 1)
        vec.push(pos_norm_tang_tex(v[2], t[5], n[5], tx[3])); // (0, 1, 1)
    }
    return vec;
}

fn pos_norm_tang_tex(
    position: Vector3<f32>,
    normal: Vector3<f32>,
    tangent: Vector3<f32>,
    tex_coord: Vector2<f32>,
) -> PosNormTangTex {
    PosNormTangTex {
        position,
        normal,
        tangent,
        tex_coord,
    }
}

#[cfg(test)]
mod test {
    use super::{Chunk, Point3};
    use crate::octree::Octree;
    use std::collections::HashSet;

    macro_rules! set {
        ($($ele:expr),*) => {{
            let mut set = HashSet::new();
            $(
                set.insert($ele);
            )*
            set
        }};
    }

    #[test]
    fn test_chunk_iterator() {
        let mut chunk = Chunk::new(Point3::new(0, 0, 0), Octree::with_uniform_dimension(8));
        chunk
            .place_block(Point3::new(0, 0, 0), 1)
            .place_block(Point3::new(0, 0, 1), 2)
            .place_block(Point3::new(0, 1, 0), 3)
            .place_block(Point3::new(0, 1, 1), 4)
            .place_block(Point3::new(1, 0, 0), 5)
            .place_block(Point3::new(1, 0, 1), 6)
            .place_block(Point3::new(1, 1, 0), 7)
            .place_block(Point3::new(1, 1, 1), 8);

        let expected = set![
            (Point3::new(1, 1, 1), &8),
            (Point3::new(1, 1, 0), &7),
            (Point3::new(1, 0, 1), &6),
            (Point3::new(1, 0, 0), &5),
            (Point3::new(0, 1, 1), &4),
            (Point3::new(0, 1, 0), &3),
            (Point3::new(0, 0, 1), &2),
            (Point3::new(0, 0, 0), &1)
        ];
        for point_and_block in chunk.block_iter() {
            assert!(
                expected.contains(&point_and_block),
                "Expected {:?} at point {:?}",
                point_and_block.1,
                point_and_block.0
            );
        }
    }

    #[test]
    fn test_chunk_insertions() {
        let mut chunk = Chunk::new(Point3::new(0, 0, 0), Octree::with_uniform_dimension(8));
        for _ in 0..1000 {
            chunk.place_block(
                Point3::new(
                    rand::random::<u8>().into(),
                    rand::random::<u8>().into(),
                    rand::random::<u8>().into(),
                ),
                1234,
            );
        }

        println!("{:?}", chunk);
    }

}
