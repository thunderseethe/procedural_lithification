use crate::octree::new_octree::*;

impl<'a, O> IntoIterator for &'a OctreeLevel<O>
where
    O: OctreeTypes + IntoIterator<Item = Self::Item>,
{
    type Item = Octant<'a, <Self as ElementType>::Element, <Self as FieldType>::Field>;
    type IntoIter = OctantIter<impl Iterator<Item = Self::Item>>;

    fn into_iter(self) -> Self::IntoIter {
        use LevelData::*;
        match self {
            Empty => OctantIter {
                iterator: None.into_iter(),
            },
            Leaf(elem) => OctantIter {
                iterator: Some(Octant::new(
                    elem.as_ref(),
                    &self.bottom_left,
                    Self::diameter(),
                ))
                .into_iter(),
            },
            Node(nodes) => OctantIter {
                iterator: nodes.iter().flat_map(|node| node.as_ref().into_iter()),
            },
        }
    }
}
struct OctantIter<I> {
    iterator: I,
}
impl<I> Iterator for OctantIter<I>
where
    I: Iterator,
{
    type Item = <I as Iterator>::Item;

    fn next(&mut self) -> Option<Self::Item> {
        self.iterator.next()
    }
}

impl<'a, E, N: Number> IntoIterator for &'a OctreeBase<E, N> {
    type Item = Octant<'a, E, N>;
    type IntoIter = <Option<Octant<'a, E, N>> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.data
            .as_option()
            .map(|elem| Octant::new(elem.as_ref(), &self.bottom_left, Self::diameter()))
            .into_iter()
    }
}

struct Octant<'a, E, N: Scalar> {
    data: &'a E,
    bottom_left_front: &'a Point3<N>,
    diameter: usize,
}
impl<'a, E, N: Scalar> Octant<'a, E, N> {
    fn new(data: &'a E, bottom_left_front: &'a Point3<N>, diameter: usize) {
        Octant {
            bottom_left_front,
            diameter,
        }
    }
}

struct EmptyIterator<T> {
    _marker: std::marker::PhantomData<T>,
}
impl<T> Iterator for EmptyIterator<T> {
    type Item = T;

    fn next(&mut self) -> Option<Self::Item> {
        None
    }
}
