use super::octant::{Octant, Octant::*};
use super::Number;
use crate::terrain::OrdPoint3;
use amethyst::core::nalgebra::geometry::Point3;
use std::{borrow::Borrow, cmp::Ordering, fmt};

#[derive(Clone, PartialEq, Eq)]
pub struct OctantDimensions {
    bottom_left: Point3<Number>,
    diameter: Number,
}

impl fmt::Debug for OctantDimensions {
    fn fmt(&self, fmt: &mut fmt::Formatter) -> fmt::Result {
        fmt.debug_struct("OctantDimensions")
            .field("bottom_left", &OrdPoint3::new(self.bottom_left))
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
    pub fn new(bottom_left: Point3<Number>, diameter: Number) -> Self {
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
        self.bottom_left.x + self.diameter - 1
    }
    pub fn y_min(&self) -> Number {
        self.bottom_left.y
    }
    pub fn y_max(&self) -> Number {
        self.bottom_left.y + self.diameter - 1
    }
    pub fn z_min(&self) -> Number {
        self.bottom_left.z
    }
    pub fn z_max(&self) -> Number {
        self.bottom_left.z + self.diameter - 1
    }

    pub fn top_right(&self) -> Point3<Number> {
        let mut top_right = self.bottom_left.clone();
        for e in top_right.iter_mut() {
            *e += self.diameter - 1;
        }
        return top_right;
    }

    pub fn center(&self) -> Point3<Number> {
        let radius = self.diameter / 2;
        let mut center = self.bottom_left.clone();
        for e in center.iter_mut() {
            *e += radius;
        }
        return center;
    }

    pub fn bottom_left(&self) -> Point3<Number> {
        self.bottom_left.clone()
    }

    pub fn diameter(&self) -> Number {
        self.diameter
    }

    pub fn get_octant<P>(&self, pos_ref: P) -> Octant
    where
        P: Borrow<Point3<Number>>,
    {
        let pos = pos_ref.borrow();
        let center = self.center();
        match (pos.x >= center.x, pos.y >= center.y, pos.z >= center.z) {
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