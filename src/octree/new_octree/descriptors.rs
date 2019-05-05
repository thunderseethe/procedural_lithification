/// Contains traits that describe properties of an Octree.
use super::*;
use alga::general::{ClosedAdd, ClosedSub, SubsetOf};
use num_traits::Num;
use std::ops::{Mul, Shl, Shr};
use typenum::{Bit, Pow, PowerOfTwo, UInt, Unsigned, B0, U1};
// Hello, it's your good pal bottom up recursion. Now with types
pub trait ElementType {
    type Element;
}
pub trait FieldType {
    type Field: Number;
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

// Convenience wrapper to avoid busting my + key
pub trait OctreeTypes: ElementType + FieldType {}
impl<T> OctreeTypes for T where T: ElementType + FieldType {}

/// Composite trait desciribing a numerical type that can be used for the coordinates of an Octree.
pub trait Number:
    Scalar
    + Num
    + NumCast
    + PartialOrd
    + ClosedSub
    + ClosedAdd
    + Shr<Self, Output = Self>
    + Shl<Self, Output = Self>
    + AsPrimitive<usize>
{
}
impl<T> Number for T where
    T: Scalar
        + Num
        + NumCast
        + PartialOrd
        + ClosedSub
        + ClosedAdd
        + Shr<Self, Output = Self>
        + Shl<Self, Output = Self>
        + AsPrimitive<usize>
{
}

/// Trait to unify our OctreeBase::Data and OctreeLevel::Data empty nodes.
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

pub trait Diameter {
    type Diameter: Unsigned + Double;
    fn diameter() -> usize;

    fn get_diameter(&self) -> usize {
        Self::diameter()
    }
}
impl<O> Diameter for OctreeLevel<O>
where
    O: Diameter + OctreeTypes,
{
    type Diameter = <O::Diameter as Double>::Output;

    fn diameter() -> usize {
        O::diameter() << 1
    }
}
impl<E, N> Diameter for OctreeBase<E, N>
where
    N: Number,
{
    type Diameter = U1;

    fn diameter() -> usize {
        1
    }
}
impl<'a, T> Diameter for &'a T
where
    T: Diameter,
{
    type Diameter = T::Diameter;

    fn diameter() -> usize {
        T::diameter()
    }
}

/// This a more specific version of ShL<B1>, with the caveat that is enforces it's Output implements Double
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

pub trait HasPosition {
    type Position;

    fn position(&self) -> &Self::Position;
}
impl<'a, T> HasPosition for &'a T
where
    T: HasPosition,
{
    type Position = PositionOf<T>;

    fn position(&self) -> &Self::Position {
        &self.position()
    }
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
//impl<E, N: Number> From<Option<E>> for LevelData<OctreeBase<E, N>> {
//    fn from(opt: Option<E>) -> Self {
//        opt.map(LevelData::Leaf).unwrap_or(LevelData::Empty)
//    }
//}
impl<O> From<LevelData<O>> for LevelData<OctreeLevel<O>>
where
    O: OctreeTypes,
{
    fn from(lower: LevelData<O>) -> Self {
        match lower {
            LevelData::Empty => LevelData::Empty,
            LevelData::Leaf(elem) => LevelData::Leaf(elem),
            LevelData::Node(nodes) => {
                panic!("Attempted to convert LevelData::Node from O to OctreeLevel<O>.")
            }
        }
    }
}

impl<O: OctreeTypes> From<Option<ElementOf<O>>> for LevelData<O> {
    fn from(opt: Option<ElementOf<O>>) -> Self {
        opt.map(LevelData::Leaf).unwrap_or(LevelData::Empty)
    }
}
