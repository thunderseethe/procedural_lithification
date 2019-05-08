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
    DataOf<O>: Clone + PartialEq,
    DataOf<Self>: From<DataOf<O>>,
{
    /// Checks the children of OctreeLevel and combines them into a Leaf or Empty node if they are all equal
    #[inline]
    fn compress_nodes(self) -> Self {
        use crate::octree::new_octree::LevelData::*;
        match &self.data {
            Node(ref nodes) => {
                if { nodes.iter().map(|node| node.data()).all_equal() } {
                    let head: DataOf<O> = nodes[0].data().clone();
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
    /// compress_nodes() is the identity function for OctreeBase
    #[inline]
    fn compress_nodes(self) -> Self {
        self
    }
}
