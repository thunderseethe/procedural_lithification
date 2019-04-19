use amethyst::core::nalgebra::{Point3, Scalar};
use num_traits::{AsPrimitive, PrimInt};
use std::{
    borrow::{Borrow, BorrowMut},
    marker::PhantomData,
};

#[derive(Debug)]
pub struct Cuboid<N: Scalar> {
    bottom_left_front: Point3<N>,
    top_right_back: Point3<N>,
}

impl<N: Scalar + PrimInt + AsPrimitive<usize>> Cuboid<N> {
    pub fn new(bottom_left_front: Point3<N>, top_right_back: Point3<N>) -> Self {
        Cuboid {
            bottom_left_front,
            top_right_back,
        }
    }

    pub fn iter(&self) -> CuboidIter<N, &Point3<N>> {
        CuboidIter::new(&self.bottom_left_front, &self.top_right_back)
    }
}

impl<N: Scalar + PrimInt + AsPrimitive<usize>> IntoIterator for Cuboid<N> {
    type Item = Point3<N>;
    type IntoIter = CuboidIter<N, Point3<N>>;

    fn into_iter(self) -> Self::IntoIter {
        CuboidIter::new(self.bottom_left_front, self.top_right_back)
    }
}

impl<'a, N: Scalar + PrimInt + AsPrimitive<usize>> IntoIterator for &'a Cuboid<N> {
    type Item = Point3<N>;
    type IntoIter = CuboidIter<N, &'a Point3<N>>;

    fn into_iter(self) -> Self::IntoIter {
        self.iter()
    }
}

pub struct CuboidIter<N, P> {
    x: Option<N>,
    y: Option<N>,
    z: Option<N>,
    start: P,
    end: P,
    size: usize,
    _marker: PhantomData<N>,
}
impl<N: Scalar + PrimInt + AsPrimitive<usize>, P: Borrow<Point3<N>>> CuboidIter<N, P> {
    pub fn new(start: P, end: P) -> Self {
        let s = start.borrow();
        let e = end.borrow();
        let (z_diff, y_diff, x_diff): (usize, usize, usize) =
            ((e.z - s.z).as_(), (e.y - s.y).as_(), (e.x - s.x).as_());
        let size: usize = z_diff * y_diff * x_diff;
        CuboidIter {
            x: Some(s.x),
            y: Some(s.y),
            z: Some(s.z),
            start,
            end,
            size,
            _marker: PhantomData,
        }
    }
}

fn optional_triple<A, B, C>(a: Option<A>, b: Option<B>, c: Option<C>) -> Option<(A, B, C)> {
    a.and_then(|a_val| b.and_then(|b_val| c.map(|c_val| (a_val, b_val, c_val))))
}

impl<N: Scalar + PrimInt, P: Borrow<Point3<N>>> Iterator for CuboidIter<N, P> {
    type Item = Point3<N>;

    fn next(&mut self) -> Option<Self::Item> {
        optional_triple(self.x, self.y, self.z).and_then(|(x, y, z)| {
            let end = self.end.borrow();
            if end.x <= x || end.y <= y || end.z <= z {
                self.x = None;
                self.y = None;
                self.z = None;
                return None;
            }
            let p = Point3::new(x, y, z);
            let start = self.start.borrow();
            self.x = x.checked_add(&N::one());
            if self.x.map(|x| x >= end.x).unwrap_or(false) {
                self.x = Some(start.x);
                self.y = y.checked_add(&N::one());

                if self.y.map(|y| y >= end.y).unwrap_or(false) {
                    self.y = Some(start.y);
                    self.z = z.checked_add(&N::one());
                }
            }
            return Some(p);
        })
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.size, Some(self.size))
    }
}
impl<N: Scalar + PrimInt, P: Borrow<Point3<N>>> ExactSizeIterator for CuboidIter<N, P> {
    fn len(&self) -> usize {
        self.size
    }
}

#[derive(Debug)]
pub struct Cube<N: Scalar> {
    shape: Cuboid<N>,
}
impl<N: Scalar + PrimInt + AsPrimitive<usize>> Cube<N> {
    pub fn new<P>(center: P, radius: N) -> Self
    where
        P: Borrow<Point3<N>>,
    {
        let c = center.borrow();
        Cube {
            shape: Cuboid::new(
                Point3::new(c.x - radius, c.y - radius, c.z - radius),
                Point3::new(
                    c.x + radius + N::one(),
                    c.y + radius + N::one(),
                    c.z + radius + N::one(),
                ),
            ),
        }
    }

    pub fn iter(&self) -> CuboidIter<N, &Point3<N>> {
        self.shape.iter()
    }
}
impl<N: Scalar + PrimInt + AsPrimitive<usize>> IntoIterator for Cube<N> {
    type Item = Point3<N>;
    type IntoIter = <Cuboid<N> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.shape.into_iter()
    }
}
impl<'a, N: Scalar + PrimInt + AsPrimitive<usize>> IntoIterator for &'a Cube<N> {
    type Item = Point3<N>;
    type IntoIter = <&'a Cuboid<N> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.shape).into_iter()
    }
}

#[derive(Debug)]
pub struct Sphere<N: Scalar> {
    center: Point3<N>,
    radius: N,
    bounding_cube: Cube<N>,
}
impl<N: Scalar + PrimInt + AsPrimitive<usize>> Sphere<N> {
    pub fn new(center: Point3<N>, radius: N) -> Self {
        let bounding_cube = Cube::new(&center, radius);
        Sphere {
            center,
            radius,
            bounding_cube,
        }
    }
    pub fn with_origin(radius: N) -> Self {
        let center = Point3::new(N::zero(), N::zero(), N::zero());
        Sphere::new(center, radius)
    }

    pub fn iter(&self) -> impl Iterator<Item = Point3<N>> + '_ {
        self.bounding_cube.iter().filter(move |p| {
            let x = Sphere::difference(p.x, self.center.x);
            let y = Sphere::difference(p.y, self.center.y);
            let z = Sphere::difference(p.z, self.center.z);
            x * x + y * y + z * z <= self.radius * self.radius
        })
    }
    pub fn into_iter(self) -> impl Iterator<Item = Point3<N>> {
        let center_x = self.center.x;
        let center_y = self.center.y;
        let center_z = self.center.z;
        let radius = self.radius;
        self.bounding_cube.into_iter().filter(move |p| {
            let x = Sphere::difference(p.x, center_x);
            let y = Sphere::difference(p.y, center_y);
            let z = Sphere::difference(p.z, center_z);
            x * x + y * y + z * z <= radius * radius
        })
    }

    // Returns a postive difference between two numbers. This matters for unsigned numbers. Since this is used in distance calculation sign doesn't matter.
    pub fn difference(a: N, b: N) -> N {
        a.max(b) - a.min(b)
    }
}

impl<N: Scalar + PrimInt + AsPrimitive<usize>> IntoIterator for Sphere<N> {
    type Item = Point3<N>;
    type IntoIter = SphereIter<N, Point3<N>, CuboidIter<N, Point3<N>>>;

    fn into_iter(self) -> Self::IntoIter {
        SphereIter::new(self.radius, self.center, self.bounding_cube.into_iter())
    }
}
impl<'a, N: Scalar + PrimInt + AsPrimitive<usize>> IntoIterator for &'a Sphere<N> {
    type Item = Point3<N>;
    type IntoIter = SphereIter<N, &'a Point3<N>, CuboidIter<N, &'a Point3<N>>>;

    fn into_iter(self) -> Self::IntoIter {
        SphereIter::new(self.radius, &self.center, (&self.bounding_cube).into_iter())
    }
}

pub struct SphereIter<N: Scalar, P, I> {
    radius: N,
    center: P,
    cube_iter: I,
}

impl<
        N: Scalar + PrimInt + AsPrimitive<usize>,
        P: Borrow<Point3<N>>,
        I: Iterator<Item = Point3<N>>,
    > SphereIter<N, P, I>
{
    pub fn new(radius: N, center: P, cube_iter: I) -> Self {
        SphereIter {
            radius,
            center,
            cube_iter,
        }
    }
}

impl<
        N: Scalar + PrimInt + AsPrimitive<usize>,
        P: Borrow<Point3<N>>,
        I: Iterator<Item = Point3<N>>,
    > Iterator for SphereIter<N, P, I>
{
    type Item = Point3<N>;

    fn next(&mut self) -> Option<Self::Item> {
        let center = self.center.borrow();
        let mut opt_point_cand = self.cube_iter.borrow_mut().next();
        while let Some(point_cand) = opt_point_cand {
            let x = Sphere::difference(point_cand.x, center.x);
            let y = Sphere::difference(point_cand.y, center.y);
            let z = Sphere::difference(point_cand.z, center.z);
            if x * x + y * y + z * z <= self.radius * self.radius {
                break;
            } else {
                opt_point_cand = self.cube_iter.next();
            }
        }
        return opt_point_cand;
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn sphere_generates_expected_points() {
        let sphere = Sphere::with_origin(1);
        let results: HashSet<Point3<i32>> = sphere.iter().collect();
        let mut expected = HashSet::new();
        expected.insert(Point3::new(0, 0, 0));
        expected.insert(Point3::new(0, 0, 1));
        expected.insert(Point3::new(0, 1, 0));
        expected.insert(Point3::new(1, 0, 0));
        expected.insert(Point3::new(0, 0, -1));
        expected.insert(Point3::new(0, -1, 0));
        expected.insert(Point3::new(-1, 0, 0));
        assert_eq!(results, expected);
    }

    macro_rules! test_cuboid_range {
        (($sx: expr, $sy: expr, $sz: expr) => ($ex: expr, $ey: expr, $ez: expr)) => {
            let cuboid = Cuboid::new(Point3::new($sx, $sy, $sz), Point3::new($ex, $ey, $ez));

            let mut expected = HashSet::new();
            for x in $sx..$ex {
                for y in $sy..$ey {
                    for z in $sz..$ez {
                        expected.insert(Point3::new(x, y, z));
                    }
                }
            }

            for point in cuboid.into_iter() {
                assert!(expected.contains(&point), "Missing point: {}", point);
                expected.remove(&point);
            }

            assert!(expected.len() == 0, "Expected points: {:?}", expected);
        };
        (($sx: expr, $sy: expr, $sz: expr) => ($ex: expr, $ey: expr, $ez: expr), $num:ty) => {
            let cuboid: Cuboid<$num> =
                Cuboid::new(Point3::new($sx, $sy, $sz), Point3::new($ex, $ey, $ez));

            let mut expected = HashSet::new();
            for x in $sx..$ex {
                for y in $sy..$ey {
                    for z in $sz..$ez {
                        expected.insert(Point3::new(x, y, z));
                    }
                }
            }

            for point in cuboid.into_iter() {
                assert!(expected.contains(&point), "Missing point: {}", point);
                expected.remove(&point);
            }

            assert!(expected.len() == 0, "Expected points: {:?}", expected);
        };
    }

    #[test]
    fn cuboid_generates_expected_points() {
        test_cuboid_range!((0, 0, 0) => (1, 1, 1));
    }

    #[test]
    fn cuboid_works_with_arbitrary_points() {
        test_cuboid_range!((0, 0, 128) => (64, 64, 192));
    }

    #[test]
    fn cuboid_works_near_max_integers() {
        test_cuboid_range!((0, 0, 192) => (64, 64, 256));
    }

    #[test]
    fn cuboid_of_size_1() {
        test_cuboid_range!((127, 126, 251) => (128, 127, 252), u16);
    }

}
