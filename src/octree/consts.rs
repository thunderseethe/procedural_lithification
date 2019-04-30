extern crate typenum;

use crate::octree::type_int_octree::*;
use typenum::{Bit, PowerOfTwo, UInt, UTerm, Unsigned};

trait ToOctree<E, N> {
    type Octree: OctreeTypes;
}

impl<E, N: Number> ToOctree<E, N> for UTerm {
    type Octree = OctreeBase<E, N>;
}
impl<E, N: Number, U: Unsigned + ToOctree<E, N>, B: Bit> ToOctree<E, N> for UInt<U, B>
where
    UInt<U, B>: PowerOfTwo,
{
    type Octree = OctreeLevel<<U as ToOctree<E, N>>::Octree>;
}

type Octree<E, N, Diameter> = <Diameter as ToOctree<E, N>>::Octree;
