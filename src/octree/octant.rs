use super::descriptors::HasPosition;
use crate::octree::descriptors::Number;
use alga::general::ClosedAdd;
use amethyst::core::nalgebra::{Point3, Scalar};
use num_traits::{AsPrimitive, FromPrimitive};

/// Represnt each possible Octant as a sum type.
#[derive(PartialEq, Clone, Eq, Copy, Debug, FromPrimitive, ToPrimitive)]
pub enum OctantId {
    // x, y, z
    HighHighHigh = 7,
    HighHighLow = 6,
    HighLowHigh = 5,
    HighLowLow = 4,
    LowHighHigh = 3,
    LowHighLow = 2,
    LowLowHigh = 1,
    LowLowLow = 0,
}
use self::OctantId::*;

impl OctantId {
    fn is_x_high(&self) -> bool {
        match self {
            HighHighHigh | HighHighLow | HighLowHigh | HighLowLow => true,
            _ => false,
        }
    }

    fn is_y_high(&self) -> bool {
        match self {
            HighHighHigh | HighHighLow | LowHighHigh | LowHighLow => true,
            _ => false,
        }
    }

    fn is_z_high(&self) -> bool {
        match self {
            HighHighHigh | HighLowHigh | LowHighHigh | LowLowHigh => true,
            _ => false,
        }
    }

    pub fn sub_octant_bottom_left<N>(
        &self,
        containing_bottom_left: Point3<N>,
        sub_octant_diameter: usize,
    ) -> Point3<N>
    where
        N: Number,
    {
        let diameter = num_traits::NumCast::from(sub_octant_diameter).unwrap();
        let x = if self.is_x_high() {
            containing_bottom_left.x + diameter
        } else {
            containing_bottom_left.x
        };
        let y = if self.is_y_high() {
            containing_bottom_left.y + diameter
        } else {
            containing_bottom_left.y
        };
        let z = if self.is_z_high() {
            containing_bottom_left.z + diameter
        } else {
            containing_bottom_left.z
        };
        Point3::new(x, y, z)
    }

    /// Iterates over all variants of Octant
    pub fn iter() -> OctantIdIter {
        OctantIdIter::default()
    }
}

pub struct OctantIdIter {
    indx: u8,
}
impl Default for OctantIdIter {
    fn default() -> Self {
        OctantIdIter { indx: 0 }
    }
}
impl Iterator for OctantIdIter {
    type Item = OctantId;

    fn next(&mut self) -> Option<Self::Item> {
        let octant = OctantId::from_u8(self.indx);
        self.indx += 1;
        octant
    }
}

#[derive(Eq, PartialEq, Debug, Copy, Clone, FromPrimitive, ToPrimitive)]
pub enum OctantFace {
    Back = 0,
    Up = 1,
    Front = 2,
    Down = 3,
    Right = 4,
    Left = 5,
}

#[derive(Hash, PartialEq, Eq, Debug)]
pub struct Octant<E, P> {
    pub data: E,
    pub bottom_left_front: P,
    pub diameter: usize,
}
impl<E, P> Octant<E, P> {
    pub fn new(data: E, bottom_left_front: P, diameter: usize) -> Self {
        Octant {
            data,
            bottom_left_front,
            diameter,
        }
    }
}

impl<'a, E, N> Octant<E, &'a Point3<N>>
where
    N: Scalar + ClosedAdd + AsPrimitive<usize>,
{
    pub fn top_right(&self) -> Point3<usize> {
        Point3::new(
            self.bottom_left_front.x.as_() + self.diameter,
            self.bottom_left_front.y.as_() + self.diameter,
            self.bottom_left_front.z.as_() + self.diameter,
        )
    }
}

impl<E, P> HasPosition for Octant<E, P> {
    type Position = P;

    fn position(&self) -> &Self::Position {
        &self.bottom_left_front
    }
}
