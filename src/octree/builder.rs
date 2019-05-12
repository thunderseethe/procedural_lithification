use crate::iter_tools::all_equal;
use crate::octree::*;
use either::Either;
use num_traits::AsPrimitive;
use morton_code::{IntoBytes, LUTType, MortonCode, MortonStorage};
use rayon::iter::plumbing::*;
use rayon::prelude::*;

/// Construct an Octree from a flat array of leaves.
pub trait FromRawTree: ElementType + Sized {
    // We return an either to essentially constructing a tree until we actually have 8 different children.
    // This avoids allocating (A)Rcs that immediately get deallocated which is slow
    fn build_octree(
        data: &[Option<Self::Element>],
        morton_raw: usize,
    ) -> Either<Option<Self::Element>, Self>;
}

impl<E, N: Number> FromRawTree for OctreeBase<E, N>
where
    E: Copy,
{
    // All our "children" are the same here so we always return Left with our datum.
    fn build_octree(data: &[Option<E>], _morton_raw: usize) -> Either<Option<E>, Self> {
        Either::Left(data[0])
    }
}

impl<O> FromRawTree for OctreeLevel<O>
where
    O: FromRawTree + Clone + OctreeTypes + Diameter + PartialEq + HasData + New,
    ElementOf<O>: Clone + PartialEq,
    FieldOf<O>: IntoBytes + MortonStorage,
    DataOf<Self>: From<DataOf<O>>,
    LUTType: AsPrimitive<FieldOf<O>> + AsPrimitive<<FieldOf<O> as MortonStorage>::Storage>,
    usize: AsPrimitive<<FieldOf<O> as MortonStorage>::Storage>,
{
    fn build_octree(
        data: &[Option<ElementOf<O>>],
        morton_raw: usize,
    ) -> Either<Option<ElementOf<O>>, Self> {
        // Segment size is the volume of the cube our octant covers
        let segment_size = usize::pow(O::DIAMETER, 3);
        // Determine slice of the leaves for each child and recurse into their build_octree() method
        let mut childrens = (0..8).map(|i| {
            let start = i * segment_size;
            let end = (i + 1) * segment_size;
            O::build_octree(&data[start..end], morton_raw + start)
        });
        // If all our children are equal we don't want to construct an octree and instead defer up the call stack
        if all_equal(childrens.clone()) {
            childrens.next().unwrap().map_right(|lower| {
                // This code generally won't be run but in the case we have 8 equal Either::Rights combine there data to construct an Octree that's one level higher
                Self::new(
                    lower.into_data().into(),
                    MortonCode::from_raw(morton_raw.as_()).into(),
                )
            })
        } else {
            // Here our children we're different so we have to construct a new octree
            let childs: [Ref<O>; 8] =
                array_init::from_iter(childrens.enumerate().map(|(i, either)| {
                    Ref::new(
                        either
                            .map_left(|option_e| {
                                O::new(
                                    option_e
                                        .map(<O as HasData>::Data::leaf)
                                        .unwrap_or_else(<O as HasData>::Data::empty),
                                    MortonCode::from_raw((morton_raw + segment_size * i).as_())
                                        .into(),
                                )
                            })
                            .into_inner(),
                    )
                }))
                .expect("Failed to construct array from children iterator in build_octree");
            let point = MortonCode::from_raw(morton_raw.as_()).into();
            let octree = Self::new(LevelData::Node(childs), point);
            Either::Right(octree)
        }
    }
}

/// Behavior of a type that can be built
/// Includes convenience method create_builder() which makes a builder from an instance of type instead of statically referencing type.
pub trait Builder {
    type Builder;

    fn builder() -> Self::Builder;
    fn create_builder(&self) -> Self::Builder {
        Self::builder()
    }
}

impl<O> Builder for OctreeLevel<O>
where
    O: OctreeTypes + Builder + Diameter + RawTreeSize,
    ElementOf<O>: Clone,
{
    type Builder = OctreeBuilder<Self>;

    fn builder() -> Self::Builder {
        OctreeBuilder {
            data: Self::raw_tree(),
            _marker: std::marker::PhantomData,
        }
    }
}
impl<E, N: Number> Builder for OctreeBase<E, N>
where
    E: Clone,
{
    type Builder = OctreeBuilder<Self>;

    fn builder() -> Self::Builder {
        OctreeBuilder {
            data: Self::raw_tree(),
            _marker: std::marker::PhantomData,
        }
    }
}

pub struct OctreeBuilder<Octree: ElementType> {
    data: RawTree<ElementOf<Octree>>,
    _marker: std::marker::PhantomData<Octree>,
}

/// Iteration over an OctreeBuilder defers to LeavesIterMut to handle iterating over the actual array and converts array index to a point via MortonCode
impl<'a, Octree> IntoParallelIterator for &'a mut OctreeBuilder<Octree>
where
    Octree: OctreeTypes,
    ElementOf<Octree>: Send,
    FieldOf<Octree>: Send + IntoBytes + MortonStorage,
    LUTType:
        AsPrimitive<FieldOf<Octree>> + AsPrimitive<<FieldOf<Octree> as MortonStorage>::Storage>,
    usize: AsPrimitive<<FieldOf<Octree> as MortonStorage>::Storage>,
{
    type Item = (Point3<FieldOf<Octree>>, &'a mut Option<ElementOf<Octree>>);
    type Iter = rayon::iter::Map<
        rayon::iter::Enumerate<LeavesIterMut<'a, ElementOf<Octree>>>,
        fn(
            (usize, &'a mut Option<ElementOf<Octree>>),
        ) -> (Point3<FieldOf<Octree>>, &'a mut Option<ElementOf<Octree>>),
    >;

    fn into_par_iter(self) -> Self::Iter {
        self.data
            .into_par_iter()
            .enumerate()
            .map(|(indx, elem)| (MortonCode::from_raw(indx.as_()).into(), elem))
    }
}

impl<Octree> OctreeBuilder<Octree>
where
    Octree: FromRawTree + New + OctreeTypes,
    DataOf<Octree>: From<Option<ElementOf<Octree>>>,
{
    pub fn build(self) -> Octree {
        Octree::build_octree(&self.data.0[..], 0)
            .map_left(|option_elem| {
                // If by some act of god we still have a Left at the end of this shit we can build a tree of a single Leaf or Empty node
                // If this code gets run be thankful for the blessings the heavens have bestowed upon you.
                Octree::new(
                    <Octree as HasData>::Data::from(option_elem),
                    Point3::origin(),
                )
            })
            .into_inner()
    }
}

/// Determines the size of Vector that will hold all possbile base leaves of Self
/// This will be Self::diamter() ^ 3 for anything with a diameter.
pub trait RawTreeSize: ElementType + Diameter
where
    Self::Element: Clone,
{
    fn raw_tree() -> RawTree<Self::Element> {
        RawTree(vec![None; usize::pow(Self::DIAMETER, 3)])
    }
}
impl<T> RawTreeSize for T
where
    T: ElementType + Diameter,
    ElementOf<T>: Clone,
{
}

pub struct RawTree<E>(Vec<Option<E>>);
impl<'data, E: Send> IntoParallelIterator for &'data mut RawTree<E> {
    type Item = &'data mut Option<E>;
    type Iter = LeavesIterMut<'data, E>;

    fn into_par_iter(self) -> Self::Iter {
        let len = self.0.len();
        LeavesIterMut {
            slice: &mut self.0[..],
            len,
        }
    }
}

pub struct LeavesIterMut<'data, E> {
    slice: &'data mut [Option<E>],
    len: usize,
}
impl<'data, E: Send> ParallelIterator for LeavesIterMut<'data, E> {
    type Item = &'data mut Option<E>;

    fn drive_unindexed<C>(self, consumer: C) -> C::Result
    where
        C: UnindexedConsumer<Self::Item>,
    {
        bridge(self, consumer)
    }
}
impl<'data, E: Send> IndexedParallelIterator for LeavesIterMut<'data, E> {
    fn len(&self) -> usize {
        self.len
    }

    fn drive<C>(self, consumer: C) -> C::Result
    where
        C: Consumer<Self::Item>,
    {
        bridge(self, consumer)
    }

    fn with_producer<CB>(self, callback: CB) -> CB::Output
    where
        CB: ProducerCallback<Self::Item>,
    {
        callback.callback(SliceProducer { slice: self.slice })
    }
}

struct SliceProducer<'a, T> {
    slice: &'a mut [T],
}
impl<'a, T: Send> Producer for SliceProducer<'a, T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;

    fn into_iter(self) -> Self::IntoIter {
        self.slice.iter_mut()
    }

    fn split_at(self, index: usize) -> (Self, Self) {
        let (left, right) = self.slice.split_at_mut(index);
        (
            SliceProducer { slice: left },
            SliceProducer { slice: right },
        )
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::octree::Octree;
    use typenum::*;

    #[test]
    fn test_raw_tree_size_matches_octree_size() {
        let raw_tree = Octree::<u32, u8, U64>::raw_tree();

        assert_eq!(raw_tree.0.len(), 262144);
    }

    #[test]
    fn test_builder_uses_expected_raw_tree_for_octree() {
        let builder = Octree::<u32, u8, U128>::builder();

        assert_eq!(builder.data.0.len(), 2097152);
    }
}
