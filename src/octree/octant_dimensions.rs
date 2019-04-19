use super::{
    octant::{Octant, Octant::*},
    octant_face::OctantFace,
    Number,
};
use amethyst::core::nalgebra::geometry::Point3;
use num_traits::ToPrimitive;
use std::{borrow::Borrow, cmp::Ordering, fmt};

#[derive(Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OctantDimensions {
    pub bottom_left: Point3<Number>,
    diameter: u16,
}

impl fmt::Debug for OctantDimensions {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("OctantDimensions")
            .field("bottom_left", &self.bottom_left)
            .field("diameter", &self.diameter)
            .finish()
    }
}

impl PartialOrd for OctantDimensions {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for OctantDimensions {
    fn cmp(&self, other: &Self) -> Ordering {
        use std::cmp::Ordering::*;
        let cmps = (
            self.bottom_left.x.cmp(&other.bottom_left.x),
            self.bottom_left.y.cmp(&other.bottom_left.y),
            self.bottom_left.z.cmp(&other.bottom_left.z),
            self.diameter.cmp(&other.diameter),
        );
        match cmps {
            (Greater, Greater, Greater, _) => Greater,
            (Greater, _, _, _) => Greater,
            (Equal, Greater, _, _) => Greater,
            (Equal, Equal, Greater, _) => Greater,
            (Equal, Equal, Equal, Greater) => Greater,
            (Equal, Equal, Equal, Equal) => Equal,
            (_, _, _, _) => Less,
        }
    }
}

impl OctantDimensions {
    pub fn new(bottom_left: Point3<Number>, diameter: u16) -> Self {
        OctantDimensions {
            bottom_left,
            diameter,
        }
    }

    pub fn nearest_octant_point(p: Point3<Number>, height: u32) -> Point3<Number> {
        let multiple = Number::pow(2, height);
        let mut new_point = p.clone();
        for e in new_point.iter_mut() {
            *e = (*e as f32 / multiple as f32).floor() as Number * multiple;
        }
        return new_point;
    }

    pub fn x_min(&self) -> Number {
        self.bottom_left.x
    }
    pub fn x_max(&self) -> Number {
        self.bottom_left.x + (self.diameter - 1) as u8
    }
    pub fn y_min(&self) -> Number {
        self.bottom_left.y
    }
    pub fn y_max(&self) -> Number {
        self.bottom_left.y + (self.diameter - 1) as u8
    }
    pub fn z_min(&self) -> Number {
        self.bottom_left.z
    }
    pub fn z_max(&self) -> Number {
        self.bottom_left.z + (self.diameter - 1) as u8
    }

    pub fn top_right(&self) -> Point3<u16> {
        return Point3::new(
            self.bottom_left.x as u16 + self.diameter,
            self.bottom_left.y as u16 + self.diameter,
            self.bottom_left.z as u16 + self.diameter,
        );
    }

    pub fn center(&self) -> Point3<Number> {
        let radius = (self.diameter / 2) as u8;
        return Point3::new(
            self.bottom_left.x + radius,
            self.bottom_left.y + radius,
            self.bottom_left.z + radius,
        );
    }

    pub fn bottom_left(&self) -> Point3<Number> {
        self.bottom_left.clone()
    }

    pub fn diameter(&self) -> u16 {
        self.diameter
    }

    /// Returns the root_point of the octant that if adjacent to a face of this octant
    pub fn face_adjacent_point(&self, face: OctantFace) -> Point3<Number> {
        use super::octant_face::OctantFace::*;
        match face {
            Back => Point3::new(self.x_min(), self.y_min(), self.z_max() + 1),
            Up => Point3::new(self.x_min(), self.y_max() + 1, self.z_min()),
            Front => Point3::new(self.x_min(), self.y_min(), self.z_min() - 1),
            Down => Point3::new(self.x_min(), self.y_min() - 1, self.z_min()),
            Right => Point3::new(self.x_max() + 1, self.y_min(), self.z_min()),
            Left => Point3::new(self.x_min() - 1, self.y_min(), self.z_min()),
        }
    }

    pub fn get_octant<P>(&self, pos_ref: P) -> Octant
    where
        P: Borrow<Point3<Number>>,
    {
        let pos = pos_ref.borrow();
        //let center = self.center();
        let r = (self.diameter / 2) as u8;
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

    pub fn get_octant_index<P>(&self, pos_ref: P) -> usize
    where
        P: Borrow<Point3<Number>>,
    {
        // We never fail to convert an octant to a usize
        self.get_octant(pos_ref).to_usize().unwrap()
    }
}
