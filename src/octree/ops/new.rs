use crate::octree::{FieldType, HasData, Number, OctreeBase, OctreeLevel, OctreeTypes};
use amethyst::core::nalgebra::Point3;

/// Trait for constructing an Octree
/// From an external users perspective this method might as well be in impl OctreeLevel<O> and impl OctreeBase<E, N> respectively.
/// However it is very useful to the recursive operations over the tree to be able to construct for example O::new(...) without having knowledge whether that will call OctreeLevel::new or OctreeBase::new
pub trait New: HasData + FieldType {
    fn new(data: Self::Data, bottom_left: Point3<Self::Field>) -> Self;
}
impl<E, N: Number> New for OctreeBase<E, N> {
    fn new(data: Self::Data, bottom_left: Point3<Self::Field>) -> Self {
        OctreeBase { data, bottom_left }
    }
}
impl<O: OctreeTypes> New for OctreeLevel<O> {
    fn new(data: Self::Data, bottom_left: Point3<Self::Field>) -> Self {
        OctreeLevel { data, bottom_left }
    }
}
