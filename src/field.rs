use alga::general::{ClosedAdd, ClosedSub};
use amethyst::core::nalgebra::Scalar;
use num_traits::{AsPrimitive, Num, NumCast};
use std::ops::{Shl, Shr};

/// Trait representing a type that has some associated Field
pub trait FieldType {
    type Field: Number;
}

/// Shorthand to refer to associated Field type of T
pub type FieldOf<T> = <T as FieldType>::Field;

/// Composite trait desciribing a numerical type that can be used as the value of a Field.
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
