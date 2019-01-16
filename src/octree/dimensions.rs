#[derive(Debug, Eq, PartialEq, Clone)]
pub struct Cuboid<N> {
    bottom_left: Point3<N>,
    top_right: Point3<N>,
}
pub struct CuboidIter<N> {
    bounds: Cuboid<N>,
    curr: Point3<N>,
}

impl<N: Scalar> Cuboid<N> {
    pub fn new(bottom_left: Point3<N>, top_right: Point3<N>) -> Self {
        Cuboid {
            // Move bottom left in by 1 in every direction to account for blocks
            botom_left: Point3::new(bottom_left.x + 1, bottom_left.y + 1, bottom_left.z + 1),
            top_right,
        }
    }
}

impl<N: Scalar> IntoIterator for Cuboid<N> {
    type Item = Point3<N>;
    type IntoIter = CuboidIter<N>;

    fn into_iter(self) -> Self::IntoIter {
        CuboidIter {
            curr: self.bottom_left,
            bounds: self,
        }
    }
}

impl<N: Scalar> Iterator for CuboidIter<N> {
    type Item = Point3<N>;

    fn next(&mut self) -> Option<Self::Item> {
        if curr > boudns.top_right {
            None
        }
    }
}
