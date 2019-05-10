use crate::octree::octant::Octant;
use crate::octree::*;
use rayon::iter::plumbing::*;
use rayon::prelude::*;

unsafe impl<O> Send for OctreeLevel<O> where O: OctreeTypes + Send {}
unsafe impl<O> Sync for OctreeLevel<O> where O: OctreeTypes + Sync {}
impl<O> IntoParallelIterator for OctreeLevel<O>
where
    O: OctreeTypes
        + Diameter
        + Clone
        + Send
        + Sync
        + IntoParallelIterator<Item = Octant<ElementOf<O>, Point3<FieldOf<O>>>>,
    ElementOf<O>: Clone + Send + Sync,
    FieldOf<O>: Send + Sync,
{
    type Iter = ParallelOctreeLevelIter<O>;
    type Item = <<O as IntoParallelIterator>::Iter as ParallelIterator>::Item;

    fn into_par_iter(self) -> Self::Iter {
        ParallelOctreeLevelIter { node: self }
    }
}

pub struct ParallelOctreeLevelIter<O: OctreeTypes> {
    node: OctreeLevel<O>,
}
impl<O> ParallelIterator for ParallelOctreeLevelIter<O>
where
    O: OctreeTypes
        + Diameter
        + Clone
        + Send
        + Sync
        + IntoParallelIterator<Item = Octant<ElementOf<O>, Point3<FieldOf<O>>>>,
    ElementOf<O>: Clone + Send + Sync,
    FieldOf<O>: Send + Sync,
{
    type Item = <<O as IntoParallelIterator>::Iter as ParallelIterator>::Item;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        use LevelData::*;
        match self.node.data {
            Empty => consumer.into_folder().complete(),
            Leaf(ref elem) => consumer
                .into_folder()
                .consume(Octant::new(
                    elem.clone(),
                    self.node.bottom_left,
                    self.node.get_diameter(),
                ))
                .complete(),
            Node(ref nodes) => parallel_drive_node_children(nodes, consumer, |node, consumer| {
                node.clone().into_par_iter().drive_unindexed(consumer)
            }),
        }
    }
}

unsafe impl<E, N: Number> Send for OctreeBase<E, N> {}
unsafe impl<E, N: Number> Sync for OctreeBase<E, N> {}
impl<E: Send + Sync, N: Number> IntoParallelIterator for OctreeBase<E, N>
where
    E: Send + Sync,
    N: Number + Send + Sync,
{
    type Item = Octant<E, Point3<N>>;
    type Iter = rayon::option::IntoIter<Self::Item>;

    fn into_par_iter(self) -> Self::Iter {
        let p = self.bottom_left;
        self.data
            .map(|elem| Octant::new(elem, p, Self::DIAMETER))
            .into_par_iter()
    }
}

fn parallel_drive_node_children<'a, ITEM, O, C, F>(
    nodes: &'a [Ref<O>; 8],
    consumer: C,
    handle_child: F,
) -> C::Result
where
    O: Send + Sync,
    C: UnindexedConsumer<ITEM>,
    F: Fn(&'a O, C) -> C::Result + Send + Sync,
{
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
                        handle_child(&nodes[0], lll_octet),
                        handle_child(&nodes[1], llr_octet),
                    )
                },
                || {
                    let r = lrl_octet.to_reducer();
                    r.reduce(
                        handle_child(&nodes[2], lrl_octet),
                        handle_child(&nodes[3], lrr_octet),
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
                        handle_child(&nodes[4], rll_octet),
                        handle_child(&nodes[5], rlr_octet),
                    )
                },
                || {
                    let r = rrl_octet.to_reducer();
                    r.reduce(
                        handle_child(&nodes[6], rrl_octet),
                        handle_child(&nodes[7], rrr_octet),
                    )
                },
            );
            reducer.reduce(left, right)
        },
    );
    reducer.reduce(left, right)
}
