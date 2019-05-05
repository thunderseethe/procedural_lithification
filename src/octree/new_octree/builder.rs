use crate::dimension::morton_code::MortonCode;
use crate::octree::new_octree::*;
use either::Either;
use itertools::Itertools;
use rayon::iter::plumbing::*;
use rayon::prelude::*;

trait RawTreeSize: ElementType + Diameter
where
    Self::Element: Clone,
{
    fn raw_tree() -> RawTree<Self::Element> {
        RawTree(vec![None; usize::pow(Self::diameter(), 3)])
    }
}
impl<T> RawTreeSize for T
where
    T: ElementType + Diameter,
    ElementOf<T>: Clone,
{
}

struct RawTree<E>(Vec<Option<E>>);

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

struct LeavesIterMut<'data, E> {
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

pub trait FromRawTree: ElementType + Sized {
    fn build_octree(
        data: &[Option<Self::Element>],
        morton_raw: usize,
    ) -> Either<Option<Self::Element>, Self>;
}

impl<E, N: Number> FromRawTree for OctreeBase<E, N> {
    fn build_octree(data: &[Option<E>], morton_raw: usize) -> Either<Option<E>, Self> {
        Either::Left(data[0])
    }
}

impl<O> FromRawTree for OctreeLevel<O>
where
    O: FromRawTree + OctreeTypes + Diameter + PartialEq + HasData + New,
    ElementOf<O>: PartialEq,
    DataOf<Self>: From<DataOf<O>>,
{
    fn build_octree(
        data: &[Option<ElementOf<O>>],
        morton_raw: usize,
    ) -> Either<Option<ElementOf<O>>, Self> {
        let segment_size = usize::pow(Self::diameter(), 3);
        let childrens = (0..7).map(|i| {
            let start = i * segment_size;
            let end = (i + 1) * segment_size;
            O::build_octree(&data[start..end], morton_raw + start)
        });
        let childrens: [Either<Option<ElementOf<O>>, O>; 8] = array_init::array_init(|i| {
            let start = i * segment_size;
            let end = (i + 1) * segment_size;
            O::build_octree(&data[start..end], morton_raw + start)
        });
        if childrens.iter().all_equal() {
            childrens[0].map_right(|lower| {
                Self::new(
                    lower.into_data().into(),
                    MortonCode::from_raw(morton_raw as u64).as_point().unwrap(),
                )
            })
        } else {
            let childs: [Ref<O>; 8] = array_init::array_init(|i| {
                Ref::new(
                    childrens[i]
                        .map_left(|option_e| {
                            O::new(
                                option_e
                                    .map(<O as HasData>::Data::leaf)
                                    .unwrap_or_else(<O as HasData>::Data::empty),
                                MortonCode::from_raw((morton_raw + segment_size * i) as u64)
                                    .as_point()
                                    .unwrap(),
                            )
                        })
                        .into_inner(),
                )
            });
            let point = MortonCode::from_raw(morton_raw as u64).as_point().unwrap();
            let octree = Self::new(LevelData::Node(childs), point);
            Either::Right(octree)
        }
    }
}

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

impl<'a, Octree> IntoParallelIterator for &'a mut OctreeBuilder<Octree>
where
    Octree: OctreeTypes,
    ElementOf<Octree>: Send,
    FieldOf<Octree>: Send,
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
            .map(|(indx, elem)| (MortonCode::from_raw(indx as u64).as_point().unwrap(), elem))
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
                Octree::new(
                    <Octree as HasData>::Data::from(option_elem),
                    Point3::origin(),
                )
            })
            .into_inner()
    }
}
