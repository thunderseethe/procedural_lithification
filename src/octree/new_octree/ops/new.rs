use crate::octree::new_octree::{
    BaseData, Diameter, FieldType, HasData, LevelData, Number, OctreeBase, OctreeLevel, OctreeTypes,
};
use amethyst::core::nalgebra::{Point3, Scalar};

pub trait New: HasData + FieldType {
    fn new(data: Self::Data, bottom_left: Point3<Self::Field>) -> Self;
}
impl<E, N: Number> New for OctreeBase<E, N> {
    fn new(data: Self::Data, bottom_left: Point3<N>) -> Self {
        OctreeBase { data, bottom_left }
    }
}
impl<O: OctreeTypes> New for OctreeLevel<O> {
    fn new(data: Self::Data, bottom_left: Point3<Self::Field>) -> Self {
        OctreeLevel { data, bottom_left }
    }
}
