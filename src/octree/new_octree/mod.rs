/// Module contains two structs, OctreeBase and OctreeLevel.
/// These can be combined recursively to form an Octree of a static height.
/// For example an Octree of height 3 would have type OctreeLevel<OctreeLevel<OctreeBase<E, N>>>.
/// This is relatively verbose but allows the rust compiler to optimize our octrees recursive methods better than general unbounded recursion.
use super::octant::{Octant, OctantId};
use super::octant_dimensions::OctantDimensions;
use amethyst::core::nalgebra::{Point3, Scalar};
use num_traits::*;
use serde::ser::SerializeStruct;
use serde::{Deserialize, Deserializer, Serialize, Serializer};
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

pub mod builder;

/// Poor man's higher kinded types.
/// Used to toggle the implementation between Ref and Arc;
type Ref<T> = Arc<T>;

pub type DataOf<T> = <T as HasData>::Data;
pub type PositionOf<T> = <T as HasPosition>::Position;
pub type ElementOf<T> = <T as ElementType>::Element;
pub type FieldOf<T> = <T as FieldType>::Field;

/// Composite trait to describe the full functionality of an Octree
/// This trait exists mostly for convenience when parametizing over an Octree
pub trait OctreeLike: New + Insert + Delete + Get + HasPosition + Diameter + OctreeTypes {}
impl<'a, T: 'a> OctreeLike for T
where
    T: New + Insert + Delete + Get + HasPosition + Diameter + OctreeTypes,
    &'a T: IntoIterator,
{
}

/// Data for a single non-temrminal level of an Octree.
#[derive(Deserialize, Serialize)]
pub enum LevelData<O>
where
    O: OctreeTypes,
{
    Node([Ref<O>; 8]),
    Leaf(O::Element),
    Empty,
}

// Rustc does not handle deriving traits for Associated types well so we much implement some foundation traits ourself.
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
impl<O> Eq for LevelData<O>
where
    O: OctreeTypes + Eq,
    ElementOf<O>: Eq,
{
}

/// Node struct for a level of an Octree.
pub struct OctreeLevel<O>
where
    O: OctreeTypes,
{
    data: LevelData<O>,
    /// The root point of this octree which will be used with diameter to determine position of each suboctant
    bottom_left: Point3<O::Field>,
}
impl<O> Serialize for OctreeLevel<O>
where
    O: OctreeTypes + Serialize,
    ElementOf<O>: Serialize,
    FieldOf<O>: Serialize,
{
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut octree = serializer.serialize_struct("OctreeLevel", 2)?;
        octree.serialize_field("data", &self.data)?;
        octree.serialize_field("bottom_left", &self.bottom_left)?;
        octree.end()
    }
}
impl<'de, O> Deserialize<'de> for OctreeLevel<O>
where
    O: OctreeTypes + Deserialize<'de> + HasPosition,
    FieldOf<O>: Deserialize<'de>,
    ElementOf<O>: Deserialize<'de>,
    PositionOf<O>: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::*;
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Data,
            BottomLeft,
        }

        struct OctreeLevelVisitor<O>(std::marker::PhantomData<O>);
        impl<'de, O> Visitor<'de> for OctreeLevelVisitor<O>
        where
            O: OctreeTypes + Deserialize<'de> + HasPosition,
            FieldOf<O>: Deserialize<'de>,
            ElementOf<O>: Deserialize<'de>,
            PositionOf<O>: Deserialize<'de>,
        {
            type Value = OctreeLevel<O>;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct OctreeLevel<O>")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let data = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(0, &self))?;
                let bottom_left = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(1, &self))?;
                Ok(OctreeLevel::new(data, bottom_left))
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut data = None;
                let mut bottom_left = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Data => {
                            if data.is_some() {
                                return Err(Error::duplicate_field("data"));
                            }
                            data = Some(map.next_value()?);
                        }
                        Field::BottomLeft => {
                            if bottom_left.is_some() {
                                return Err(Error::duplicate_field("bottom_left"));
                            }
                            bottom_left = Some(map.next_value()?);
                        }
                    }
                }
                let data = data.ok_or_else(|| Error::missing_field("data"))?;
                let bottom_left = bottom_left.ok_or_else(|| Error::missing_field("bottom_left"))?;
                Ok(OctreeLevel::new(data, bottom_left))
            }
        }

        const FIELDS: &'static [&'static str] = &["data", "bottom_left"];
        deserializer.deserialize_struct(
            "OctreeLevel",
            FIELDS,
            OctreeLevelVisitor(std::marker::PhantomData),
        )
    }
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
impl<O> Eq for OctreeLevel<O>
where
    O: OctreeTypes + Eq,
    ElementOf<O>: Eq,
{
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

/// Represents termination of the recursive Octree type.
/// Only allows for Leaf nodes since we are at the bottom of the tree.
#[derive(PartialEq, Eq, Debug, Serialize, Deserialize)]
pub struct OctreeBase<E, N: Scalar> {
    /// Since we have no Node variant here our type is isomorphic to Option.
    /// Because of this an Option is used in place of a custom type as Option has far more support by default.
    data: Option<E>,
    /// Since we're at the base of the tree we no longer have octants.
    /// This point represents the point our data E is at in the tree.
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
    /// Convenience wrapper to convert the output of [get_octant](struct.OctreeLevel.html#get_octant) to a usize
    /// This is a safe transformation since [OctantId](enum.OctantId.html) is always within `0..8`
    fn get_octant_index<P>(&self, pos: P) -> usize
    where
        P: Borrow<<Self as HasPosition>::Position>,
    {
        self.get_octant(pos).to_usize().unwrap()
    }

    /// Determines the sub octant of `self`  that `pos_ref` resides in.
    /// Returns the OctantId specifying that Octant.
    /// This method assumes `pos_ref` is within the boundaries of `self` and does no bounds checking.
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
    /// Create a new Octree at `Point3::<FieldOf<Self>>::(0, 0, 0)`
    pub fn at_origin(init: Option<ElementOf<Self>>) -> Self {
        let data: DataOf<Self> = init
            .map(<Self as HasData>::Data::leaf)
            .unwrap_or_else(<Self as HasData>::Data::empty);
        OctreeLevel::new(data, Point3::origin())
    }

    fn with_data(&self, data: DataOf<Self>) -> Self {
        OctreeLevel {
            data: data,
            ..(*self.clone())
        }
    }

    /// Returns the root point of this node of the Octree.
    /// For example:
    ///
    /// ```
    /// let octree: Octree8<u32, u8> = Octree8::at_origin(None);
    /// assert_eq!(octree.root_point(), Point3::origin())
    /// ```
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

    /// Maps over a single level of the Octree using 3 functions to handle each possible case.
    /// This should not be confused with the more traditional concept of map from Functor.
    /// To accomplish something like that use `&octree.into_iter().map(...)`
    ///
    /// Example usage:
    ///
    /// ```
    /// let octree = Octree::<u32, u8, U64>::at_origin(None);
    ///
    /// let number_of_leaves = octree.map(
    ///     || 0,
    ///     |leaf_elem| 1,
    ///     |node_children| 8,
    /// );
    /// assert_eq!(number_of_leaves, 0);
    /// ```
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
