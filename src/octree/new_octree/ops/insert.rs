use super::New;
use crate::octree::new_octree::*;
use amethyst::core::nalgebra::Point3;
use std::borrow::Borrow;

pub trait Insert: ElementType + HasPosition {
    fn insert<P, R>(&self, pos: P, elem: R) -> Self
    where
        P: Borrow<Self::Position>,
        R: Into<Self::Element>;
}
impl<O> Insert for OctreeLevel<O>
where
    O: Insert + New + CreateSubNodes + Diameter + HasData + FieldType,
    ElementOf<O>: PartialEq + Clone,
    DataOf<O>: PartialEq + Clone,
    DataOf<Self>: From<DataOf<O>>,
    Self: HasPosition<Position = PositionOf<O>>,
    O: HasPosition<Position = Point3<FieldOf<O>>>,
{
    fn insert<P, R>(&self, pos: P, elem: R) -> Self
    where
        P: Borrow<PositionOf<Self>>,
        R: Into<ElementOf<O>>,
    {
        if self.outside_bounds(pos.borrow()) {
            panic!("Position out of bounds");
        } else {
            self.place(pos, Some(elem.into()))
        }
    }
}
impl<E, N> Insert for OctreeBase<E, N>
where
    N: Number,
{
    fn insert<P, R>(&self, pos: P, elem: R) -> Self
    where
        P: Borrow<Point3<N>>,
        R: Into<E>,
    {
        OctreeBase::new(
            <Self as HasData>::Data::leaf(elem.into()),
            pos.borrow().clone(),
        )
    }
}
