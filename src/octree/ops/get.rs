use crate::octree::descriptors::{ElementType, FieldType, HasPosition, Number};
use crate::octree::{Diameter, LevelData::*, OctreeBase, OctreeLevel, PositionOf};
use std::borrow::Borrow;

/// Retrieve an element from the Octree
pub trait Get: ElementType + HasPosition {
    fn get<P>(&self, pos: P) -> Option<&Self::Element>
    where
        P: Borrow<Self::Position>;
}
impl<O> Get for OctreeLevel<O>
where
    O: Get + Diameter + FieldType,
    Self: HasPosition<Position = PositionOf<O>>,
{
    #[inline]
    fn get<P>(&self, pos: P) -> Option<&Self::Element>
    where
        P: Borrow<PositionOf<Self>>,
    {
        match self.data {
            Empty => None,
            Leaf(ref elem) => Some(elem),
            Node(ref octants) => {
                let index: usize = self.get_octant_index(pos.borrow());
                octants[index].get(pos)
            }
        }
    }
}
impl<E, N> Get for OctreeBase<E, N>
where
    N: Number,
{
    #[inline]
    fn get<P>(&self, _pos: P) -> Option<&E>
    where
        P: Borrow<PositionOf<Self>>,
    {
        self.data.as_ref()
    }
}
