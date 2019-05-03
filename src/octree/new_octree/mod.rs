/// Module contains two structs, OctreeBase and OctreeLevel.
/// These can be combined to form an Octree of a static height.
/// For example an Octree of height 3 would have type OctreeLevel<OctreeLevel<OctreeBase<E, N>>>.
/// This relatively verbose but allows the rust compiler to optimize our Trees recursive methods much better than more traditional unbounded recursion.
/// A lof of the boilerplat can be alleviated by the use of type aliases.
use super::octant::{Octant, OctantId};
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
type Ref<T> = Arc<T>;

pub type DataOf<T> = <T as HasData>::Data;
pub type ElementOf<T> = <T as ElementType>::Element;
pub type FieldOf<T> = <T as FieldType>::Field;

/// Data for a single level of an Octree.
pub enum LevelData<O>
where
    O: OctreeTypes,
{
    Node([Ref<O>; 8]),
    Leaf(O::Element),
    Empty,
}
impl<O> fmt::Debug for LevelData<O>
where
    O: OctreeTypes + fmt::Debug,
    ElementOf<O>: fmt::Debug,
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
    ElementOf<O>: Clone,
{
    fn clone(&self) -> Self {
        use LevelData::*;
        match self {
            Node(ref nodes) => Node(nodes.clone()),
            Leaf(e) => Leaf(e.clone()),
            Empty => Empty,
        }
    }
}
impl<O> PartialEq for LevelData<O>
where
    O: OctreeTypes + PartialEq,
    ElementOf<O>: PartialEq,
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
    ElementOf<O>: fmt::Debug,
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
    ElementOf<O>: PartialEq,
{
    fn eq(&self, other: &OctreeLevel<O>) -> bool {
        self.bottom_left.eq(&other.bottom_left) && self.data.eq(&other.data)
    }
}
impl<O> Clone for OctreeLevel<O>
where
    O: OctreeTypes + Clone,
    ElementOf<O>: Clone,
{
    fn clone(&self) -> Self {
        OctreeLevel::new(self.data.clone(), self.bottom_left.clone())
    }
}

/// Base of the Octree. This level can only contain Leaf nodes
#[derive(PartialEq, Debug)]
pub struct OctreeBase<E, N: Scalar> {
    data: Option<E>,
    bottom_left: Point3<N>,
}
impl<E: Clone, N: Number> Clone for OctreeBase<E, N> {
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

    fn get_octant<P>(&self, pos_ref: P) -> OctantId
    where
        P: Borrow<<Self as HasPosition>::Position>,
    {
        use crate::octree::octant::OctantId::*;
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
    fn with_data(&self, data: DataOf<Self>) -> Self {
        OctreeLevel {
            data: data,
            ..(*self.clone())
        }
    }

    pub fn root_point(&self) -> &<Self as HasPosition>::Position {
        &self.bottom_left
    }

    pub fn data(&self) -> &DataOf<Self> {
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
            Leaf(elem) => leaf_fn(&elem),
            Node(ref nodes) => node_fn(nodes),
        }
    }
}
impl<E: Clone, N: Number> OctreeBase<E, N> {
    fn with_data(&self, data: DataOf<Self>) -> Self {
        OctreeBase {
            data: data,
            ..(*self).clone()
        }
    }
    pub fn get_diameter(&self) -> usize {
        Self::diameter()
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

    #[test]
    fn octree_delete_is_idempotent() {
        let p = Point3::new(1, 1, 1);
        let octree: Octree8<i32, u8> =
            Octree8::new(LevelData::Empty, Point3::origin()).insert(&p, 1234);

        let result = octree.delete(&p).delete(&p);
        assert_eq!(result.get(&p), None);
    }

    #[test]
    fn octree_iterator_length_is_correct() {
        let octree: Octree8<i32, u8> = OctreeLevel::new(LevelData::Empty, Point3::origin())
            .insert(Point3::new(2, 2, 2), 1234)
            .insert(Point3::new(1, 1, 2), 4567)
            .insert(Point3::new(2, 1, 1), 7890);

        let oct_ref = &octree;
        assert_eq!(oct_ref.into_iter().count(), 3);
    }

    #[test]
    fn octree_iterator_contains_correct_elements() {
        use std::collections::HashSet;

        let points = vec![
            Point3::new(2, 2, 2),
            Point3::new(2, 4, 2),
            Point3::new(4, 4, 4),
            Point3::new(2, 2, 4),
        ];

        let octree = Octree8::new(LevelData::Empty, Point3::origin())
            .insert(points[0], 1)
            .insert(points[1], 2)
            .insert(points[2], 3)
            .insert(points[3], 4);

        let mut expected = HashSet::new();
        expected.insert(Octant::new(&1, &points[0], 1));
        expected.insert(Octant::new(&2, &points[1], 1));
        expected.insert(Octant::new(&3, &points[2], 1));
        expected.insert(Octant::new(&4, &points[3], 1));

        for octant in &octree {
            assert!(expected.contains(&octant));
        }
    }

    #[test]
    fn octree_insertion_compresses_common_nodes_in_subtree() {
        let octree = Octree8::new(LevelData::Empty, Point3::origin())
            .insert(Point3::new(1, 1, 1), 1234)
            .insert(Point3::new(1, 1, 0), 1234)
            .insert(Point3::new(1, 0, 1), 1234)
            .insert(Point3::new(0, 1, 0), 1234)
            .insert(Point3::new(0, 1, 1), 1234)
            .insert(Point3::new(1, 0, 0), 1234)
            .insert(Point3::new(0, 0, 1), 1234)
            .insert(Point3::new(0, 0, 0), 1234);

        let mut iter = (&octree).into_iter();
        assert_eq!(iter.next(), Some(Octant::new(&1234, &Point3::origin(), 2)));
    }

}
