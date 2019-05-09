use amethyst::core::nalgebra::Point3;
use noise::{NoiseFn, Perlin, Seedable};
use rayon::prelude::*;
use std::borrow::Borrow;

use crate::chunk::{
    block::{Block, DIRT_BLOCK},
    chunk_builder::ChunkBuilder,
    Chunk,
};
use crate::octree::Number;

pub type HeightMap = [[u8; 256]; 256];

pub trait GenerateBlockFn {
    fn generate(&self, height_map: &HeightMap, point: &Point3<Number>) -> Option<Block>;
}
impl<F> GenerateBlockFn for F
where
    F: Fn(&HeightMap, &Point3<Number>) -> Option<Block>,
{
    fn generate(&self, height_map: &HeightMap, pos: &Point3<Number>) -> Option<Block> {
        self(height_map, pos)
    }
}
pub struct DefaultGenerateBlock();
impl GenerateBlockFn for DefaultGenerateBlock {
    fn generate(&self, height_map: &HeightMap, p: &Point3<Number>) -> Option<Block> {
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
        parallel_array_init::par_array_init(|x| {
            parallel_array_init::par_array_init(|z| {
                let nx = (chunk_pos.x as f64) + ((x as f64 / 256.0) - 0.5);
                let nz = (chunk_pos.z as f64) + ((z as f64 / 256.0) - 0.5);
                let noise = self.perlin.get([nx, nz])
                    + 0.5 * self.perlin.get([2.0 * nx, 2.0 * nz])
                    + 0.25 * self.perlin.get([4.0 * nx, 4.0 * nz])
                    + 0.13 * self.perlin.get([8.0 * nx, 8.0 * nz])
                    + 0.06 * self.perlin.get([16.0 * nx, 16.0 * nz])
                    + 0.03 * self.perlin.get([32.0 * nx, 32.0 * nz]);
                let noise = noise / (1.0 + 0.5 + 0.25 + 0.13 + 0.06 + 0.03);
                ((noise / 2.0 + 0.5) * 256.0).ceil() as u8
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
        let mut chunk_to_be = ChunkBuilder::new(*chunk_pos);
        chunk_to_be
            .par_iter_mut()
            .for_each(|(pos, block)| *block = self.generate_block.generate(&height_map, &pos));
        chunk_to_be.build()
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_generating_plateau_works_correctly() {
        let terrain = Terrain::default().with_block_generator(
            |_height_map: &HeightMap, p: &Point3<Number>| {
                if p.y < 128 {
                    Some(1)
                } else {
                    None
                }
            },
        );
        let chunk = terrain.generate_chunk(Point3::origin());
        println!("{:?}", chunk);
        for (dim, block) in chunk.iter() {
            assert_eq!(block, &1);
            assert!(dim.bottom_left.y < 128);
        }
    }

    #[test]
    fn test_generating_sphere_works_correctly() {
        let terrain = Terrain::default().with_block_generator(
            |_height_map: &HeightMap, p: &Point3<Number>| {
                let x = p.x as isize - 128;
                let y = p.y as isize - 128;
                let z = p.z as isize - 128;
                if x * x + y * y + z * z <= 64 * 64 {
                    Some(1)
                } else {
                    None
                }
            },
        );
        let chunk = terrain.generate_chunk(Point3::origin());
        for (dim, _) in chunk.iter() {
            let x = dim.bottom_left.x as isize - 128;
            let y = dim.bottom_left.y as isize - 128;
            let z = dim.bottom_left.z as isize - 128;
            assert!(x * x + y * y + z * z <= 64 * 64);
        }
    }
}
