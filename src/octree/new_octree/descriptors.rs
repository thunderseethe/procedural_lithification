/// Contains traits that describe properties of an Octree.
use super::*;
use num_traits::Num;
use std::ops::{Shl, Shr};
use typenum::{Bit, PowerOfTwo, Shleft, UInt, Unsigned, B0, B1, U1};
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
impl<E, N: Number> FieldType for OctreeBase<E, N> {
    type Field = N;
}

impl<O: OctreeTypes> ElementType for OctreeLevel<O> {
    type Element = O::Element;
}
impl<'a, O: OctreeTypes> ElementType for &'a OctreeLevel<O> {
    type Element = <OctreeLevel<O> as ElementType>::Element;
}
impl<O: OctreeTypes> FieldType for OctreeLevel<O> {
    type Field = O::Field;
}
impl<'a, O: OctreeTypes> FieldType for &'a OctreeLevel<O> {
    type Field = <O as FieldType>::Field;
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
        + Shr<Self, Output = Self>
        + Shl<Self, Output = Self>
        + AsPrimitive<usize>
{
}

/// Trait to unify our BaseData and LevelData empty nodes.
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
impl<E> Empty for BaseData<E> {
    fn empty() -> Self {
        BaseData::Empty
    }

    fn is_empty(&self) -> bool {
        match self {
            BaseData::Empty => true,
            _ => false,
        }
    }
}

/// Trait to unify our BaseData and LevelData leaf nodes.
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
impl<E> Leaf<E> for BaseData<E> {
    fn leaf(value: E) -> Self {
        BaseData::Leaf(value)
    }

    fn is_leaf(&self) -> bool {
        match self {
            BaseData::Leaf(_) => true,
            _ => false,
        }
    }

    fn get_leaf(&self) -> &E {
        match self {
            BaseData::Leaf(ref e) => e,
            _ => panic!("Called get_leaf() on non leaf node."),
        }
    }
}

pub trait Diameter {
    type Diameter: Unsigned + Double;
    fn diameter() -> usize;
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
impl<'a, O> Diameter for &'a OctreeLevel<O>
where
    O: Diameter + OctreeTypes,
{
    type Diameter = <OctreeLevel<O> as Diameter>::Diameter;

    fn diameter() -> usize {
        OctreeLevel::<O>::diameter()
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
impl<'a, E, N> Diameter for &'a OctreeBase<E, N>
where
    N: Number,
{
    type Diameter = <OctreeBase<E, N> as Diameter>::Diameter;

    fn diameter() -> usize {
        OctreeBase::<E, N>::diameter()
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
