use super::octant::Octant;
use amethyst::core::nalgebra::{Point3, Scalar};
use num_traits::*;
use std::borrow::Borrow;
use std::rc::Rc;

#[derive(PartialEq)]
pub enum LevelData<E, O> {
    Node([Rc<O>; 8]),
    Leaf(Rc<E>),
    Empty,
}
impl<E, O> Clone for LevelData<E, O> {
    fn clone(&self) -> Self {
        use LevelData::*;
        match self {
            Node(ref nodes) => Node(nodes.clone()),
            Leaf(e) => Leaf(Rc::clone(e)),
            Empty => Empty,
        }
    }
}
#[derive(PartialEq)]
pub struct OctreeLevel<E, N, O>
where
    N: Scalar,
{
    data: LevelData<E, O>,
    bottom_left: Point3<N>,
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
impl<E, N: Scalar> Clone for OctreeBase<E, N> {
    fn clone(&self) -> Self {
        OctreeBase::new(self.data.clone(), self.bottom_left.clone())
    }
}
impl<E, N: Scalar, O: Clone> Clone for OctreeLevel<E, N, O> {
    fn clone(&self) -> Self {
        OctreeLevel::new(self.data.clone(), self.bottom_left.clone())
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

pub trait Diameter<N> {
    fn diameter() -> N;
}
impl<E, N, O: Diameter<N>> Diameter<N> for OctreeLevel<E, N, O>
where
    N: Number,
{
    fn diameter() -> N {
        O::diameter() << N::one()
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

pub trait HasPosition {
    type Position;

    fn position(&self) -> &Self::Position;
}
impl<E, N: Scalar, O> HasPosition for OctreeLevel<E, N, O> {
    type Position = Point3<N>;

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

pub trait HasData {
    type Data: Clone;
    fn data(&self) -> &Self::Data;
}
impl<E, N: Scalar, O> HasData for OctreeLevel<E, N, O> {
    type Data = LevelData<E, O>;

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
impl<E, O> Empty for LevelData<E, O> {
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
impl<E, O> Leaf<Rc<E>> for LevelData<E, O> {
    fn leaf(value: Rc<E>) -> Self {
        LevelData::Leaf(value)
    }

    fn is_leaf(&self) -> bool {
        match self {
            LevelData::Leaf(_) => true,
            _ => false,
        }
    }

    fn get_leaf(&self) -> &Rc<E> {
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
impl<E, N: Scalar, O> OctreeLevel<E, N, O> {
    fn with_data(&self, data: LevelData<E, O>) -> Self {
        OctreeLevel {
            data: data,
            ..(*self.clone())
        }
    }
}
impl<E, N: Scalar, O> OctreeLevel<E, N, O>
where
    N: Number,
    O: Insert<E, N> + New<N> + Diameter<N> + HasData,
    <O as HasData>::Data: PartialEq + Clone + Leaf<Rc<E>> + Empty,
{
    fn create_sub_nodes<P>(&self, pos: P, elem: Rc<E>, default: O::Data) -> Self
    where
        P: Borrow<Point3<N>>,
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
        .unwrap();
        self.with_data(Node(octree_nodes)).compress_nodes()
    }
}

pub trait Get<E, N: Scalar> {
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

pub trait Insert<E, N: Scalar> {
    fn insert<P, R>(&self, pos: P, elem: R) -> Self
    where
        P: Borrow<Point3<N>>,
        R: Into<Rc<E>>;
}
impl<E, N, O> Insert<E, N> for OctreeLevel<E, N, O>
where
    E: PartialEq,
    N: Number,
    O: Insert<E, N> + New<N> + Diameter<N> + HasData,
    <O as HasData>::Data: PartialEq + Leaf<Rc<E>> + Empty,
{
    fn insert<P, R>(&self, pos: P, elem: R) -> Self
    where
        P: Borrow<Point3<N>>,
        R: Into<Rc<E>>,
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
impl<E, N> Insert<E, N> for OctreeBase<E, N>
where
    N: Scalar,
{
    fn insert<P, R>(&self, pos: P, elem: R) -> Self
    where
        P: Borrow<Point3<N>>,
        R: Into<Rc<E>>,
    {
        OctreeBase::new(BaseData::leaf(elem.into()), pos.borrow().clone())
    }
}

pub trait New<N: Scalar>: HasData {
    fn new(data: Self::Data, bottom_left: Point3<N>) -> Self;
}
impl<E, N: Scalar> New<N> for OctreeBase<E, N> {
    fn new(data: Self::Data, bottom_left: Point3<N>) -> Self {
        OctreeBase { data, bottom_left }
    }
}
impl<E, N: Scalar, O> New<N> for OctreeLevel<E, N, O> {
    fn new(data: Self::Data, bottom_left: Point3<N>) -> Self {
        OctreeLevel { data, bottom_left }
    }
}

pub trait Compress {
    fn compress_nodes(self) -> Self;
}
impl<E, N: Scalar, O: HasData> Compress for OctreeLevel<E, N, O>
where
    <O as HasData>::Data: PartialEq + Clone + Leaf<Rc<E>> + Empty,
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

//impl<E, O> Into<LevelData<E, O>> for BaseData<E> {
//    fn into(self) -> LevelData<E, O> {
//        use BaseData::*;
//        match self {
//            Leaf(elem) => LevelData::leaf(elem),
//            Empty => LevelData::empty(),
//        }
//    }
//}
//impl<E, N, O> Into<LevelData<E, OctreeLevel<E, N, O>>> for LevelData<E, O>
//where
//    N: Scalar,
//    O: HasData + HasPosition<Position = Point3<N>>,
//    <O as HasData>::Data: Into<LevelData<E, O>>,
//{
//    fn into(self) -> LevelData<E, OctreeLevel<E, N, O>> {
//        use LevelData::*;
//        match self {
//            Empty => LevelData::empty(),
//            Leaf(elem) => LevelData::leaf(elem),
//            Node(nodes) => Node(array_init::array_init(|i| {
//                Rc::new(OctreeLevel::from(Rc::clone(&nodes[i])))
//            })),
//        }
//    }
//}

//impl<E, N, O> From<Rc<O>> for OctreeLevel<E, N, O>
//where
//    N: Scalar,
//    O: HasData + HasPosition<Position = Point3<N>>,
//    <O as HasData>::Data: Into<LevelData<E, O>> + Clone,
//{
//    fn from(lower: Rc<O>) -> Self {
//        OctreeLevel::new(lower.data().clone().into(), *lower.position())
//    }
//}

pub type Octree8<E, N> = OctreeLevel<
    E,
    N,
    OctreeLevel<
        E,
        N,
        OctreeLevel<
            E,
            N,
            OctreeLevel<
                E,
                N,
                OctreeLevel<E, N, OctreeLevel<E, N, OctreeLevel<E, N, OctreeBase<E, N>>>>,
            >,
        >,
    >,
>;
