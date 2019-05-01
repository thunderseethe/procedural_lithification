use crate::octree::new_octree::*;

impl<'a, O> IntoIterator for &'a OctreeLevel<O> 
where
    O: OctreeTypes + IntoIterator<Item=Self::Item>,
{
    type Item = Octant<'a, Self::Element, Self::Field>;
    type IntoIter = impl Iterator<Item=Self::Item>;
    
    fn into_iter(self) -> Self::IntoIter {
        use LevelData::*;
        match
    }
}

impl<E, N: Number> IntoIterator for &'a OctreeBase<E, N> 
{
    type Item = Octant<'a, E, N>;
    type IntoIter = <Option<Octant<'a, E, N>> as IntoIterator>::IntoIterator;

    fn into_iter(self) -> Self::IntoIter {
        self.data.as_option().map(|elem| {
            Octant::new(elem.as_ref(), &self.bottom_left, Self::diameter())
        }).into_iter()
    }
}

struct Octant<'a, E, N: Scalar> {
    data: &'a E,
    bottom_left_front: &'a Point3<N>,
    diameter: usize,
}
impl<'a, E, N: Scalar> OctantCoordinates<'a, E, N> {
    fn new(data: &'a E, bottom_left_front: &'a Point3<N>, diameter: usize) {
        OctantCoordinates { bottom_left_front, diameter }
    }
}