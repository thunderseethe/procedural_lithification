use crate::mut_ptr::MultiThreadMutPtr;
use crate::octree::{octant_dimensions::*, octant_face::OctantFace, octree_data::OctreeData, *};
use amethyst::{
    core::nalgebra::{convert, Point3, Vector2, Vector3},
    renderer::{MeshData, PosNormTangTex},
};
use array_init::array_init;
use num_traits::FromPrimitive;
use rayon::iter::{plumbing::*, *};
use std::{borrow::Borrow, default::Default, sync::Arc};

pub type Block = u32;
pub static AIR_BLOCK: Block = 0;
pub static DIRT_BLOCK: Block = 1;

#[derive(Clone, Debug, Serialize, Deserialize, Eq, PartialEq)]
pub struct Chunk {
    pub pos: Point3<i32>,
    octree: Octree<Block>,
}

impl Chunk {
    pub fn new(pos: Point3<i32>, octree: Octree<Block>) -> Self {
        Chunk { pos, octree }
    }

    pub fn get_block<P>(&self, pos: P) -> Block
    where
        P: Borrow<Point3<Number>>,
    {
        self.octree
            .get(pos)
            .map_or(AIR_BLOCK, |arc| arc.as_ref().clone())
    }

    pub fn place_block<P>(&mut self, pos: P, block: Block) -> &mut Self
    where
        P: Borrow<Point3<Number>>,
    {
        self.octree = self.octree.insert(pos, block);
        self
    }

    pub fn generate_mesh(&self) -> MeshData {
        let root_octree = &self.octree;
        self.octree
            .clone()
            .into_par_iter()
            .map(|(dim, _)| {
                let faces: [bool; 6] = array_init(|i| {
                    let face = OctantFace::from_usize(i).unwrap();
                    if root_octree.face_boundary_adjacent(&dim, face) {
                        true
                    } else {
                        root_octree.check_octant_face_visible(
                            dim.face_adjacent_point(face),
                            dim.diameter(),
                        )
                    }
                });
                cube_mesh(convert(dim.bottom_left()), dim.diameter() as f32, &faces)
            })
            .reduce(
                || Vec::new(),
                |mut vec1, vec2| {
                    vec1.extend(vec2);
                    vec1
                },
            )
            .into()
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

pub fn cube_mesh(pos: Point3<f32>, size: f32, faces: &[bool; 6]) -> Vec<PosNormTangTex> {
    // vertices
    let base = Vector3::new(pos.x, pos.y, pos.z);
    let v = [
        base + Vector3::new(0.0, 0.0, size),
        base + Vector3::new(size, 0.0, size),
        base + Vector3::new(0.0, size, size),
        base + Vector3::new(size, size, size),
        base + Vector3::new(0.0, size, 0.0),
        base + Vector3::new(size, size, 0.0),
        base + Vector3::new(0.0, 0.0, 0.0),
        base + Vector3::new(size, 0.0, 0.0),
    ];
    // textures
    let tx = [
        Vector2::new(0.0, 0.0),
        Vector2::new(size, 0.0),
        Vector2::new(0.0, size),
        Vector2::new(size, size),
    ];
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

    // Back
    if faces[0] {
        vec.push(pos_norm_tang_tex(v[0], n[0], t[0], tx[0]));
        vec.push(pos_norm_tang_tex(v[1], n[0], t[0], tx[1]));
        vec.push(pos_norm_tang_tex(v[2], n[0], t[0], tx[2]));
        vec.push(pos_norm_tang_tex(v[2], n[0], t[0], tx[2]));
        vec.push(pos_norm_tang_tex(v[1], n[0], t[0], tx[1]));
        vec.push(pos_norm_tang_tex(v[3], n[0], t[0], tx[3]));
    }
    // Up
    if faces[1] {
        vec.push(pos_norm_tang_tex(v[2], t[1], n[1], tx[0]));
        vec.push(pos_norm_tang_tex(v[3], t[1], n[1], tx[1]));
        vec.push(pos_norm_tang_tex(v[4], t[1], n[1], tx[2]));
        vec.push(pos_norm_tang_tex(v[4], t[1], n[1], tx[2]));
        vec.push(pos_norm_tang_tex(v[3], t[1], n[1], tx[1]));
        vec.push(pos_norm_tang_tex(v[5], t[1], n[1], tx[3]));
    }
    // Front
    if faces[2] {
        vec.push(pos_norm_tang_tex(v[4], t[2], n[2], tx[3]));
        vec.push(pos_norm_tang_tex(v[5], t[2], n[2], tx[2]));
        vec.push(pos_norm_tang_tex(v[6], t[2], n[2], tx[1]));
        vec.push(pos_norm_tang_tex(v[6], t[2], n[2], tx[1]));
        vec.push(pos_norm_tang_tex(v[5], t[2], n[2], tx[2]));
        vec.push(pos_norm_tang_tex(v[7], t[2], n[2], tx[0]));
    }
    // Down
    if faces[3] {
        vec.push(pos_norm_tang_tex(v[6], t[3], n[3], tx[0]));
        vec.push(pos_norm_tang_tex(v[7], t[3], n[3], tx[1]));
        vec.push(pos_norm_tang_tex(v[0], t[3], n[3], tx[2]));
        vec.push(pos_norm_tang_tex(v[0], t[3], n[3], tx[2]));
        vec.push(pos_norm_tang_tex(v[7], t[3], n[3], tx[1]));
        vec.push(pos_norm_tang_tex(v[1], t[3], n[3], tx[3]));
    }
    // Right
    if faces[4] {
        vec.push(pos_norm_tang_tex(v[1], t[4], n[4], tx[0]));
        vec.push(pos_norm_tang_tex(v[7], t[4], n[4], tx[1]));
        vec.push(pos_norm_tang_tex(v[3], t[4], n[4], tx[2]));
        vec.push(pos_norm_tang_tex(v[3], t[4], n[4], tx[2]));
        vec.push(pos_norm_tang_tex(v[7], t[4], n[4], tx[1]));
        vec.push(pos_norm_tang_tex(v[5], t[4], n[4], tx[3]));
    }
    // Left
    if faces[5] {
        vec.push(pos_norm_tang_tex(v[6], t[5], n[5], tx[0]));
        vec.push(pos_norm_tang_tex(v[0], t[5], n[5], tx[1]));
        vec.push(pos_norm_tang_tex(v[4], t[5], n[5], tx[2]));
        vec.push(pos_norm_tang_tex(v[4], t[5], n[5], tx[2]));
        vec.push(pos_norm_tang_tex(v[0], t[5], n[5], tx[1]));
        vec.push(pos_norm_tang_tex(v[2], t[5], n[5], tx[3]));
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
                if point == dim.top_right() {
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

pub struct ChunkBuilder {
    pos: Point3<i32>,
    tree: Octree<Block>,
}

impl ChunkBuilder {
    pub fn new(pos: Point3<i32>) -> Self {
        ChunkBuilder {
            pos,
            tree: gen_subtree(Point3::new(0, 0, 0), 8),
        }
    }

    fn with_tree(pos: Point3<i32>, tree: Octree<Block>) -> Self {
        ChunkBuilder { pos, tree }
    }

    pub fn build(mut self) -> Chunk {
        self.tree.compress();
        Chunk::new(self.pos, self.tree)
    }
}

impl ParallelIterator for ChunkBuilder {
    type Item = Octree<Block>;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        use crate::octree::{octree_data::OctreeData::Node, parallel_drive_node_children};
        match self.tree.data() {
            Node(nodes) => parallel_drive_node_children(nodes, consumer, |node, cnsmr| {
                ChunkBuilder::with_tree(self.pos, node.clone())
                    .into_par_iter()
                    .drive_unindexed(cnsmr)
            }),
            _ => consumer.into_folder().consume(self.tree).complete(),
        }
    }
}

impl<'data> IntoParallelIterator for &'data mut ChunkBuilder {
    type Iter = IterMut<'data>;
    type Item = <IterMut<'data> as ParallelIterator>::Item;

    fn into_par_iter(self) -> Self::Iter {
        IterMut {
            tree: &mut self.tree,
        }
    }
}

pub struct IterMut<'data> {
    tree: &'data mut Octree<Block>,
}
impl<'data> ParallelIterator for IterMut<'data> {
    type Item = &'data mut Octree<Block>;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        if self.tree.is_node() {
            if self.tree.height() == 1 {
                consumer
                    .into_folder()
                    .consume_iter(
                        self.tree
                            .mut_children()
                            .into_iter()
                            .map(|arc| Arc::get_mut(arc).unwrap()),
                    )
                    .complete()
            } else {
                parallel_drive_mut_node_children(
                    self.tree.mut_children(),
                    consumer,
                    |node, cnsmr| {
                        (IterMut {
                            tree: Arc::make_mut(node),
                        })
                        .into_par_iter()
                        .drive_unindexed(cnsmr)
                    },
                )
            }
        } else {
            consumer.into_folder().consume(self.tree).complete()
        }
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

fn gen_subtree(pos: Point3<Number>, height: u32) -> Octree<Block> {
    // Base case
    if height == 0 {
        Octree::new(pos, None, height)
    } else {
        let child_height = height - 1;
        let dimension = Number::pow(2, child_height);
        let (((hhh, hhl), (hlh, hll)), ((lhh, lhl), (llh, lll))) = rayon::join(
            || {
                rayon::join(
                    || {
                        rayon::join(
                            /* 0 */
                            || {
                                gen_subtree(
                                    Point3::new(
                                        pos.x + dimension,
                                        pos.y + dimension,
                                        pos.z + dimension,
                                    ),
                                    child_height,
                                )
                            },
                            /* 1 */
                            || {
                                gen_subtree(
                                    Point3::new(pos.x + dimension, pos.y + dimension, pos.z),
                                    child_height,
                                )
                            },
                        )
                    },
                    || {
                        rayon::join(
                            /* 2 */
                            || {
                                gen_subtree(
                                    Point3::new(pos.x + dimension, pos.y, pos.z + dimension),
                                    child_height,
                                )
                            },
                            /* 3 */
                            || {
                                gen_subtree(
                                    Point3::new(pos.x + dimension, pos.y, pos.z),
                                    child_height,
                                )
                            },
                        )
                    },
                )
            },
            || {
                rayon::join(
                    || {
                        rayon::join(
                            /* 4 */
                            || {
                                gen_subtree(
                                    Point3::new(pos.x, pos.y + dimension, pos.z + dimension),
                                    child_height,
                                )
                            },
                            /* 5 */
                            || {
                                gen_subtree(
                                    Point3::new(pos.x, pos.y + dimension, pos.z),
                                    child_height,
                                )
                            },
                        )
                    },
                    || {
                        rayon::join(
                            /* 6 */
                            || {
                                gen_subtree(
                                    Point3::new(pos.x, pos.y, pos.z + dimension),
                                    child_height,
                                )
                            },
                            /* 7 */
                            || gen_subtree(Point3::new(pos.x, pos.y, pos.z), child_height),
                        )
                    },
                )
            },
        );
        Octree::with_fields(
            OctreeData::Node([
                Arc::new(hhh),
                Arc::new(hhl),
                Arc::new(hlh),
                Arc::new(hll),
                Arc::new(lhh),
                Arc::new(lhl),
                Arc::new(llh),
                Arc::new(lll),
            ]),
            OctantDimensions::new(pos, u16::pow(2, height)),
            height,
        )
    }
}
#[cfg(test)]
mod test {
    use super::{Chunk, Point3};
    use crate::octree::Octree;

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

        let mut iter = chunk.block_iter();

        assert_eq!(iter.next(), Some((Point3::new(1, 1, 1), &8)));
        assert_eq!(iter.next(), Some((Point3::new(1, 1, 0), &7)));
        assert_eq!(iter.next(), Some((Point3::new(1, 0, 1), &6)));
        assert_eq!(iter.next(), Some((Point3::new(1, 0, 0), &5)));
        assert_eq!(iter.next(), Some((Point3::new(0, 1, 1), &4)));
        assert_eq!(iter.next(), Some((Point3::new(0, 1, 0), &3)));
        assert_eq!(iter.next(), Some((Point3::new(0, 0, 1), &2)));
        assert_eq!(iter.next(), Some((Point3::new(0, 0, 0), &1)));
        assert_eq!(iter.next(), None);
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
