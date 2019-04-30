/// Module contains two structs, OctreeBase and OctreeLevel.
/// These can be combined to form an Octree of a static height.
/// For example an Octree of height 3 would have type OctreeLevel<OctreeLevel<OctreeBase<E, N>>>.
/// This relatively verbose but allows the rust compiler to optimize our Trees recursive methods much better than more traditional unbounded recursion.
/// A lof of the boilerplat can be alleviated by the use of type aliases.
use super::octant::Octant;
use amethyst::core::nalgebra::{Point3, Scalar};
use num_traits::*;
use std::borrow::Borrow;
use std::rc::Rc;

mod ops;
pub use ops::*;

pub mod descriptors;
use descriptors::*;

pub mod consts;

/// Data for a single level of an Octree.
pub enum LevelData<O>
where
    O: OctreeTypes,
{
    Node([Rc<O>; 8]),
    Leaf(Rc<O::Element>),
    Empty,
}
impl<O> Clone for LevelData<O>
where
    O: OctreeTypes,
{
    fn clone(&self) -> Self {
        use LevelData::*;
        match self {
            Node(ref nodes) => Node(nodes.clone()),
            Leaf(e) => Leaf(Rc::clone(e)),
            Empty => Empty,
        }
    }
}
impl<O> PartialEq for LevelData<O>
where
    O: OctreeTypes + PartialEq,
    <O as ElementType>::Element: PartialEq,
{
    fn eq(&self, other: &LevelData<O>) -> bool {
        use LevelData::*;
        match (self, other) {
            (Node(node_a), Node(node_b)) => node_a == node_b,
            (Leaf(elem_a), Leaf(elem_b)) => elem_a == elem_b,
            (Empty, Empty) => true,
            _ => false,
        }
    }
}

/// Node struct for level of an Octree.
pub struct OctreeLevel<O>
where
    O: OctreeTypes,
{
    data: LevelData<O>,
    bottom_left: Point3<O::Field>,
}
impl<O> PartialEq for OctreeLevel<O>
where
    O: OctreeTypes + PartialEq,
    <O as ElementType>::Element: PartialEq,
{
    fn eq(&self, other: &OctreeLevel<O>) -> bool {
        self.bottom_left.eq(&other.bottom_left) && self.data.eq(&other.data)
    }
}
impl<O> Clone for OctreeLevel<O>
where
    O: OctreeTypes + Clone,
{
    fn clone(&self) -> Self {
        OctreeLevel::new(self.data.clone(), self.bottom_left.clone())
    }
}

/// A leaf node of an Octree. It can either contain a value E or not and is isomorphic to Option<Rc<E>>.
#[derive(PartialEq)]
pub enum BaseData<E> {
    Leaf(Rc<E>),
    Empty,
}
impl<E> Clone for BaseData<E> {
    fn clone(&self) -> Self {
        use BaseData::*;
        match self {
            Leaf(e) => Leaf(Rc::clone(e)),
            Empty => Empty,
        }
    }
}
#[derive(PartialEq)]
pub struct OctreeBase<E, N: Scalar> {
    data: BaseData<E>,
    bottom_left: Point3<N>,
}
impl<E, N: Number> Clone for OctreeBase<E, N> {
    fn clone(&self) -> Self {
        OctreeBase::new(self.data.clone(), self.bottom_left.clone())
    }
}

pub trait Diameter: FieldType {
    fn diameter() -> Self::Field;
}
impl<O> Diameter for OctreeLevel<O>
where
    O: Diameter + OctreeTypes,
{
    fn diameter() -> Self::Field {
        O::diameter() << Self::Field::one()
    }
}
impl<E, N> Diameter for OctreeBase<E, N>
where
    N: Number,
{
    fn diameter() -> N {
        N::one()
    }
}

pub trait HasPosition {
    type Position;

    fn position(&self) -> &Self::Position;
}
impl<O> HasPosition for OctreeLevel<O>
where
    O: OctreeTypes,
{
    type Position = Point3<<Self as FieldType>::Field>;

    fn position(&self) -> &Self::Position {
        &self.bottom_left
    }
}
impl<E, N: Scalar> HasPosition for OctreeBase<E, N> {
    type Position = Point3<N>;

    fn position(&self) -> &Self::Position {
        &self.bottom_left
    }
}

pub trait HasData: ElementType {
    type Data: Clone + Leaf<Rc<Self::Element>> + Empty;
    fn data(&self) -> &Self::Data;
}
impl<O> HasData for OctreeLevel<O>
where
    O: OctreeTypes,
{
    type Data = LevelData<O>;

    fn data(&self) -> &Self::Data {
        &self.data
    }
}
impl<E, N: Scalar> HasData for OctreeBase<E, N> {
    type Data = BaseData<E>;

    fn data(&self) -> &Self::Data {
        &self.data
    }
}

impl<O> OctreeLevel<O>
where
    O: Diameter + OctreeTypes,
{
    fn get_octant_index<P>(&self, pos: P) -> usize
    where
        P: Borrow<<Self as HasPosition>::Position>,
    {
        self.get_octant(pos).to_usize().unwrap()
    }

    fn get_octant<P>(&self, pos_ref: P) -> Octant
    where
        P: Borrow<<Self as HasPosition>::Position>,
    {
        use crate::octree::octant::Octant::*;
        let pos = pos_ref.borrow();
        let r = Self::diameter() >> <Self as FieldType>::Field::one();
        match (
            pos.x >= self.bottom_left.x + r,
            pos.y >= self.bottom_left.y + r,
            pos.z >= self.bottom_left.z + r,
        ) {
            (true, true, true) => HighHighHigh,
            (true, true, false) => HighHighLow,
            (true, false, true) => HighLowHigh,
            (true, false, false) => HighLowLow,
            (false, true, true) => LowHighHigh,
            (false, true, false) => LowHighLow,
            (false, false, true) => LowLowHigh,
            (false, false, false) => LowLowLow,
        }
    }
}
impl<O: OctreeTypes> OctreeLevel<O> {
    fn with_data(&self, data: LevelData<O>) -> Self {
        OctreeLevel {
            data: data,
            ..(*self.clone())
        }
    }
}
//impl<O> OctreeLevel<O>
//where
//    O: Insert + New + Diameter + HasData,
//    <O as HasData>::Data: PartialEq,
//{
//    fn create_sub_nodes<P>(
//        &self,
//        pos: P,
//        elem: Rc<<Self as ElementType>::Element>,
//        default: O::Data,
//    ) -> Self
//    where
//        P: Borrow<Point3<<Self as FieldType>::Field>>,
//    {
//        use crate::octree::octant::OctantIter;
//        use LevelData::Node;
//        let modified_octant = self.get_octant(pos.borrow());
//        let octree_nodes: [Rc<O>; 8] = array_init::from_iter(OctantIter::default().map(|octant| {
//            let data = default.clone();
//            let sub_bottom_left = octant.sub_octant_bottom_left(self.bottom_left, O::diameter());
//            let octree = O::new(data, sub_bottom_left);
//            let octree = if modified_octant == octant {
//                octree.insert(pos.borrow(), elem.clone())
//            } else {
//                octree
//            };
//            Rc::new(octree)
//        }))
//        .expect("Failed to construct array from iterator");
//        self.with_data(Node(octree_nodes)).compress_nodes()
//    }
//}
