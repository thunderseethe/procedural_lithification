//! Module to ease the use of OctreeLevel and OctreeBase
//! This module provides aliases to mask the verbosity of an Octrees type.
//!
//! Example:
//! ```
//! let octree = Octree::<isize, u32, U128>::at_origin(Some(-124));
//! ```
//!
//! This example will construct an Octree of size 128 (height 7) filled with the element -124.
extern crate typenum;

use crate::octree::new_octree::*;
use typenum::{Bit, PowerOfTwo, Shright, UInt, UTerm, Unsigned, B1, U256};

/// Trait implemented by UInt and UTerm to map them to their respective Octrees.
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
/// An octree of height 8 is common enough that it get it's own type alias
pub type Octree8<E, N> = Octree<E, N, U256>;
