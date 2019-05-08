use crate::octree::new_octree::*;
use crate::octree::octant::Octant;

// Iteration over an OctreeLevel happens in two ways.
// In the trivial case we have an Empty or Leaf node and we iterate over std::option::IntoIter<E>
// For the Node case we convert each sub node to an iterator and them combine them via flat_map() and store that in our OctantIter
// This makes for a very verbose OctantIter type definition due to the type parameters required by std::iter::FlatMap however the actual logic is straightforward.
impl<'a, O> IntoIterator for &'a OctreeLevel<O>
where
    O: OctreeTypes + Diameter + HasPosition,
    //&'a O: IntoIterator<Item = Octant<&'a O::Element, &'a O::Position>>,
    &'a O: IntoIterator<Item = Octant<&'a ElementOf<O>, &'a Point3<FieldOf<O>>>>
        + Diameter
        + OctreeTypes,
{
    type Item = Octant<&'a ElementOf<O>, &'a Point3<FieldOf<O>>>;
    type IntoIter = OctantIter<
        Self::Item,
        std::slice::Iter<'a, Ref<O>>,
        <&'a O as IntoIterator>::IntoIter,
        fn(&'a Ref<O>) -> <&'a O as IntoIterator>::IntoIter,
    >;

    fn into_iter(self) -> Self::IntoIter {
        use LevelData::*;
        match &self.data {
            // Trivial cases map Leaf => Some and Empty => None and convert to iterator
            Empty => OctantIter::Leaf(None.into_iter()),
            Leaf(ref elem) => OctantIter::Leaf(
                Some(Octant::new(elem, &self.bottom_left, Self::diameter())).into_iter(),
            ),
            // In this case we map over our children and convert each one to an iterator and then combine them all with flat_map
            Node(ref nodes) => OctantIter::Nodes(nodes.iter().flat_map(
                (|node| node.as_ref().into_iter())
                    as fn(&'a Ref<O>) -> <&'a O as IntoIterator>::IntoIter,
            )),
        }
    }
}

/// An enum to abstract iteration over either an Option iterator or a flatmap iterator
/// It's iterator implementation delegates to whichever iterator it contains to actually produce values.
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

// Since we have no Node variant our Iter is always std::option::IntoIter
impl<'a, E: Clone, N: Number> IntoIterator for OctreeBase<E, N> {
    type Item = Octant<E, Point3<N>>;
    type IntoIter = std::option::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.data
            .as_ref()
            .map(|elem| Octant::new(elem.clone(), self.bottom_left.clone(), Self::diameter()))
            .into_iter()
    }
}
impl<'a, E, N: Number> IntoIterator for &'a OctreeBase<E, N> {
    type Item = Octant<&'a E, &'a Point3<N>>;
    type IntoIter = std::option::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        self.data
            .as_ref()
            .map(|elem| Octant::new(elem, &self.bottom_left, Self::diameter()))
            .into_iter()
    }
}
