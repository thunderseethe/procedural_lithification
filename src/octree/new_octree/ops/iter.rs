use crate::octree::new_octree::*;
use crate::octree::octant::Octant;

impl<'a, O> IntoIterator for &'a OctreeLevel<O>
where
    O: OctreeTypes + Diameter,
    &'a O: IntoIterator<Item = Octant<&'a O::Element, &'a Point3<O::Field>>>,
{
    type Item = <&'a O as IntoIterator>::Item;
    type IntoIter = OctantIter<
        Self::Item,
        std::slice::Iter<'a, Ref<O>>,
        <&'a O as IntoIterator>::IntoIter,
        fn(&'a Ref<O>) -> <&'a O as IntoIterator>::IntoIter,
    >;

    fn into_iter(self) -> Self::IntoIter {
        use LevelData::*;
        match &self.data {
            Empty => OctantIter::Leaf(None.into_iter()),
            Leaf(ref elem) => OctantIter::Leaf(
                Some(Octant::new(elem, &self.bottom_left, Self::diameter())).into_iter(),
            ),
            Node(ref nodes) => OctantIter::Nodes(nodes.iter().flat_map(
                (|node| node.as_ref().into_iter())
                    as fn(&'a Ref<O>) -> <&'a O as IntoIterator>::IntoIter,
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

impl<'a, E: Clone, N: Number> IntoIterator for OctreeBase<E, N> {
    type Item = Octant<E, Point3<N>>;
    type IntoIter = std::option::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.data
            .as_option()
            .map(|elem| Octant::new(elem.clone(), self.bottom_left.clone(), Self::diameter()))
            .into_iter()
    }
}
impl<'a, E, N: Number> IntoIterator for &'a OctreeBase<E, N> {
    type Item = Octant<&'a E, &'a Point3<N>>;
    type IntoIter = std::option::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.data
            .as_option()
            .map(|elem| Octant::new(elem, &self.bottom_left, Self::diameter()))
            .into_iter()
    }
}
