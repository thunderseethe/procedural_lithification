extern crate typenum;

use crate::octree::new_octree::*;
use typenum::{Bit, PowerOfTwo, Shright, UInt, UTerm, Unsigned, B1, U256};

pub trait ToOctree<E, N> {
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

pub type Octree<E, N, Diameter> = <Shright<Diameter, B1> as ToOctree<E, N>>::Octree;
pub type Octree8<E, N> = Octree<E, N, U256>;
