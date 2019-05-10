use amethyst::core::nalgebra::Point3;
use noise::{NoiseFn, Perlin, Seedable};
use rayon::prelude::*;
use std::borrow::Borrow;

use crate::chunk::{
    block::{Block, DIRT_BLOCK},
    Chunk,
};
use crate::octree::builder::Builder;
use crate::octree::{Diameter, FieldOf};

pub type HeightMap = [[u8; 256]; 256];

pub trait GenerateBlockFn {
    fn generate(&self, height_map: &HeightMap, point: &Point3<FieldOf<Chunk>>) -> Option<Block>;
}
impl<F> GenerateBlockFn for F
where
    F: Fn(&HeightMap, &Point3<FieldOf<Chunk>>) -> Option<Block>,
{
    fn generate(&self, height_map: &HeightMap, pos: &Point3<FieldOf<Chunk>>) -> Option<Block> {
        self(height_map, pos)
    }
}
pub struct DefaultGenerateBlock();
impl GenerateBlockFn for DefaultGenerateBlock {
    fn generate(&self, height_map: &HeightMap, p: &Point3<FieldOf<Chunk>>) -> Option<Block> {
        let subarray: [u8; 256] = height_map[p.x as usize];
        let height: u8 = subarray[p.z as usize];
        if p.y <= height {
            Some(DIRT_BLOCK)
        } else {
            None
        }
    }
}

pub struct Terrain<F> {
    perlin: Perlin,
    generate_block: F,
}

impl Default for Terrain<DefaultGenerateBlock> {
    fn default() -> Self {
        Terrain {
            perlin: Perlin::new(),
            generate_block: DefaultGenerateBlock(),
        }
    }
}

impl<F> Terrain<F>
where
    F: GenerateBlockFn + Sync,
{
    pub fn new(seed: u32, generate_block: F) -> Self {
        Terrain {
            perlin: Perlin::new().set_seed(seed),
            generate_block,
        }
    }

    pub fn with_seed(self, seed: u32) -> Self {
        Terrain {
            perlin: Perlin::new().set_seed(seed),
            ..self
        }
    }

    pub fn with_block_generator<NewF>(self, generate_block: NewF) -> Terrain<NewF>
    where
        NewF: GenerateBlockFn + Sync,
    {
        Terrain {
            generate_block,
            perlin: self.perlin,
        }
    }

    pub fn generate_chunk<P>(&self, chunk_pos_ref: P) -> Chunk
    where
        P: Borrow<Point3<i32>>,
    {
        let chunk_pos = chunk_pos_ref.borrow();
        if chunk_pos.y > 0 {
            Chunk::with_empty(*chunk_pos)
        } else if chunk_pos.y < 0 {
            Chunk::with_block(*chunk_pos, DIRT_BLOCK)
        } else {
            self.y_zero_chunk_generator(chunk_pos)
        }
    }

    #[inline]
    fn create_height_map(&self, chunk_pos: &Point3<i32>) -> HeightMap {
        // TODO: generalize this over Octree::Diameter once new_octree lands
        let chunk_size = Chunk::diameter() as f64;
        parallel_array_init::par_array_init(|x| {
            parallel_array_init::par_array_init(|z| {
                let nx = (chunk_pos.x as f64) + ((x as f64 / chunk_size) - 0.5);
                let nz = (chunk_pos.z as f64) + ((z as f64 / chunk_size) - 0.5);
                let noise = self.perlin.get([nx, nz])
                    + 0.5 * self.perlin.get([2.0 * nx, 2.0 * nz])
                    + 0.25 * self.perlin.get([4.0 * nx, 4.0 * nz])
                    + 0.13 * self.perlin.get([8.0 * nx, 8.0 * nz])
                    + 0.06 * self.perlin.get([16.0 * nx, 16.0 * nz])
                    + 0.03 * self.perlin.get([32.0 * nx, 32.0 * nz]);
                let noise = noise / (1.0 + 0.5 + 0.25 + 0.13 + 0.06 + 0.03);
                ((noise / 2.0 + 0.5) * chunk_size).ceil() as u8
            })
        })
    }

    #[inline]
    pub fn y_zero_chunk_generator<P>(&self, chunk_pos_ref: P) -> Chunk
    where
        P: Borrow<Point3<i32>>,
    {
        let chunk_pos = chunk_pos_ref.borrow();
        let height_map = self.create_height_map(chunk_pos);
        let mut chunk_to_be = Chunk::builder();
        chunk_to_be
            .par_iter_mut()
            .for_each(|(pos, block)| *block = self.generate_block.generate(&height_map, &pos));
        chunk_to_be.build(*chunk_pos)
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use num_traits::AsPrimitive;

    #[test]
    fn test_generating_plateau_works_correctly() {
        let threshold = (Chunk::diameter() / 2).as_();
        let terrain = Terrain::default().with_block_generator(
            |_height_map: &HeightMap, p: &Point3<FieldOf<Chunk>>| {
                if p.y < threshold {
                    Some(1)
                } else {
                    None
                }
            },
        );
        let chunk = terrain.generate_chunk(Point3::origin());
        println!("{:?}", chunk);
        for octant in chunk.iter() {
            assert_eq!(octant.data, &1);
            assert!(octant.bottom_left_front.y < threshold);
        }
    }

    #[test]
    fn test_generating_sphere_works_correctly() {
        let chunk_size = Chunk::diameter() as isize;
        let chunk_half = chunk_size / 2;
        let chunk_quarter = chunk_half / 2;
        let terrain = Terrain::default().with_block_generator(
            |_height_map: &HeightMap, p: &Point3<FieldOf<Chunk>>| {
                let x = p.x as isize - chunk_half;
                let y = p.y as isize - chunk_half;
                let z = p.z as isize - chunk_half;
                if x * x + y * y + z * z <= chunk_quarter * chunk_quarter {
                    Some(1)
                } else {
                    None
                }
            },
        );
        let chunk = terrain.generate_chunk(Point3::origin());
        for octant in chunk.iter() {
            let x = octant.bottom_left_front.x as isize - chunk_half;
            let y = octant.bottom_left_front.y as isize - chunk_half;
            let z = octant.bottom_left_front.z as isize - chunk_half;
            assert!(x * x + y * y + z * z <= chunk_quarter * chunk_quarter);
        }
    }
}
