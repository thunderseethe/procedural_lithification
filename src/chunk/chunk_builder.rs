use crate::chunk::{block::Block, parallel_drive_mut_node_children, Chunk};
use crate::mut_ptr::MultiThreadMutPtr;
use crate::octree::{
    octant_dimensions::OctantDimensions,
    octree_data::OctreeData,
    {Number, Octree},
};
use amethyst::core::nalgebra::Point3;
use rayon::iter::{plumbing::*, *};
use std::sync::Arc;
use toolshed::Arena;

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

#[derive(Copy, Clone)]
struct RawLeaf([Option<Block>; 8]);
impl Default for RawLeaf {
    fn default() -> Self {
        RawLeaf([None; 8])
    }
}
impl<'a> IntoParallelIterator for &'a mut RawLeaf {
    type Item = &'a mut Option<Block>;
    type Iter = RawLeafIterMut<'a>;

    fn into_par_iter(self) -> Self::Iter {
        RawLeafIterMut { slice: &mut self.0 }
    }
}

struct RawLeafIterMut<'data> {
    slice: &'data mut [Option<Block>],
}
impl<'a> ParallelIterator for RawLeafIterMut<'a> {
    type Item = &'a mut Option<Block>;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        bridge(self, consumer)
    }
}
impl<'a> IndexedParallelIterator for RawLeafIterMut<'a> {
    fn len(&self) -> usize {
        8
    }

    fn drive<C>(self, consumer: C) -> C::Result
    where
        C: Consumer<Self::Item>,
    {
        bridge(self, consumer)
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        callback.callback(RawLeafProducer { slice: self.slice })
    }
}

struct RawLeafProducer<'data, T> {
    slice: &'data mut [T],
}
impl<'data, T: Send + Sync> Producer for RawLeafProducer<'data, T> {
    type Item = &'data mut T;
    type IntoIter = std::slice::IterMut<'data, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.slice.iter_mut()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.slice.split_at_mut(index);
        (
            RawLeafProducer { slice: left },
            RawLeafProducer { slice: right },
        )
    }
}

#[derive(Copy, Clone)]
struct RawNode<T>([T; 8]);
impl<T: Default + Copy> Default for RawNode<T> {
    fn default() -> Self {
        RawNode([T::default(); 8])
    }
}
impl<'a, T: Send + Sync> IntoParallelIterator for &'a mut RawNode<T>
where
    &'a mut T: IntoParallelIterator,
{
    type Item = <&'a mut T as IntoParallelIterator>::Item;
    type Iter = RawNodeIterMut<'a, T>;

    fn into_par_iter(self) -> Self::Iter {
        RawNodeIterMut {
            slice: &mut self.0[..],
        }
    }
}

struct RawNodeIterMut<'data, T> {
    slice: &'data mut [T],
}

macro_rules! split_consumer {
    ($nodes: ident, $consumer: ident, @split 1) => {{
        let (left, right, reducer) = $consumer.split_at(1);
        let left_res = unsafe { $nodes.0.as_mut().unwrap().into_par_iter().drive_unindexed(left) };
        let right_res = unsafe { $nodes.offset(1).0.as_mut().unwrap().into_par_iter().drive_unindexed(right) };
        reducer.reduce(left_res, right_res)
    }};
    ($nodes: ident, $consumer: ident, @split 2) => {{
        let (left, right, reducer) = $consumer.split_at(2);
        let (left_res, right_res) = rayon::join(
            || {
                split_consumer!($nodes, left, @split 1)
            },
            || {
                let nodes = unsafe { $nodes.offset(2) };
                split_consumer!(nodes, right, @split 1)
            }
        );
        reducer.reduce(left_res, right_res)
    }};
    ($nodes: ident, $consumer: ident, @split 4) => {{
        let (left, right, reducer) = $consumer.split_at(4);
        let (left_res, right_res) = rayon::join(
            || {
                split_consumer!($nodes, left, @split 2)
            },
            || {
                let nodes = unsafe { $nodes.offset(4) };
                split_consumer!(nodes, right, @split 2)
            }
        );
        reducer.reduce(left_res, right_res)
    }};
    ($nodes:ident, $consumer: ident) => {{
        split_consumer!($nodes, $consumer, @split 4)
    }};
}

impl<'a, T: Send + Sync> ParallelIterator for RawNodeIterMut<'a, T>
where
    &'a mut T: IntoParallelIterator,
{
    type Item = <&'a mut T as IntoParallelIterator>::Item;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        let nodes = MultiThreadMutPtr::new(self.slice.as_mut_ptr());
        split_consumer!(nodes, consumer)
    }
}

struct RawTree(RawNode<RawNode<RawNode<RawNode<RawNode<RawNode<RawNode<RawLeaf>>>>>>>);
impl RawTree {
    pub fn new() -> Self {
        RawTree(RawNode::default())
    }
}

pub struct ChunkBuilder2 {
    pos: Point3<i32>,
    tree: RawTree,
}

impl ChunkBuilder2 {
    pub fn new(pos: Point3<i32>) -> Self {
        ChunkBuilder2 {
            pos,
            tree: RawTree::new(),
        }
    }

    pub fn par_iter_mut<'a>(&'a mut self) -> impl ParallelIterator<Item = &'a mut Option<Block>> {
        self.tree.0.into_par_iter()
    }
}
