use super::octant::Octant;
use amethyst::core::nalgebra::{Point3, Scalar};
use num_traits::*;
use std::borrow::Borrow;
use std::rc::Rc;

pub enum LevelData<O>
where
    O: OctreeTypes,
{
    Node([Rc<O>; 8]),
    Leaf(Rc<O::Element>),
    Empty,
}
impl<O> Clone for LevelData<O>
where
    O: OctreeTypes,
{
    fn clone(&self) -> Self {
        use LevelData::*;
        match self {
            Node(ref nodes) => Node(nodes.clone()),
            Leaf(e) => Leaf(Rc::clone(e)),
            Empty => Empty,
        }
    }
}
impl<O> PartialEq for LevelData<O>
where
    O: OctreeTypes + PartialEq,
    <O as ElementType>::Element: PartialEq,
{
    fn eq(&self, other: &LevelData<O>) -> bool {
        use LevelData::*;
        match (self, other) {
            (Node(node_a), Node(node_b)) => node_a == node_b,
            (Leaf(elem_a), Leaf(elem_b)) => elem_a == elem_b,
            (Empty, Empty) => true,
            _ => false,
        }
    }
}

pub struct OctreeLevel<O>
where
    O: OctreeTypes,
{
    data: LevelData<O>,
    bottom_left: Point3<O::Field>,
}
impl<O> PartialEq for OctreeLevel<O>
where
    O: OctreeTypes + PartialEq,
    <O as ElementType>::Element: PartialEq,
{
    fn eq(&self, other: &OctreeLevel<O>) -> bool {
        self.bottom_left.eq(&other.bottom_left) && self.data.eq(&other.data)
    }
}
impl<O> Clone for OctreeLevel<O>
where
    O: OctreeTypes + Clone,
{
    fn clone(&self) -> Self {
        OctreeLevel::new(self.data.clone(), self.bottom_left.clone())
    }
}

#[derive(PartialEq)]
pub enum BaseData<E> {
    Leaf(Rc<E>),
    Empty,
}
impl<E> Clone for BaseData<E> {
    fn clone(&self) -> Self {
        use BaseData::*;
        match self {
            Leaf(e) => Leaf(Rc::clone(e)),
            Empty => Empty,
        }
    }
}
#[derive(PartialEq)]
pub struct OctreeBase<E, N: Scalar> {
    data: BaseData<E>,
    bottom_left: Point3<N>,
}
impl<E, N: Number> Clone for OctreeBase<E, N> {
    fn clone(&self) -> Self {
        OctreeBase::new(self.data.clone(), self.bottom_left.clone())
    }
}

pub trait Number:
    Scalar + Num + PartialOrd + Shr<Self, Output = Self> + Shl<Self, Output = Self>
{
}
impl<T> Number for T where
    T: Scalar + Num + PartialOrd + Shr<Self, Output = Self> + Shl<Self, Output = Self>
{
}

// Hello, it's your good pal bottom up recursion. Now with types
pub trait ElementType {
    type Element;
}
pub trait FieldType {
    type Field: Number;
}

impl<E, N: Scalar> ElementType for OctreeBase<E, N> {
    type Element = E;
}
impl<E, N: Number> FieldType for OctreeBase<E, N> {
    type Field = N;
}

impl<O: OctreeTypes> ElementType for OctreeLevel<O> {
    type Element = O::Element;
}
impl<O: OctreeTypes> FieldType for OctreeLevel<O> {
    type Field = O::Field;
}

// Convenience wrapper to avoid busting my + key
pub trait OctreeTypes: ElementType + FieldType {}
impl<T> OctreeTypes for T where T: ElementType + FieldType {}

pub trait Diameter: FieldType {
    fn diameter() -> Self::Field;
}
impl<O> Diameter for OctreeLevel<O>
where
    O: Diameter + OctreeTypes,
{
    fn diameter() -> Self::Field {
        O::diameter() << Self::Field::one()
    }
}
impl<E, N> Diameter for OctreeBase<E, N>
where
    N: Number,
{
    fn diameter() -> N {
        N::one()
    }
}

pub trait HasPosition {
    type Position;

    fn position(&self) -> &Self::Position;
}
impl<O> HasPosition for OctreeLevel<O>
where
    O: OctreeTypes,
{
    type Position = Point3<<Self as FieldType>::Field>;

    fn position(&self) -> &Self::Position {
        &self.bottom_left
    }
}
impl<E, N: Scalar> HasPosition for OctreeBase<E, N> {
    type Position = Point3<N>;

    fn position(&self) -> &Self::Position {
        &self.bottom_left
    }
}

pub trait HasData: ElementType {
    type Data: Clone + Leaf<Rc<Self::Element>> + Empty;
    fn data(&self) -> &Self::Data;
}
impl<O> HasData for OctreeLevel<O>
where
    O: OctreeTypes,
{
    type Data = LevelData<O>;

    fn data(&self) -> &Self::Data {
        &self.data
    }
}
impl<E, N: Scalar> HasData for OctreeBase<E, N> {
    type Data = BaseData<E>;

    fn data(&self) -> &Self::Data {
        &self.data
    }
}

pub trait Empty {
    fn empty() -> Self;
    fn is_empty(&self) -> bool;
}
impl<O> Empty for LevelData<O>
where
    O: OctreeTypes,
{
    fn empty() -> Self {
        LevelData::Empty
    }

    fn is_empty(&self) -> bool {
        match self {
            LevelData::Empty => true,
            _ => false,
        }
    }
}
impl<E> Empty for BaseData<E> {
    fn empty() -> Self {
        BaseData::Empty
    }

    fn is_empty(&self) -> bool {
        match self {
            BaseData::Empty => true,
            _ => false,
        }
    }
}

pub trait Leaf<T> {
    fn leaf(value: T) -> Self;
    fn is_leaf(&self) -> bool;
    fn get_leaf(&self) -> &T;
}
impl<O> Leaf<Rc<O::Element>> for LevelData<O>
where
    O: OctreeTypes,
{
    fn leaf(value: Rc<O::Element>) -> Self {
        LevelData::Leaf(value)
    }

    fn is_leaf(&self) -> bool {
        match self {
            LevelData::Leaf(_) => true,
            _ => false,
        }
    }

    fn get_leaf(&self) -> &Rc<O::Element> {
        match self {
            LevelData::Leaf(ref e) => e,
            _ => panic!("Called get_leaf() on non leaf node."),
        }
    }
}
impl<E> Leaf<Rc<E>> for BaseData<E> {
    fn leaf(value: Rc<E>) -> Self {
        BaseData::Leaf(value)
    }

    fn is_leaf(&self) -> bool {
        match self {
            BaseData::Leaf(_) => true,
            _ => false,
        }
    }

    fn get_leaf(&self) -> &Rc<E> {
        match self {
            BaseData::Leaf(ref e) => e,
            _ => panic!("Called get_leaf() on non leaf node."),
        }
    }
}

use std::ops::*;
impl<O> OctreeLevel<O>
where
    O: Diameter + OctreeTypes,
{
    fn get_octant_index<P>(&self, pos: P) -> usize
    where
        P: Borrow<Point3<<Self as FieldType>::Field>>,
    {
        self.get_octant(pos).to_usize().unwrap()
    }

    fn get_octant<P>(&self, pos_ref: P) -> Octant
    where
        P: Borrow<Point3<<Self as FieldType>::Field>>,
    {
        use crate::octree::octant::Octant::*;
        let pos = pos_ref.borrow();
        let r = Self::diameter() >> <Self as FieldType>::Field::one();
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
impl<O: OctreeTypes> OctreeLevel<O> {
    fn with_data(&self, data: LevelData<O>) -> Self {
        OctreeLevel {
            data: data,
            ..(*self.clone())
        }
    }
}
impl<O> OctreeLevel<O>
where
    O: Insert + New + Diameter + HasData,
    <O as HasData>::Data: PartialEq,
{
    fn create_sub_nodes<P>(
        &self,
        pos: P,
        elem: Rc<<Self as ElementType>::Element>,
        default: O::Data,
    ) -> Self
    where
        P: Borrow<Point3<<Self as FieldType>::Field>>,
    {
        use crate::octree::octant::OctantIter;
        use LevelData::Node;
        let modified_octant = self.get_octant(pos.borrow());
        let octree_nodes: [Rc<O>; 8] = array_init::from_iter(OctantIter::default().map(|octant| {
            let data = default.clone();
            let sub_bottom_left = octant.sub_octant_bottom_left(self.bottom_left, O::diameter());
            let octree = O::new(data, sub_bottom_left);
            let octree = if modified_octant == octant {
                octree.insert(pos.borrow(), elem.clone())
            } else {
                octree
            };
            Rc::new(octree)
        }))
        .expect("Failed to construct array from iterator");
        self.with_data(Node(octree_nodes)).compress_nodes()
    }
}

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
impl<E, N> Get for OctreeBase<E, N>
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
        use LevelData::*;
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
        use LevelData::*;
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

pub type Octree8<E, N> = OctreeBase<E, N>;
