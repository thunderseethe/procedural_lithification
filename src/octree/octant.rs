use super::octant_dimensions::OctantDimensions;
use amethyst::core::nalgebra::geometry::Point3;
use num_traits::FromPrimitive;

/// Represnt each possible Octant as a sum type.
#[derive(PartialEq, Clone, Eq, Copy, Debug, FromPrimitive, ToPrimitive)]
pub enum Octant {
    // x, y, z
    HighHighHigh = 0,
    HighHighLow = 1,
    HighLowHigh = 2,
    HighLowLow = 3,
    LowHighHigh = 4,
    LowHighLow = 5,
    LowLowHigh = 6,
    LowLowLow = 7,
}
use self::Octant::*;

impl Octant {
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

    pub fn sub_octant_bounds(&self, containing_bounds: &OctantDimensions) -> OctantDimensions {
        let (bottom_left, center) = (containing_bounds.bottom_left(), containing_bounds.center());

        let x_center = if self.is_x_high() {
            center.x
        } else {
            bottom_left.x
        };
        let y_center = if self.is_y_high() {
            center.y
        } else {
            bottom_left.y
        };
        let z_center = if self.is_z_high() {
            center.z
        } else {
            bottom_left.z
        };

        OctantDimensions::new(
            Point3::new(x_center, y_center, z_center),
            containing_bounds.diameter() / 2,
        )
    }

    /// Iterates over all variants of Octant
    pub fn iter() -> OctantIter {
        OctantIter::default()
    }
}

pub struct OctantIter {
    indx: u8,
}
impl Default for OctantIter {
    fn default() -> Self {
        OctantIter { indx: 0 }
    }
}
impl Iterator for OctantIter {
    type Item = Octant;

    fn next(&mut self) -> Option<Self::Item> {
        let octant = Octant::from_u8(self.indx);
        self.indx += 1;
        octant
    }
}
