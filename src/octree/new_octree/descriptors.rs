/// Contains traits that describe properties of an Octree.
use super::*;
use num_traits::{AsPrimitive, Num};
use std::ops::{Shl, Shr};

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
impl<O: OctreeTypes> FieldType for OctreeLevel<O> {
    type Field = O::Field;
}

// Convenience wrapper to avoid busting my + key
pub trait OctreeTypes: ElementType + FieldType {}
impl<T> OctreeTypes for T where T: ElementType + FieldType {}

/// Composite trait desciribing a numerical type that can be used for the coordinates of an Octree.
pub trait Number:
    Scalar + Num + NumCast + PartialOrd + Shr<Self, Output = Self> + Shl<Self, Output = Self>
{
}
impl<T> Number for T where
    T: Scalar + Num + NumCast + PartialOrd + Shr<Self, Output = Self> + Shl<Self, Output = Self>
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
impl<O> Leaf<Rc<O::Element>> for LevelData<O>
where
    O: OctreeTypes,
{
    fn leaf(value: Rc<O::Element>) -> Self {
        LevelData::Leaf(value)
    }

    fn is_leaf(&self) -> bool {
        match self {
            LevelData::Leaf(_) => true,
            _ => false,
        }
    }

    fn get_leaf(&self) -> &Rc<O::Element> {
        match self {
            LevelData::Leaf(ref e) => e,
            _ => panic!("Called get_leaf() on non leaf node."),
        }
    }
}
impl<E> Leaf<Rc<E>> for BaseData<E> {
    fn leaf(value: Rc<E>) -> Self {
        BaseData::Leaf(value)
    }

    fn is_leaf(&self) -> bool {
        match self {
            BaseData::Leaf(_) => true,
            _ => false,
        }
    }

    fn get_leaf(&self) -> &Rc<E> {
        match self {
            BaseData::Leaf(ref e) => e,
            _ => panic!("Called get_leaf() on non leaf node."),
        }
    }
}
