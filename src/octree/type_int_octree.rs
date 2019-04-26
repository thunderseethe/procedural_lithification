extern crate typenum;
use super::octant::Octant;
use amethyst::core::nalgebra::{Point3, Scalar};
use num_traits::*;
use std::borrow::Borrow;
use std::rc::Rc;

//trait Unwrap {
//    type Output: Diameter;
//}
//impl<U: Diameter + Unsigned + NonZero, B: Bit> Unwrap for UInt<U, B> {
//    type Output = U;
//}
//impl Unwrap for UInt<UTerm, B1> {
//    type Output = UInt<UTerm, B1>;
//}
//
//trait Diameter: Unwrap + PowerOfTwo + NonZero {}
//impl<U> Diameter for U where U: Unwrap + PowerOfTwo + NonZero {}
//
//enum OctreeData<E, N, D>
//where
//    N: Scalar,
//    D: Diameter,
//{
//    Node([Rc<Octree<E, N, <D as Unwrap>::Output>>; 8]),
//    Leaf(Rc<E>),
//    Empty,
//}
//
//struct Octree<E, N, D>
//where
//    N: Scalar,
//    D: Diameter,
//{
//    data: OctreeData<E, N, D>,
//    _marker: std::marker::PhantomData<D>,
//}
enum LevelData<E, O> {
    Node([Rc<O>; 8]),
    Leaf(Rc<E>),
    Empty,
}
struct OctreeLevel<E, N, O>
where
    N: Scalar,
{
    data: LevelData<E, O>,
    bottom_left: Point3<N>,
}

trait Diameter<N> {
    fn diameter() -> N;
}
impl<E, N, O: Diameter<N>> Diameter<N> for OctreeLevel<E, N, O>
where
    N: Scalar + Num,
{
    fn diameter() -> N {
        O::diameter() * (N::one() + N::one())
    }
}
impl<E, N> Diameter<N> for OctreeBase<E, N>
where
    N: Scalar + Num,
{
    fn diameter() -> N {
        N::one()
    }
}

trait Number: Scalar + Num + PartialOrd + Shr<Self, Output = Self> {}

use std::ops::*;
impl<E, N, O: Diameter<N>> OctreeLevel<E, N, O>
where
    N: Number,
{
    fn get_octant_index<P>(&self, pos: P) -> usize
    where
        P: Borrow<Point3<N>>,
    {
        self.get_octant(pos).to_usize().unwrap()
    }

    fn get_octant<P>(&self, pos_ref: P) -> Octant
    where
        P: Borrow<Point3<N>>,
    {
        use crate::octree::octant::Octant::*;
        let pos = pos_ref.borrow();
        let r = Self::diameter() >> N::one();
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
}
impl<E, N, O: Insert<E, N> + Diameter<N>> OctreeLevel<E, N, O> {
    fn create_sub_nodes<P>(&self, pos: P, elem: Option<E>, default: BaseData<E>) -> Self
    where
        P: Borrow<Point3<N>>,
    {
        let modified_octant = self.get_octant(pos.borrow());
        let octree_nodes: [Rc<O>; 8] 
    }
}

enum BaseData<E> {
    Leaf(Rc<E>),
    Empty,
}
struct OctreeBase<E, N: Scalar> {
    data: BaseData<E>,
    bottom_left: Point3<N>,
}

trait Get<E, N: Scalar> {
    fn get<P>(&self, pos: P) -> Option<&E>
    where
        P: Borrow<Point3<N>>;
}
impl<E, N, O> Get<E, N> for OctreeLevel<E, N, O>
where
    N: Number,
    O: Get<E, N> + Diameter<N>,
{
    fn get<P>(&self, pos: P) -> Option<&E>
    where
        P: Borrow<Point3<N>>,
    {
        use LevelData::*;
        match self.data {
            Empty => None,
            Leaf(ref elem) => Some(elem.as_ref()),
            Node(ref octants) => {
                let index: usize = self.get_octant_index(pos.borrow());
                octants[index].get(pos)
            }
        }
    }
}
impl<E, N> Get<E, N> for OctreeBase<E, N>
where
    N: Number,
{
    fn get<P>(&self, _pos: P) -> Option<&E>
    where
        P: Borrow<Point3<N>>,
    {
        use BaseData::*;
        match self.data {
            Empty => None,
            Leaf(ref elem) => Some(elem.as_ref()),
        }
    }
}

trait Insert<E, N: Scalar> {
    fn insert<P, R>(&self, pos: P, elem: R) -> Self
    where
        P: Borrow<Point3<N>>,
        R: Into<Rc<E>>;
}
//impl<E, N, O> Insert<E, N> for OctreeLevel<E, N, O> where N: Scalar {
//    fn insert<P, R>(&self, pos: P, elem: R) -> Self
//    where
//        P: Borrow<Point3<N>>,
//        R: Into<Rc<E>>,
//    {
//        match self.data {
//            Empty => self.create_sub_nodes()
//        }
//    }
//}
