use crate::octree::new_octree::*;

impl<'a, O> IntoIterator for &'a OctreeLevel<O>
where
    O: OctreeTypes + Diameter,
    &'a O: IntoIterator<Item = Octant<O::Element, O::Field>>,
{
    type Item = <&'a O as IntoIterator>::Item;
    type IntoIter = OctantIter<
        Octant<O::Element, O::Field>,
        std::slice::Iter<'a, Rc<O>>,
        <&'a O as IntoIterator>::IntoIter,
        fn(&'a Rc<O>) -> <&'a O as IntoIterator>::IntoIter,
    >;

    fn into_iter(self) -> Self::IntoIter {
        use LevelData::*;
        match &self.data {
            Empty => OctantIter::Leaf(None.into_iter()),
            Leaf(ref elem) => OctantIter::Leaf(
                Some(Octant::new(
                    Ref::clone(&elem),
                    self.bottom_left.clone(),
                    Self::diameter(),
                ))
                .into_iter(),
            ),
            Node(ref nodes) => OctantIter::Nodes(nodes.iter().flat_map(
                (|node| node.as_ref().into_iter())
                    as fn(&'a Rc<O>) -> <&'a O as IntoIterator>::IntoIter,
            )),
        }
    }
}

pub enum OctantIter<E, I, U, F>
where
    U: IntoIterator,
{
    Leaf(std::option::IntoIter<E>),
    Nodes(std::iter::FlatMap<I, U, F>),
}
impl<E, I, U, F> Iterator for OctantIter<E, I, U, F>
where
    I: Iterator,
    U: IntoIterator<Item = E>,
    F: FnMut(<I as Iterator>::Item) -> U,
{
    type Item = E;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            OctantIter::Leaf(iter) => iter.next(),
            OctantIter::Nodes(iter) => iter.next(),
        }
    }
}

impl<'a, E, N: Number> IntoIterator for OctreeBase<E, N> {
    type Item = Octant<E, N>;
    type IntoIter = std::option::IntoIter<Octant<E, N>>;

    fn into_iter(self) -> Self::IntoIter {
        self.data
            .as_option()
            .map(|elem| Octant::new(Ref::clone(elem), self.bottom_left.clone(), Self::diameter()))
            .into_iter()
    }
}

pub struct Octant<E, N: Scalar> {
    data: Ref<E>,
    bottom_left_front: Point3<N>,
    diameter: usize,
}
impl<E, N: Scalar> Octant<E, N> {
    fn new(data: Ref<E>, bottom_left_front: Point3<N>, diameter: usize) -> Self {
        Octant {
            data,
            bottom_left_front,
            diameter,
        }
    }
}
