use crate::octree::*;
use std::borrow::Borrow;

/// Delete an element from the Octree.
pub trait Delete: HasPosition {
    fn delete<P>(&self, pos: P) -> Self
    where
        P: Borrow<Self::Position>;
}

impl<O> Delete for OctreeLevel<O>
where
    O: OctreeTypes + HasData + New + Diameter + CreateSubNodes + Delete + Clone,
    ElementOf<O>: PartialEq + Clone,
    DataOf<O>: PartialEq + Clone,
    DataOf<Self>: From<DataOf<O>>,
    Self: HasPosition<Position = PositionOf<O>>,
    O: HasPosition<Position = Point3<FieldOf<O>>>,
{
    #[inline]
    fn delete<P>(&self, pos: P) -> Self
    where
        P: Borrow<PositionOf<Self>>,
    {
        use LevelData::*;
        match self.data {
            Empty => (*self).clone(),
            Leaf(ref elem) => self.create_sub_nodes(pos, None, O::Data::leaf(elem.clone())),
            Node(ref old_nodes) => {
                let mut nodes = old_nodes.clone();
                let index: usize = self.get_octant_index(pos.borrow());
                let old_octant = &old_nodes[index];
                nodes[index] = Ref::new(old_octant.delete(pos));
                self.with_data(Node(nodes)).compress_nodes()
            }
        }
    }
}

impl<E: Clone, N: Number> Delete for OctreeBase<E, N> {
    #[inline]
    fn delete<P>(&self, _pos: P) -> Self
    where
        P: Borrow<Point3<N>>,
    {
        self.with_data(None)
    }
}
