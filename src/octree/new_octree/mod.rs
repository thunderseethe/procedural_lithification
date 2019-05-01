/// Module contains two structs, OctreeBase and OctreeLevel.
/// These can be combined to form an Octree of a static height.
/// For example an Octree of height 3 would have type OctreeLevel<OctreeLevel<OctreeBase<E, N>>>.
/// This relatively verbose but allows the rust compiler to optimize our Trees recursive methods much better than more traditional unbounded recursion.
/// A lof of the boilerplat can be alleviated by the use of type aliases.
use super::octant::Octant;
use super::octant_dimensions::OctantDimensions;
use amethyst::core::nalgebra::{Point3, Scalar};
use num_traits::*;
use std::borrow::Borrow;
use std::fmt;
use std::rc::Rc;
use std::sync::Arc;

mod ops;
pub use ops::*;

pub mod descriptors;
pub use descriptors::*;

pub mod consts;
pub use consts::{Octree, Octree8};

/// Poor man's higher kinded types.
/// Used to toggle the implementation between Ref and Arc;
type Ref<T> = Rc<T>;

/// Data for a single level of an Octree.
pub enum LevelData<O>
where
    O: OctreeTypes,
{
    Node([Ref<O>; 8]),
    Leaf(Ref<O::Element>),
    Empty,
}
impl<O> fmt::Debug for LevelData<O>
where
    O: OctreeTypes + fmt::Debug,
    O::Element: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        use LevelData::*;
        match self {
            Node(nodes) => write!(f, "Node({:?})", nodes),
            Leaf(elem) => write!(f, "Leaf({:?})", elem),
            Empty => write!(f, "Empty"),
        }
    }
}
impl<O> Clone for LevelData<O>
where
    O: OctreeTypes,
{
    fn clone(&self) -> Self {
        use LevelData::*;
        match self {
            Node(ref nodes) => Node(nodes.clone()),
            Leaf(e) => Leaf(Ref::clone(e)),
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
impl<O> fmt::Debug for OctreeLevel<O>
where
    O: OctreeTypes + fmt::Debug,
    O::Element: fmt::Debug,
{
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_struct("OctreeLevel")
            .field("data", &self.data)
            .field("bottom_left", &self.bottom_left)
            .finish()
    }
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

/// A leaf node of an Octree. It can either contain a value E or not and is isomorphic to Option<Ref<E>>.
#[derive(PartialEq, Debug)]
pub enum BaseData<E> {
    Leaf(Ref<E>),
    Empty,
}
impl<E> Clone for BaseData<E> {
    fn clone(&self) -> Self {
        use BaseData::*;
        match self {
            Leaf(e) => Leaf(Ref::clone(e)),
            Empty => Empty,
        }
    }
}
impl<E> BaseData<E> {
    fn as_option(&self) -> Option<&Ref<E>> {
        use BaseData::*;
        match self {
            Leaf(ref elem) => Some(elem),
            Empty => None,
        }
    }
}
#[derive(PartialEq, Debug)]
pub struct OctreeBase<E, N: Scalar> {
    data: BaseData<E>,
    bottom_left: Point3<N>,
}
impl<E, N: Number> Clone for OctreeBase<E, N> {
    fn clone(&self) -> Self {
        OctreeBase::new(self.data.clone(), self.bottom_left.clone())
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
        let r = num_traits::NumCast::from(Self::diameter() >> 1).unwrap();
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
impl<O> OctreeLevel<O>
where
    O: OctreeTypes + Diameter,
{
    /// Convenience method to access diameter from an instance of type
    pub fn get_diameter(&self) -> usize {
        Self::diameter()
    }

    fn outside_bounds<P>(&self, pos_ref: P) -> bool
    where
        P: Borrow<<Self as HasPosition>::Position>,
    {
        let pos = pos_ref.borrow();
        let diameter: usize = <Self as Diameter>::diameter();
        pos.x.as_() > self.bottom_left.x.as_() + diameter
            || pos.x < self.bottom_left.x
            || pos.y.as_() > self.bottom_left.y.as_() + diameter
            || pos.y < self.bottom_left.y
            || pos.z.as_() > self.bottom_left.z.as_() + diameter
            || pos.z < self.bottom_left.z
    }
}
// This is the least restrictive impl for our OctreeLevel so most of our helper methods live here
impl<O: OctreeTypes> OctreeLevel<O> {
    fn with_data(&self, data: <Self as HasData>::Data) -> Self {
        OctreeLevel {
            data: data,
            ..(*self.clone())
        }
    }

    pub fn root_point(&self) -> &<Self as HasPosition>::Position {
        &self.bottom_left
    }

    pub fn data(&self) -> &<Self as HasData>::Data {
        &self.data
    }

    pub fn is_empty(&self) -> bool {
        self.data.is_empty()
    }

    pub fn is_node(&self) -> bool {
        use LevelData::*;
        match self.data {
            Node(_) => true,
            _ => false,
        }
    }

    pub fn map<EFn, LFn, NFn, Output>(&self, empty_fn: EFn, leaf_fn: LFn, node_fn: NFn) -> Output
    where
        EFn: FnOnce() -> Output,
        LFn: FnOnce(&<Self as ElementType>::Element) -> Output,
        NFn: FnOnce(&[Ref<O>; 8]) -> Output,
    {
        use LevelData::*;
        match &self.data {
            Empty => empty_fn(),
            Leaf(elem) => leaf_fn(elem.as_ref()),
            Node(ref nodes) => node_fn(nodes),
        }
    }
}
impl<E, N: Number> OctreeBase<E, N> {
    fn with_data(&self, data: <Self as HasData>::Data) -> Self {
        OctreeBase {
            data: data,
            ..(*self).clone()
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn octree_new_constructs_expected_tree() {
        use typenum::*;
        let octree: OctreeLevel<
            OctreeLevel<
                OctreeLevel<
                    OctreeLevel<
                        OctreeLevel<OctreeLevel<OctreeLevel<OctreeLevel<OctreeBase<u32, u8>>>>>,
                    >,
                >,
            >,
        > = Octree::<u32, u8, U256>::new(LevelData::Empty, Point3::origin());

        assert_eq!(octree.get_diameter(), 256);
        assert_eq!(
            octree,
            OctreeLevel {
                data: LevelData::Empty,
                bottom_left: Point3::origin(),
            }
        );
    }

    #[test]
    fn octree_insert_handles_center_point() {
        let octree: Octree8<i32, u8> = Octree8::new(LevelData::Empty, Point3::origin());

        assert_eq!(
            octree.insert(Point3::origin(), 1234).get(Point3::origin()),
            Some(&1234)
        );
    }

    #[test]
    fn octree_element_retrieved_after_insertion_in_same_octants() {
        let p1 = Point3::new(2, 2, 2);
        let p2 = Point3::new(1, 1, 1);
        let octree: Octree8<i32, u8> = Octree8::new(LevelData::Empty, Point3::origin())
            .insert(&p1, 1234)
            .insert(&p2, 5678);

        assert_eq!(octree.get(&p1), Some(&1234));
        assert_eq!(octree.get(&p2), Some(&5678));
    }

    #[test]
    fn octree_element_retrieved_after_inserterion_in_diff_octants() {
        let p1 = Point3::new(1, 1, 1);
        let p2 = Point3::new(7, 7, 7);
        let octree: Octree8<i32, u8> = Octree8::new(LevelData::Empty, Point3::origin())
            .insert(&p1, 1234)
            .insert(&p2, 5678);

        assert_eq!(octree.get(&p1), Some(&1234));
        assert_eq!(octree.get(&p2), Some(&5678));
    }

    #[test]
    fn octree_insert_updates_element() {
        let p = Point3::new(1, 1, 1);
        let octree: Octree8<i32, u8> =
            Octree8::new(LevelData::Empty, Point3::origin()).insert(&p, 1234);

        assert_eq!(octree.get(&p), Some(&1234));

        let octree = octree.insert(&p, 5678);
        assert_eq!(octree.get(&p), Some(&5678));
    }

    #[test]
    fn octree_deletes_expected_element() {
        let p = Point3::new(4, 1, 1);
        let octree: Octree8<i32, u8> = OctreeLevel::new(LevelData::Empty, Point3::origin())
            .insert(Point3::new(1, 1, 1), 1234)
            .insert(Point3::new(1, 1, 2), 4567)
            .insert(&p, 7890);

        assert_eq!(octree.get(&p), Some(&7890));
        let octree = octree.delete(&p);
        assert_eq!(octree.get(&p), None);
    }
}
