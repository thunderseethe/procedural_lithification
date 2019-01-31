use super::octant_dimensions::OctantDimensions;
use amethyst::core::nalgebra::geometry::Point3;
// Represnt each possible Octant as a sum type.
#[derive(PartialEq, Clone, Eq, Copy, Debug)]
pub enum Octant {
    // x, y, z
    HighHighHigh,
    HighHighLow,
    HighLowHigh,
    HighLowLow,
    LowHighHigh,
    LowHighLow,
    LowLowHigh,
    LowLowLow,
}
use self::Octant::*;

macro_rules! octant_num_conversions {
    ($( $num:ty ),* ) => {
        $(
            impl Into<$num> for Octant {
                fn into(self) -> $num {
                    match self {
                        HighHighHigh => 0,
                        HighHighLow => 1,
                        HighLowHigh => 2,
                        HighLowLow => 3,
                        LowHighHigh => 4,
                        LowHighLow => 5,
                        LowLowHigh => 6,
                        LowLowLow => 7,
                    }
                }
            }

            impl From<$num> for Octant {
                fn from(num: $num) -> Self {
                    match num {
                        0 => HighHighHigh,
                        1 => HighHighLow,
                        2 => HighLowHigh,
                        3 => HighLowLow,
                        4 => LowHighHigh,
                        5 => LowHighLow,
                        6 => LowLowHigh,
                        7 => LowLowLow,
                        _ => panic!("Tried to create more than 8 elements in an octree"),
                    }
                }
            }
        )*
    };
}

octant_num_conversions!(u8, u16, u32, u64, usize, i8, i16, i32, i64, isize);

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
}