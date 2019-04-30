/// Module for the operations that can be performed on an Octree.
use super::{
    descriptors::{Empty, Leaf},
    BaseData, Diameter, FieldType, HasData, LevelData, Number, OctreeBase, OctreeLevel,
    OctreeTypes,
};
use amethyst::core::nalgebra::{Point3, Scalar};
use std::borrow::Borrow;
use std::rc::Rc;

pub trait Get: OctreeTypes {
    fn get<P>(&self, pos: P) -> Option<&Self::Element>
    where
        P: Borrow<Point3<Self::Field>>;
}
impl<O> Get for OctreeLevel<O>
where
    O: Get + Diameter,
{
    fn get<P>(&self, pos: P) -> Option<&Self::Element>
    where
        P: Borrow<Point3<Self::Field>>,
    {
        use super::LevelData::*;
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
impl<E, N> Get for OctreeBase<E, N>
where
    N: Number,
{
    fn get<P>(&self, _pos: P) -> Option<&E>
    where
        P: Borrow<Point3<N>>,
    {
        use super::BaseData::*;
        match self.data {
            Empty => None,
            Leaf(ref elem) => Some(elem.as_ref()),
        }
    }
}

pub trait Insert: OctreeTypes {
    fn insert<P, R>(&self, pos: P, elem: R) -> Self
    where
        P: Borrow<Point3<Self::Field>>,
        R: Into<Rc<Self::Element>>;
}
impl<O> Insert for OctreeLevel<O>
where
    O: Insert + New + Diameter + HasData,
    <O as HasData>::Data: PartialEq,
    Self::Element: PartialEq,
{
    fn insert<P, R>(&self, pos: P, elem: R) -> Self
    where
        P: Borrow<Point3<Self::Field>>,
        R: Into<Rc<Self::Element>>,
    {
        use super::LevelData::*;
        match &self.data {
            Empty => self.create_sub_nodes(pos, elem.into(), <O as HasData>::Data::empty()),
            Leaf(old_elem) => {
                let new_elem = elem.into();
                if old_elem == &new_elem {
                    self.with_data(Leaf(new_elem))
                } else {
                    self.create_sub_nodes(
                        pos,
                        new_elem,
                        <O as HasData>::Data::leaf(Rc::clone(old_elem)),
                    )
                }
            }
            Node(ref old_nodes) => {
                let mut nodes = old_nodes.clone();
                let index: usize = self.get_octant_index(pos.borrow());
                let old_octant: &Rc<O> = &old_nodes[index];
                nodes[index] = Rc::new(old_octant.insert(pos, elem));
                self.with_data(Node(nodes)).compress_nodes()
            }
        }
    }
}
impl<E, N> Insert for OctreeBase<E, N>
where
    N: Number,
{
    fn insert<P, R>(&self, pos: P, elem: R) -> Self
    where
        P: Borrow<Point3<N>>,
        R: Into<Rc<E>>,
    {
        OctreeBase::new(BaseData::leaf(elem.into()), pos.borrow().clone())
    }
}

pub trait New: HasData + FieldType {
    fn new(data: Self::Data, bottom_left: Point3<Self::Field>) -> Self;
}
impl<E, N: Number> New for OctreeBase<E, N> {
    fn new(data: Self::Data, bottom_left: Point3<N>) -> Self {
        OctreeBase { data, bottom_left }
    }
}
impl<O: OctreeTypes> New for OctreeLevel<O> {
    fn new(data: Self::Data, bottom_left: Point3<Self::Field>) -> Self {
        OctreeLevel { data, bottom_left }
    }
}

pub trait Compress {
    fn compress_nodes(self) -> Self;
}
impl<O> Compress for OctreeLevel<O>
where
    O: HasData + OctreeTypes,
    <O as HasData>::Data: PartialEq,
{
    fn compress_nodes(self) -> Self {
        use super::LevelData::*;
        match self.data {
            Node(ref nodes) => {
                let mut iter = nodes.iter().map(|node| node.data());
                if iter.next().map_or(true, |head| iter.all(|ele| head == ele)) {
                    let head = nodes[0].data();
                    self.with_data(if head.is_empty() {
                        LevelData::empty()
                    } else if head.is_leaf() {
                        LevelData::leaf(Rc::clone(head.get_leaf()))
                    } else {
                        panic!("Attempted to compress Node(..) node which should be impossible.");
                    })
                } else {
                    self
                }
            }
            _ => self,
        }
    }
}
impl<E, N: Scalar> Compress for OctreeBase<E, N> {
    // Compress is the identity function for BaseNodes
    fn compress_nodes(self) -> Self {
        self
    }
}
