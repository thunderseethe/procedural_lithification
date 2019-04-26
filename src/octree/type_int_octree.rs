extern crate typenum;
use amethyst::core::nalgebra::{Point3, Scalar};
use std::ops::Shr;
use std::rc::Rc;
use typenum::{PowerOfTwo, Shright, Unsigned, B1};

trait Diameter: Unsigned {}
impl<T> Diameter for T where T: Unsigned {}
impl Shr<B1> for Diameter {
    type Output = Diameter;
}

enum OctreeData<E, N, D>
where
    N: Scalar,
    D: Unsigned,
{
    Node([Rc<Octree<E, N, Shright<D, B1>>>; 8]),
    Leaf(Rc<E>),
    Empty,
}

struct Dimensions<N, D>
where
    N: Scalar,
    D: Unsigned,
{
    bottom_left: Point3<N>,
    _marker: std::marker::PhantomData<D>,
}

struct Octree<E, N, D>
where
    N: Scalar,
    D: Unsigned,
{
    data: OctreeData<E, N, D>,
    dimensions: Dimensions<N, D>,
    _marker: std::marker::PhantomData<D>,
}
