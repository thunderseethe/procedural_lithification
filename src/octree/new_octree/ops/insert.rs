use super::New;
use crate::octree::new_octree::*;
use amethyst::core::nalgebra::Point3;
use std::borrow::Borrow;
use std::rc::Rc;

pub trait Insert: OctreeTypes {
    fn insert<P, R>(&self, pos: P, elem: R) -> Self
    where
        P: Borrow<Point3<Self::Field>>,
        R: Into<Rc<Self::Element>>;
}
impl<O> Insert for OctreeLevel<O>
where
    O: Insert + New + Diameter + HasData + CreateSubNodes,
    <O as HasData>::Data: PartialEq,
    Self::Element: PartialEq,
{
    fn insert<P, R>(&self, pos: P, elem: R) -> Self
    where
        P: Borrow<Point3<Self::Field>>,
        R: Into<Rc<Self::Element>>,
    {
        self.place(pos, Some(elem.into()))
    }
}
impl<E, N> Insert for OctreeBase<E, N>
where
    N: Number,
{
    fn insert<P, R>(&self, pos: P, elem: R) -> Self
    where
        P: Borrow<Point3<N>>,
        R: Into<Rc<E>>,
    {
        OctreeBase::new(BaseData::leaf(elem.into()), pos.borrow().clone())
    }
}
