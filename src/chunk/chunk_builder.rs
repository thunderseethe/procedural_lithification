use super::HasOctree;
use crate::chunk::{Chunk, OctreeOf};
use crate::octree::new_octree::builder::*;
use crate::octree::new_octree::*;
use crate::octree::Octree;
use amethyst::core::nalgebra::Point3;
use either::Either;
use rayon::iter::{plumbing::*, *};
use std::sync::Arc;

//trait ToChunkBuilder {
//type Output;
//}
//impl<O> ToChunkBuilder for OctreeLevel<O>
//where
//O: OctreeTypes + ToChunkBuilder,
//{
//type Output = RawNode<<O as ToChunkBuilder>::Output>;
//}
//impl<E, N> ToChunkBuilder for OctreeBase<E, N>
//where
//N: Number,
//{
//type Output = RawLeaf;
//}

//struct RawTree(Box<[Option<Block>; 16777216]>);
//impl RawTree {
//pub fn new() -> Self {
//let mut v: Vec<Option<Block>> = vec![None; 16777216];
//unsafe {
//let ptr = v.as_mut_ptr();
//std::mem::forget(v);
//RawTree(Box::from_raw(ptr as *mut [Option<Block>; 16777216]))
//}
//}
//}
//impl<'data> IntoParallelIterator for &'data mut RawTree {
//type Item = &'data mut Option<Block>;
//type Iter = LeavesIterMut<'data>;

//fn into_par_iter(self) -> Self::Iter {
//LeavesIterMut {
//slice: &mut self.0[..],
//}
//}
//}

//struct LeavesIterMut<'data> {
//slice: &'data mut [Option<Block>],
//}
//impl<'data> ParallelIterator for LeavesIterMut<'data> {
//type Item = &'data mut Option<Block>;

//fn drive_unindexed<C>(self, consumer: C) -> C::Result
//where
//C: UnindexedConsumer<Self::Item>,
//{
//bridge(self, consumer)
//}
//}
//impl<'data> IndexedParallelIterator for LeavesIterMut<'data> {
//fn len(&self) -> usize {
//16777216
//}

//fn drive<C>(self, consumer: C) -> C::Result
//where
//C: Consumer<Self::Item>,
//{
//bridge(self, consumer)
//}

//fn with_producer<CB>(self, callback: CB) -> CB::Output
//where
//CB: ProducerCallback<Self::Item>,
//{
//callback.callback(SliceProducer { slice: self.slice })
//}
//}

//struct SliceProducer<'a, T> {
//slice: &'a mut [T],
//}
//impl<'a, T: Send> Producer for SliceProducer<'a, T> {
//type Item = &'a mut T;
//type IntoIter = std::slice::IterMut<'a, T>;

//fn into_iter(self) -> Self::IntoIter {
//self.slice.iter_mut()
//}

//fn split_at(self, index: usize) -> (Self, Self) {
//let (left, right) = self.slice.split_at_mut(index);
//(
//SliceProducer { slice: left },
//SliceProducer { slice: right },
//)
//}
//}

//trait BuildOctree {
//fn build_octree(
//data: &[Option<Block>; 16777216],
//start: usize,
//end: usize,
//) -> Either<Option<Block>, Octree<Block>>;

//fn height() -> u32;
//fn segment_size() -> usize;
//}
//struct RawNode<T> {
//_marker: std::marker::PhantomData<T>,
//}
//struct RawLeaf;

//impl BuildOctree for RawLeaf {
//fn build_octree(
//data: &[Option<Block>; 16777216],
//start: usize,
//end: usize,
//) -> Either<Option<Block>, Octree<Block>> {
//// Our leaf only covers one index.
//assert_eq!(
//end - start,
//1,
//"Height {} covers {} to {}",
//Self::height(),
//start,
//end
//);
//Either::Left(data[start])
//}

//fn height() -> u32 {
//0
//}

//fn segment_size() -> usize {
//1
//}
//}

//macro_rules! create_octree {
//($t: ident, $either:ident, $raw:expr) => {{
//let point = MortonCode::from_raw(($raw) as u64).as_point().unwrap();
//Arc::new(
//$either
//.map_left(|option_block| Octree::new(point, option_block, $t::height()))
//.into_inner(),
//)
//}};
//}

//impl<T: BuildOctree> BuildOctree for RawNode<T> {
//fn build_octree(
//data: &[Option<Block>; 16777216],
//start: usize,
//end: usize,
//) -> Either<Option<Block>, Octree<Block>> {
//let segment = T::segment_size();
//let (a, b, c, d, e, f, g, h) = (
//T::build_octree(data, start, start + segment),
//T::build_octree(data, start + segment, start + (segment * 2)),
//T::build_octree(data, start + (segment * 2), start + (segment * 3)),
//T::build_octree(data, start + (segment * 3), start + (segment * 4)),
//T::build_octree(data, start + (segment * 4), start + (segment * 5)),
//T::build_octree(data, start + (segment * 5), start + (segment * 6)),
//T::build_octree(data, start + (segment * 6), start + (segment * 7)),
//T::build_octree(data, start + (segment * 7), end),
//);
//if a == b && a == c && a == d && a == e && a == f && a == g && a == h {
//a.map_right(|octree| octree.set_height(Self::height()))
//} else {
//let childs = [
//create_octree!(T, a, start),
//create_octree!(T, b, start + segment),
//create_octree!(T, c, start + (2 * segment)),
//create_octree!(T, d, start + (3 * segment)),
//create_octree!(T, e, start + (4 * segment)),
//create_octree!(T, f, start + (5 * segment)),
//create_octree!(T, g, start + (6 * segment)),
//create_octree!(T, h, start + (7 * segment)),
//];
//let point = MortonCode::from_raw(start as u64).as_point().unwrap();
//let octree = Octree::with_children(childs, point, Self::height());
//Either::Right(octree)
//}
//}

//fn height() -> u32 {
//T::height() + 1
//}

//fn segment_size() -> usize {
//T::segment_size() * 8
//}
//}

//pub struct ChunkBuilder {
//pos: Point3<i32>,
//tree: RawTree,
//}

////type OctreeBuilder =
////    RawNode<RawNode<RawNode<RawNode<RawNode<RawNode<RawNode<RawNode<RawLeaf>>>>>>>>;
//type OctreeBuilder = <OctreeOf<Chunk> as ToChunkBuilder>::Output;
//impl ChunkBuilder {
//pub fn new(pos: Point3<i32>) -> Self {
//ChunkBuilder {
//pos,
//tree: RawTree::new(),
//}
//}

//pub fn par_iter_mut<'a>(
//&'a mut self,
//) -> impl ParallelIterator<Item = (Point3<u8>, &'a mut Option<Block>)> {
//(&mut self.tree)
//.into_par_iter()
//.enumerate()
//.map(|(indx, block)| {
//(
//MortonCode::from_raw(indx as u64)
//.as_point::<FieldOf<OctreeOf<Chunk>>>()
//.unwrap(),
//block,
//)
//})
//}

//pub fn build(self) -> Chunk {
//let octree = OctreeBuilder::build_octree(&self.tree.0, 0, 16777216)
//.map_left(|option_block| Octree::new(Point3::new(0, 0, 0), option_block, 8))
//.into_inner();
//Chunk {
//pos: self.pos,
//octree,
//}
//}
//}

impl Builder for Chunk
where
    OctreeOf<Chunk>: Builder,
{
    type Builder = ChunkBuilder;

    fn builder() -> Self::Builder {
        ChunkBuilder(<Chunk as HasOctree>::Octree::builder())
    }
}

pub struct ChunkBuilder(<OctreeOf<Chunk> as Builder>::Builder);
impl ChunkBuilder {
    pub fn build(self, point: Point3<i32>) -> Chunk {
        Chunk {
            pos: point,
            octree: self.0.build(),
        }
    }
}

impl<'a> IntoParallelIterator for &'a mut ChunkBuilder {
    type Iter = <&'a mut <OctreeOf<Chunk> as Builder>::Builder as IntoParallelIterator>::Iter;
    type Item = <Self::Iter as ParallelIterator>::Item;

    fn into_par_iter(self) -> Self::Iter {
        self.0.into_par_iter()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::volume::{Cube, Sphere};

    #[test]
    fn test_plateau_built_correctly() {
        let mut chunk_to_be = Chunk::builder();
        chunk_to_be
            .par_iter_mut()
            .for_each(|(point, block)| *block = if point.y < 128 { Some(1) } else { None });
        let chunk = chunk_to_be.build(Point3::origin());
        Cube::<u16>::new(Point3::new(128, 128, 128), 128)
            .iter()
            .for_each(|point| {
                let pos = Point3::new(point.x as u8, point.y as u8, point.z as u8);
                assert_eq!(
                    chunk.get_block(pos),
                    if pos.y < 128 { Some(1) } else { None },
                    "{:?}",
                    pos
                );
            })
    }

    #[test]
    fn test_sphere_built_correctly() {
        let r_2: u16 = 128 * 128;
        let mut chunk_to_be = Chunk::builder();
        chunk_to_be.par_iter_mut().for_each(|(point, block)| {
            let x = Sphere::difference(point.x as u16, 128);
            let y = Sphere::difference(point.y as u16, 128);
            let z = Sphere::difference(point.z as u16, 128);
            *block = if x * x + y * y + z * z <= r_2 {
                Some(1)
            } else {
                None
            }
        });
        let chunk = chunk_to_be.build(Point3::origin());
        Sphere::<u16>::new(Point3::new(128, 128, 128), 128)
            .iter()
            .for_each(|point| {
                let pos = Point3::new(point.x as u8, point.y as u8, point.z as u8);
                assert_eq!(chunk.get_block(pos), Some(1), "{:?}", pos);
            });
    }
}
