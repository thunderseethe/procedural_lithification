use crate::octree::*;
use amethyst::core::nalgebra::Point3;
use std::{borrow::Borrow, default::Default};

pub type Block = u32;
pub static DIRT_BLOCK: Block = 1;

#[derive(Debug)]
pub struct Chunk {
    octree: Octree<Block>,
    // check that boxes are placed at their top right corner.
}

impl Default for Chunk {
    fn default() -> Self {
        // Default chunk size is 256 x 256 x 256
        Chunk::new(Octree::with_root_default(8))
    }
}

impl Chunk {
    pub fn new(octree: Octree<Block>) -> Self {
        Chunk { octree }
    }

    pub fn place_block<P>(&self, pos: P, block: Block) -> Self
    where
        P: Borrow<Point3<Number>>,
    {
        Chunk::new(self.octree.insert(pos, block))
    }

    pub fn iter<'a>(&'a self) -> ChunkIterator<'a> {
        ChunkIterator {
            iter: self.octree.iter(),
            state: None,
        }
    }
}

pub struct ChunkIterator<'a> {
    iter: OctreeIterator<'a, Block>,
    state: Option<(&'a OctantDimensions, &'a Block, Point3<Number>)>,
}

impl<'a> ChunkIterator<'a> {
    fn increment(&self, dim: &'a OctantDimensions, point: Point3<Number>) -> Point3<Number> {
        let mut result = Point3::new(point.x + 1, point.y, point.z);
        if result.x > dim.x_max() {
            result.x = dim.x_min() + 1;
            result.y += 1;
        }
        if result.y > dim.y_max() {
            result.y = dim.y_min() + 1;
            result.z += 1;
        }
        if result.z > dim.z_max() {
            panic!("Iter should have stopped before leaving dimension bounds");
        }
        return result;
    }
}

impl<'a> Iterator for ChunkIterator<'a> {
    type Item = (Point3<Number>, &'a Block);

    fn next(&mut self) -> Option<Self::Item> {
        self.state
            .and_then(|(dim, block, point)| {
                if point == dim.top_right() {
                    self.state = None;
                    self.next()
                } else {
                    let new_point = self.increment(dim, point);
                    self.state = Some((dim, block, new_point));
                    Some((new_point, block))
                }
            })
            .or_else(|| {
                self.iter.next().map(|(dim, block)| {
                    let point = Point3::new(dim.x_min() + 1, dim.y_min() + 1, dim.z_min() + 1);
                    self.state = Some((dim, block, point));
                    (point, block)
                })
            })
    }
}

#[cfg(test)]
mod test {
    use super::{Chunk, Point3};

    #[test]
    fn test_chunk_iterator() {
        let block = 1234;
        let mut chunk = Chunk::default();
        let chunk = chunk
            .place_block(Point3::new(1, 1, 1), block)
            .place_block(Point3::new(1, 1, 2), block)
            .place_block(Point3::new(1, 2, 1), block)
            .place_block(Point3::new(1, 2, 2), block)
            .place_block(Point3::new(2, 1, 1), block)
            .place_block(Point3::new(2, 1, 2), block)
            .place_block(Point3::new(2, 2, 1), block)
            .place_block(Point3::new(2, 2, 2), block);

        let mut iter = chunk.iter();

        assert_eq!(iter.next(), Some((Point3::new(1, 1, 1), &block)));
        assert_eq!(iter.next(), Some((Point3::new(2, 1, 1), &block)));
        assert_eq!(iter.next(), Some((Point3::new(1, 2, 1), &block)));
        assert_eq!(iter.next(), Some((Point3::new(2, 2, 1), &block)));
        assert_eq!(iter.next(), Some((Point3::new(1, 1, 2), &block)));
        assert_eq!(iter.next(), Some((Point3::new(2, 1, 2), &block)));
        assert_eq!(iter.next(), Some((Point3::new(1, 2, 2), &block)));
        assert_eq!(iter.next(), Some((Point3::new(2, 2, 2), &block)));
        assert_eq!(iter.next(), None);
    }

}
