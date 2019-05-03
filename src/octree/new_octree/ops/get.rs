use crate::octree::new_octree::descriptors::{Number, OctreeTypes};
use crate::octree::new_octree::{Diameter, OctreeBase, OctreeLevel};
use amethyst::core::nalgebra::Point3;
use std::borrow::Borrow;

pub trait Get: OctreeTypes {
    fn get<P>(&self, pos: P) -> Option<&Self::Element>
    where
        P: Borrow<Point3<Self::Field>>;
}
impl<O> Get for OctreeLevel<O>
where
    O: Get + Diameter,
{
    fn get<P>(&self, pos: P) -> Option<&Self::Element>
    where
        P: Borrow<Point3<Self::Field>>,
    {
        use crate::octree::new_octree::LevelData::*;
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
    fn get<P>(&self, _pos: P) -> Option<&E>
    where
        P: Borrow<Point3<N>>,
    {
        self.data.as_ref()
    }
}
