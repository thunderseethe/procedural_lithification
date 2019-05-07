use crate::octree::new_octree::*;
use amethyst::core::nalgebra::Scalar;
use itertools::Itertools;

pub trait Compress {
    fn compress_nodes(self) -> Self;
}
impl<O> Compress for OctreeLevel<O>
where
    O: HasData + OctreeTypes,
    ElementOf<O>: Clone,
    DataOf<O>: PartialEq,
    DataOf<Self>: From<DataOf<O>>,
{
    fn compress_nodes(self) -> Self {
        use crate::octree::new_octree::LevelData::*;
        match self.data {
            Node(nodes) => {
                let mut iter = nodes.iter().map(|node| node.data());
                if iter.all_equal() {
                    let head = nodes[0].into_data();
                    self.with_data(head.into())
                } else {
                    self
                }
            }
            _ => self,
        }
    }
}
impl<E, N: Scalar> Compress for OctreeBase<E, N> {
    // Compress is the identity function for BaseNodes
    fn compress_nodes(self) -> Self {
        self
    }
}
