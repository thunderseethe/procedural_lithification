use amethyst::core::nalgebra::Point3;
use array_init;
use num_traits::{FromPrimitive, ToPrimitive};
use std::borrow::Borrow;
use std::iter::Iterator;
use std::sync::Arc;

use crate::chunk::block::Block;
use crate::chunk::Chunk;
use crate::octree::{
    octant::OctantId, octant_dimensions::OctantDimensions, octree_data::OctreeData, Octree,
};

#[derive(ToPrimitive, FromPrimitive, Clone, Copy)]
enum NodeVariant {
    Error = 0,
    Empty = 1,
    Leaf = 2,
    Branch = 3,
}

fn variant_to_bits<V: Borrow<NodeVariant>>(var: V) -> u8 {
    // Variants are encoded as the 2 LSBs of the u8
    var.borrow().to_u8().unwrap()
}
fn bits_to_variant(bits: u8) -> NodeVariant {
    NodeVariant::from_u8(bits).unwrap_or_else(|| {
        panic!(
            "Invalid bit pattern {:?} encountered when converting bits to variants.",
            bits
        );
    })
}

fn variants_to_bytes(vars: Vec<NodeVariant>) -> Vec<u8> {
    vars.chunks(4)
        .map(|variants| {
            variants.into_iter().enumerate().fold(0, |byte, (i, var)| {
                let bits = variant_to_bits(var);
                let mask = bits << (2 * (3 - i));
                byte | mask
            })
        })
        .collect()
}
fn bytes_to_variants<'a>(bytes: Vec<u8>) -> Vec<NodeVariant> {
    let mut accum = Vec::with_capacity(bytes.len() * 4);
    for byte in bytes {
        accum.push(bits_to_variant((byte >> 6) & 0x03));
        accum.push(bits_to_variant((byte >> 4) & 0x03));
        accum.push(bits_to_variant((byte >> 2) & 0x03));
        accum.push(bits_to_variant((byte >> 0) & 0x03));
    }
    accum
}

fn block_to_bytes(block: Block) -> [u8; 4] {
    array_init::array_init(|i| ((block >> (3 - i) * 8) & 0xFF) as u8)
}
fn bytes_to_block(bytes: &[u8]) -> Block {
    bytes
        .into_iter()
        .enumerate()
        .fold(0, |block, (i, b)| block | (*b as u32) << (3 - i) * 8)
}

fn chunk_lists_to_bytes(nodes: Vec<NodeVariant>, blocks: Vec<Block>) -> Vec<u8> {
    let mut bytes = variants_to_bytes(nodes);
    bytes.reserve_exact(blocks.len() * 4);
    for block in blocks {
        bytes.extend_from_slice(&block_to_bytes(block));
    }
    bytes
}

struct VarIter<'a> {
    byte: usize,
    shift: u8,
    data: &'a Vec<u8>,
}
impl<'a> VarIter<'a> {
    fn new(data: &'a Vec<u8>) -> Self {
        Self {
            byte: 0,
            shift: 0,
            data,
        }
    }
    fn remnants(self) -> Vec<u8> {
        if self.shift == 0 {
            self.data[self.byte..self.data.len()].to_vec()
        } else {
            self.data[self.byte + 1..self.data.len()].to_vec()
        }
    }
}

impl<'a> Iterator for VarIter<'a> {
    type Item = NodeVariant;

    fn next(&mut self) -> Option<NodeVariant> {
        if self.byte >= self.data.len() {
            return None;
        }
        if self.shift >= 4 {
            self.shift = 0;
            self.byte += 1;
        }
        let curr_byte = self.data[self.byte];
        let bits = (curr_byte >> (2 * (3 - self.shift))) & 0x03;
        self.shift += 1;
        Some(bits_to_variant(bits))
    }
}

fn bytes_to_chunk_lists(bytes: &Vec<u8>) -> (Vec<NodeVariant>, Vec<Block>) {
    fn count_vars(iter: &mut VarIter, num_to_read: u8) -> Vec<NodeVariant> {
        assert!(num_to_read >= 1, "Should never read zero bytes");
        let mut accum = vec![iter.next().unwrap()];
        let first = &accum[0];
        match first {
            NodeVariant::Branch => accum.append(&mut count_vars(iter, 8)),
            _ => (),
        }
        for _ in 1..num_to_read {
            accum.append(&mut count_vars(iter, 1))
        }
        accum
    }

    let mut iter = VarIter::new(bytes);
    let nodes = count_vars(&mut iter, 1);
    let block_bytes = iter.remnants();
    assert!(
        block_bytes.len() % 4 == 0,
        "The block byte list is corrupted"
    );
    let blocks = block_bytes
        .chunks(4)
        .map(|bits| bytes_to_block(bits))
        .collect();
    (nodes, blocks)
}

pub fn chunk_to_bytes(chunk: &Chunk) -> Vec<u8> {
    fn empty_octree_translate() -> (Vec<NodeVariant>, Vec<Block>) {
        (vec![NodeVariant::Empty], vec![])
    }
    fn leaf_octree_translate(block: &Block) -> (Vec<NodeVariant>, Vec<Block>) {
        (vec![NodeVariant::Leaf], vec![*block])
    }
    fn node_octree_translate(tree: &[Arc<Octree<Block>>; 8]) -> (Vec<NodeVariant>, Vec<Block>) {
        let mut node_accum = vec![NodeVariant::Branch];
        let mut block_accum = Vec::new();

        let lists: Vec<_> = tree
            .iter()
            .map(|e| {
                e.map(
                    empty_octree_translate,
                    leaf_octree_translate,
                    node_octree_translate,
                )
            })
            .collect();
        for (node_list, block_list) in lists {
            node_accum.extend(node_list);
            block_accum.extend(block_list);
        }
        (node_accum, block_accum)
    }

    let (vars, blocks) = chunk.octree.map(
        empty_octree_translate,
        leaf_octree_translate,
        node_octree_translate,
    );
    chunk_lists_to_bytes(vars, blocks)
}
pub fn bytes_to_chunk(bytes: &Vec<u8>, chunk_pos: Point3<i32>) -> Chunk {
    fn construct_tree<N, B>(
        nodes: &mut N,
        blocks: &mut B,
        height: u32,
        bounds: OctantDimensions,
    ) -> Octree<Block>
    where
        N: Iterator<Item = NodeVariant>,
        B: Iterator<Item = Block>,
    {
        let data = match nodes.next().unwrap() {
            NodeVariant::Empty => OctreeData::Empty,
            NodeVariant::Leaf =>
                OctreeData::Leaf(Arc::new(blocks.next().unwrap())),
            NodeVariant::Branch =>OctreeData::Node(
                array_init::from_iter(OctantId::iter().map(|octant|
                    Arc::new(construct_tree(
                        nodes,
                        blocks,
                        height-1,
                        octant.sub_octant_bounds(&bounds)
                    ))
                )).expect("Recursive reconstruction of Octree failed.")
            ),
            NodeVariant::Error =>
                panic!("Attempted to reconstitute an erroneous node value. Something something the bounding is fucked."),
        };
        Octree::with_fields(data, bounds, height)
    }
    let (nodes, blocks) = bytes_to_chunk_lists(bytes);
    let (mut nodes, mut blocks) = (nodes.into_iter(), blocks.into_iter());
    let root_dims = OctantDimensions::new(Point3::new(0, 0, 0), 256);
    let root = construct_tree(&mut nodes, &mut blocks, 8, root_dims);
    Chunk {
        pos: chunk_pos,
        octree: root,
    }
}

mod test {
    use amethyst::core::nalgebra::Point3;

    use super::{bytes_to_chunk, chunk_to_bytes};
    use crate::terrain::{DefaultGenerateBlock, Terrain};
    #[test]
    fn translation_bidirectionality_test() {
        // This test will be considered successful if the chunk stays the same
        // after going through the encoding-decoding process. This shows that
        // transcoding the chunk to a byte stream does not affect its value.
        let center = Point3::origin();
        let test_chunk = Terrain::default().generate_chunk(center);
        let rebuilt_chunk = bytes_to_chunk(&chunk_to_bytes(&test_chunk), center);
        assert_eq!(test_chunk, rebuilt_chunk);
    }

}
