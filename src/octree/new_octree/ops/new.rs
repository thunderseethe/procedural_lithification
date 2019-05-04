use crate::octree::new_octree::{
    FieldType, HasData, HasPosition, Number, OctreeBase, OctreeLevel, OctreeTypes,
};

pub trait New: HasData + HasPosition {
    fn new(data: Self::Data, bottom_left: Self::Position) -> Self;
}
impl<E, N: Number> New for OctreeBase<E, N> {
    fn new(data: Self::Data, bottom_left: Self::Position) -> Self {
        OctreeBase { data, bottom_left }
    }
}
impl<O: OctreeTypes> New for OctreeLevel<O> {
    fn new(data: Self::Data, bottom_left: Self::Position) -> Self {
        OctreeLevel { data, bottom_left }
    }
}
