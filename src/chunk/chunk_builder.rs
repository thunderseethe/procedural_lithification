use super::HasOctree;
use crate::chunk::{Chunk, OctreeOf};
use crate::field::*;
use crate::octree::builder::*;
use amethyst::core::nalgebra::Point3;
use rayon::iter::*;

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
    pub fn build(self, point: Point3<FieldOf<Chunk>>) -> Chunk {
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
    use crate::octree::descriptors::Diameter;
    use crate::volume::{Cube, Sphere};

    #[test]
    fn test_plateau_built_correctly() {
        let mut chunk_to_be = Chunk::builder();
        let half_chunk = (Chunk::DIAMETER / 2) as u16;
        chunk_to_be.par_iter_mut().for_each(|(point, block)| {
            *block = if point.y < half_chunk as u8 {
                Some(1)
            } else {
                None
            }
        });
        let chunk = chunk_to_be.build(Point3::origin());
        Cube::<u16>::new(Point3::new(half_chunk, half_chunk, half_chunk), half_chunk)
            .iter()
            .for_each(|point| {
                let pos = Point3::new(point.x as u8, point.y as u8, point.z as u8);
                assert_eq!(
                    chunk.get_block(pos),
                    if point.y < half_chunk { Some(1) } else { None },
                    "{:?}",
                    pos
                );
            })
    }

    #[test]
    fn test_sphere_built_correctly() {
        let half_chunk = (Chunk::DIAMETER / 2) as u16;
        let r_2: u16 = half_chunk * half_chunk;
        let mut chunk_to_be = Chunk::builder();
        chunk_to_be.par_iter_mut().for_each(|(point, block)| {
            let x = Sphere::difference(point.x as u16, half_chunk);
            let y = Sphere::difference(point.y as u16, half_chunk);
            let z = Sphere::difference(point.z as u16, half_chunk);
            *block = if x * x + y * y + z * z <= r_2 {
                Some(1)
            } else {
                None
            }
        });
        let chunk = chunk_to_be.build(Point3::origin());
        Sphere::<u16>::new(Point3::new(half_chunk, half_chunk, half_chunk), half_chunk)
            .iter()
            .for_each(|point| {
                let pos = Point3::new(point.x as u8, point.y as u8, point.z as u8);
                assert_eq!(chunk.get_block(pos), Some(1), "{:?}", pos);
            });
    }
}
