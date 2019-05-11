//! Contains traits that describe properties of an Octree and it's data.
use super::*;
use crate::field::{FieldOf, FieldType, Number};
use typenum::{Bit, PowerOfTwo, UInt, Unsigned, B0, U1};
// Hello, it's your good pal bottom up recursion. Now with types

pub trait ElementType {
    type Element;
}

impl<E, N: Scalar> ElementType for OctreeBase<E, N> {
    type Element = E;
}
impl<'a, T> ElementType for &'a T
where
    T: ElementType,
{
    type Element = ElementOf<T>;
}
impl<E, N: Number> FieldType for OctreeBase<E, N> {
    type Field = N;
}
impl<'a, T> FieldType for &'a T
where
    T: FieldType,
{
    type Field = FieldOf<T>;
}

impl<O: OctreeTypes> ElementType for OctreeLevel<O> {
    type Element = O::Element;
}
impl<O: OctreeTypes> FieldType for OctreeLevel<O> {
    type Field = O::Field;
}

/// ElementType and FieldType are used to carry the two type parameters E and N of OctreeBase<E, N> up each OctreeLevel.
/// This prevents each octree from requiring it's own E and N parameters and ensures that all levels of the Octree are using the same Element and Field.
pub trait OctreeTypes: ElementType + FieldType {}
impl<T> OctreeTypes for T where T: ElementType + FieldType {}

/// Trait to unify our OctreeBase::Data and OctreeLevel::Data empty nodes.
/// This trait in tandem with (Leaf)[trait.Leaf.html] allows for operations on the Octree to refer to their particular type's Data generically
/// Often a method won't have access to LevelData or Option specifically since it is dealing with an opaque type parameter O. However if we constrain O to implement [HasData](trait.HasData.html) then we can construct an instance of it's data via Empty::empty() or Leaf::leaf()
pub trait Empty {
    fn empty() -> Self;
    fn is_empty(&self) -> bool;
}
impl<O> Empty for LevelData<O>
where
    O: OctreeTypes,
{
    fn empty() -> Self {
        LevelData::Empty
    }

    fn is_empty(&self) -> bool {
        match self {
            LevelData::Empty => true,
            _ => false,
        }
    }
}
impl<E> Empty for Option<E> {
    fn empty() -> Self {
        None
    }

    fn is_empty(&self) -> bool {
        self.is_none()
    }
}

/// Trait to unify our OctreeBase::Data and OctreeLevel::Data leaf nodes.
pub trait Leaf<T> {
    fn leaf(value: T) -> Self;
    fn is_leaf(&self) -> bool;
    fn get_leaf(&self) -> &T;
}
impl<O> Leaf<ElementOf<O>> for LevelData<O>
where
    O: OctreeTypes,
{
    fn leaf(value: ElementOf<O>) -> Self {
        LevelData::Leaf(value)
    }

    fn is_leaf(&self) -> bool {
        match self {
            LevelData::Leaf(_) => true,
            _ => false,
        }
    }

    fn get_leaf(&self) -> &ElementOf<O> {
        match self {
            LevelData::Leaf(ref e) => e,
            _ => panic!("Called get_leaf() on non leaf node."),
        }
    }
}
impl<E> Leaf<E> for Option<E> {
    fn leaf(value: E) -> Self {
        Some(value)
    }

    fn is_leaf(&self) -> bool {
        self.is_some()
    }

    fn get_leaf(&self) -> &E {
        self.as_ref().expect("Called get_leaf() on empty node.")
    }
}

/// Tracks the diameter of octant that our Self type covers
/// Like OctreeTypes this is using bottom up recursion.
/// OctreeBase starts at 1 (as it only covers a single element)
/// Each OctreeLevel on top of this doubles it's sub octrees diameter
pub trait Diameter {
    type Diameter: Unsigned + Double;

    const DIAMETER: usize;

    fn diameter(&self) -> usize {
        Self::DIAMETER
    }
}
impl<O> Diameter for OctreeLevel<O>
where
    O: Diameter + OctreeTypes,
{
    type Diameter = <O::Diameter as Double>::Output;

    const DIAMETER: usize = O::DIAMETER << 1;
}
impl<E, N> Diameter for OctreeBase<E, N>
where
    N: Number,
{
    type Diameter = U1;

    const DIAMETER: usize = 1;
}
impl<'a, T> Diameter for &'a T
where
    T: Diameter,
{
    type Diameter = T::Diameter;

    const DIAMETER: usize = T::DIAMETER;
}

/// This a more specific version of Shl<B1>, with the caveat that is enforces it's Output implements Double
/// Because the Output must also implement Double the trait can be recursed as examplified by the Diameter trait
pub trait Double: PowerOfTwo {
    type Output: Unsigned + Double;
}
impl<U: Unsigned, B: Bit> Double for UInt<U, B>
where
    Self: PowerOfTwo,
{
    type Output = UInt<Self, B0>;
}

/// Trait for the Data of an Octree
/// Provides accessor methods to Data as well as enforces that the data is Leaf-like and Empty-like
pub trait HasData: ElementType {
    type Data: Leaf<Self::Element> + Empty;

    fn data(&self) -> &Self::Data;
    fn into_data(self) -> Self::Data;
}
impl<O> HasData for OctreeLevel<O>
where
    O: OctreeTypes,
{
    type Data = LevelData<O>;

    fn data(&self) -> &Self::Data {
        &self.data
    }
    fn into_data(self) -> Self::Data {
        self.data
    }
}
impl<E, N: Scalar> HasData for OctreeBase<E, N> {
    type Data = Option<E>;

    fn data(&self) -> &Self::Data {
        &self.data
    }
    fn into_data(self) -> Self::Data {
        self.data
    }
}

/// Trait for the Position of an Octree
pub trait HasPosition {
    type Position;

    fn position(&self) -> &Self::Position;
}
impl<O> HasPosition for OctreeLevel<O>
where
    O: OctreeTypes,
{
    type Position = Point3<FieldOf<Self>>;

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

/// We can move a data type up the tree arbitratily as long as it's not a Node variant.
/// This is helpful when building an Octree and when compressing an Octree.
/// During these operations we want to unify elements of are sub octree that are all equal into one element of our parent octree.
/// The easiest way to accomplish this is with a recast as done here
impl<O> From<LevelData<O>> for LevelData<OctreeLevel<O>>
where
    O: OctreeTypes,
{
    fn from(lower: LevelData<O>) -> Self {
        match lower {
            LevelData::Empty => LevelData::Empty,
            LevelData::Leaf(elem) => LevelData::Leaf(elem),
            // Since each node _should_ have a different position this case should never come up.
            // If it does more than likely an Octree invariant has been invalidated.
            LevelData::Node(_) => {
                panic!("Attempted to convert LevelData::Node from O to OctreeLevel<O>.")
            }
        }
    }
}

/// This tranformation is always safe since OctreeBase::Data is a subset of OctreeLevel::Data
impl<O: OctreeTypes> From<Option<ElementOf<O>>> for LevelData<O> {
    fn from(opt: Option<ElementOf<O>>) -> Self {
        opt.map(LevelData::Leaf).unwrap_or(LevelData::Empty)
    }
}
