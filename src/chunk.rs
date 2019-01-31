use crate::octree::{octant_dimensions::*, octree_data::OctreeData, *};
use amethyst::core::nalgebra::Point3;
use rayon::iter::{plumbing::*, *};
use rayon::prelude::*;
use std::{borrow::Borrow, default::Default, sync::Arc};

pub type Block = u32;
pub static AIR_BLOCK: Block = 0;
pub static DIRT_BLOCK: Block = 1;

#[derive(Debug)]
pub struct Chunk {
    octree: Octree<Block>,
    // check that boxes are placed at their top right corner.
}

impl Default for Chunk {
    fn default() -> Self {
        // Default chunk size is 256 x 256 x 256
        Chunk::new(Octree::with_uniform_dimension(8))
    }
}

impl Chunk {
    pub fn new(octree: Octree<Block>) -> Self {
        Chunk { octree }
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
    tree: Octree<Block>,
}

impl ChunkBuilder {
    pub fn new() -> Self {
        ChunkBuilder {
            tree: gen_subtree(Point3::new(0, 0, 0), 8),
        }
    }

    fn with_tree(tree: Octree<Block>) -> Self {
        ChunkBuilder { tree }
    }

    pub fn build(mut self) -> Chunk {
        self.tree.compress();
        Chunk::new(self.tree)
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
                ChunkBuilder::with_tree(node.clone())
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
            parallel_drive_mut_node_children(self.tree.mut_children(), consumer, |node, cnsmr| {
                (IterMut {
                    tree: Arc::make_mut(node),
                })
                .into_par_iter()
                .drive_unindexed(cnsmr)
            })
        } else {
            consumer.into_folder().consume(self.tree).complete()
        }
    }
}

macro_rules! node_index {
    ($node:expr, $i:expr) => {
        unsafe { $node.ptr.add($i).as_mut().unwrap() }
    };
}

struct MultiThreadMutPtr<T> {
    ptr: *mut T,
}
unsafe impl<T> Send for MultiThreadMutPtr<T> {}
unsafe impl<T> Sync for MultiThreadMutPtr<T> {}

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
    let nodes_ptr = MultiThreadMutPtr {
        ptr: nodes.as_mut_ptr(),
    };
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
//impl<'data> ParallelIterator for &'data mut ChunkBuilder {
//    type Item = &'data mut Octree<Block>;
//
//    fn drive_unindexed<C>(self, consumer: C) -> C::Result
//    where
//        C: UnindexedConsumer<Self::Item>,
//    {
//        use crate::octree::{parallel_drive_node_children, OctreeData::Node};
//        match self.tree.data() {
//            Node(nodes) => parallel_drive_node_children(nodes, consumer, |node, cnsmr| {
//                let mut chunk_builder = ChunkBuilder::with_tree(node.clone());
//                chunk_builder.par_iter_mut().drive_unindexed(cnsmr)
//            }),
//            _ => consumer.into_folder().consume(&mut self.tree).complete(),
//        }
//    }
//}

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
            OctantDimensions::new(pos, Number::pow(2, height)),
            height,
        )
    }
}
#[cfg(test)]
mod test {
    use super::{Chunk, Point3};

    #[test]
    fn test_chunk_iterator() {
        let mut chunk = Chunk::default();
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
        let mut chunk = Chunk::default();
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
