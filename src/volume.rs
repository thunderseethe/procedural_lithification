use amethyst::core::nalgebra::{Point3, Scalar};
use num_traits::{AsPrimitive, PrimInt};
use std::{borrow::Borrow, marker::PhantomData};

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

pub struct CuboidIter<N, P> {
    x: N,
    y: N,
    z: N,
    start: P,
    end: P,
    size: usize,
    _marker: PhantomData<N>,
}
impl<N: Scalar + PrimInt + AsPrimitive<usize>, P: Borrow<Point3<N>>> CuboidIter<N, P> {
    pub fn new(start: P, end: P) -> Self {
        let (x, y, z, size) = {
            let s = start.borrow();
            let e = end.borrow();
            let size: usize = ((e.z - s.z) * (e.y - s.y) * (e.x - s.x)).as_();
            (s.x, s.y, s.x, size)
        };
        CuboidIter {
            x,
            y,
            z,
            start,
            end,
            size,
            _marker: PhantomData,
        }
    }
}

impl<N: Scalar + PrimInt, P: Borrow<Point3<N>>> Iterator for CuboidIter<N, P> {
    type Item = Point3<N>;

    fn next(&mut self) -> Option<Self::Item> {
        let start = self.start.borrow();
        let end = self.end.borrow();
        if end.x == self.x && end.y == self.y && end.z == self.z {
            return None;
        }
        self.x = self.x + N::one();
        if self.x > end.x {
            self.x = start.x;
            self.y = self.y + N::one();

            if self.y > end.y {
                self.y = start.y;
                self.z = self.z + N::one();
            }
        }
        return Some(Point3::new(self.x, self.y, self.z));
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        (self.size, Some(self.size))
    }
}

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
                Point3::new(c.x + radius, c.y + radius, c.z + radius),
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
            let x = p.x - self.center.x;
            let y = p.y - self.center.y;
            let z = p.z - self.center.z;
            x * x + y * y + z * z <= self.radius * self.radius
        })
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
}
