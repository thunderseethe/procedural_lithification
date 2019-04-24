use std::sync::Arc;
use std::iter::Iterator;
use amethyst::core::nalgebra::Point3;

use crate::chunk::block::Block;
use crate::chunk::Chunk;
use crate::octree::{
    Octree,
    octree_data::OctreeData,
    octant::Octant,
    octant_dimensions::OctantDimensions,
};

enum NodeVariant {
    Empty,
    Leaf,
    Branch,
    Error,
}

fn variant_to_bits(var: NodeVariant) -> u8 {
    // Variants are encoded as the 2 LSBs of the u8
    match var {
        NodeVariant::Empty => 0x00,
        NodeVariant::Leaf => 0x01,
        NodeVariant::Branch => 0x02,
        NodeVariant::Error => 0x03,
    }
}
fn bits_to_variant(bits: u8) -> NodeVariant {
    match bits {
        0x00 =>NodeVariant::Empty,
        0x01 =>NodeVariant::Leaf,
        0x02 =>NodeVariant::Branch,
        0x03 =>NodeVariant::Error,
        _ => panic!("Invalid bit pattern {:?} encountered when converting bits to variants.", bits),
    }
}

fn variants_to_bytes(vars: Vec<NodeVariant>) -> Vec<u8> {
    fn accum_var(accum: &mut u8, var: NodeVariant, pos: u8) {
        let bits = variant_to_bits(var);
        let mask = bits << (2*(3 - pos));
        *accum |= mask;
    }
    // Should only need this one allocation.
    let mut accum_vec = Vec::with_capacity((vars.len()/4 + 1));
    // Accumulates 4 variants into one byte
    let mut accum_byte = 0x00;
    // Tracks the number of accumulations into the accumulator byte. Reset at 4.
    let mut shifts = 0;
    for var in vars {
        accum_var(&mut accum_byte, var, shifts);
        shifts += 1;
        if shifts >= 4 {
            accum_vec.push(accum_byte);
            accum_byte = 0;
            shifts = 0;
        }
    }
    // Pad out the final byte with invalid values
    while shifts < 4 {
        accum_var(&mut accum_byte, NodeVariant::Error, shifts);
        shifts += 1;
    }
    accum_vec.push(accum_byte);
    accum_vec
}
fn bytes_to_variants<'a>(bytes: Vec<u8>) -> Vec<NodeVariant> {
    let mut accum = Vec::new();
    for byte in bytes {
        accum.push(bits_to_variant((byte >> 6) & 0x03));
        accum.push(bits_to_variant((byte >> 4) & 0x03));
        accum.push(bits_to_variant((byte >> 2) & 0x03));
        accum.push(bits_to_variant((byte >> 0) & 0x03));
    }
    accum
}

fn block_to_bytes(block: Block) -> [u8; 4] {
    let v1 = ((block >> 24) & 0xFF) as u8;
    let v2 = ((block >> 16) & 0xFF) as u8;
    let v3 = ((block >> 08) & 0xFF) as u8;
    let v4 = ((block >> 00) & 0xFF) as u8;
    [v1, v2, v3, v4]
}
fn bytes_to_block(bytes: [u8; 4]) -> Block {
    let mut block = 0;
    block |= (bytes[0] as u32) << 24;
    block |= (bytes[1] as u32) << 16;
    block |= (bytes[2] as u32) << 08;
    block |= (bytes[3] as u32) << 00;
    block
}

fn chunk_lists_to_bytes(nodes: Vec<NodeVariant>, blocks:Vec<Block>) -> Vec<u8>{
    let mut bytes = variants_to_bytes(nodes);
    bytes.reserve_exact(blocks.len()*4);
    for block in blocks {
        bytes.extend_from_slice(&block_to_bytes(block));
    }
    bytes
}
fn bytes_to_chunk_lists(bytes: Vec<u8>) -> (Vec<NodeVariant>, Vec<Block>) {
    struct VarIter {
        byte: usize,
        shift: u8,
        data: Vec<u8>,
    }
    impl VarIter {
        fn new(data: Vec<u8>) -> Self {Self {byte: 0, shift: 0, data}}
        fn next(&mut self) -> NodeVariant {
            if self.shift >= 4 {
                self.shift = 0;
                self.byte += 1;
            }
            let curr_byte = self.data[self.byte];
            let bits = (curr_byte >> (2 * (3 - self.shift))) & 0x03;
            self.shift += 1;
            bits_to_variant(bits)
        }
        fn remnants(self) -> Vec<u8> {
            if self.shift == 0 {
                self.data[self.byte..self.data.len()].to_vec()
            } else {
                self.data[self.byte+1..self.data.len()].to_vec()
            }
        }
    }

    fn count_vars(iter: &mut VarIter, num_to_read: u8) -> Vec<NodeVariant> {
        assert!(num_to_read >= 1, "Should never read zero bytes");
        let mut accum = vec!(iter.next());
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
    assert!(block_bytes.len() % 4 == 0, "The block byte list is corrupted");
    let mut blocks = Vec::with_capacity(block_bytes.len()/4);
    let mut block_num = 0;
    while block_num * 4 < block_bytes.len(){
        let b1 = block_bytes[block_num * 4 + 0] as u32;
        let b2 = block_bytes[block_num * 4 + 1] as u32;
        let b3 = block_bytes[block_num * 4 + 2] as u32;
        let b4 = block_bytes[block_num * 4 + 3] as u32;
        let mut block: u32 = 0;
        block |= b1 << 24;
        block |= b2 << 16;
        block |= b3 << 08;
        block |= b4 << 00;
        blocks.push(block);
    }
    (nodes, blocks)
}

fn chunk_to_bytes(chunk: Chunk) -> Vec<u8> {
    fn empty_octree_translate() -> (Vec<NodeVariant>, Vec<Block>) {
        (vec!(NodeVariant::Empty), vec!())
    }
    fn leaf_octree_translate(block: &Block) -> (Vec<NodeVariant>, Vec<Block>) {
        (vec!(NodeVariant::Leaf), vec!(*block))
    }
    fn node_octree_translate(tree: &[Arc<Octree<Block>>; 8])
            -> (Vec<NodeVariant>, Vec<Block>) {
        let mut node_accum = vec!(NodeVariant::Branch);
        let mut block_accum = Vec::new();

        let lists: Vec<_> = tree.iter().map(|e| e.map(
            empty_octree_translate,
            leaf_octree_translate,
            node_octree_translate
        )).collect();
        for (mut node_list, mut block_list) in lists {
            node_accum.append(&mut node_list);
            block_accum.append(&mut block_list);
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
fn bytes_to_chunk(bytes: Vec<u8>, chunk_pos: Point3<i32>) -> Chunk {
    fn construct_tree<N, B>(nodes: &mut N, blocks: &mut B, height: u32, bounds: OctantDimensions) 
        -> Octree<Block>
        where N: Iterator<Item=NodeVariant>,
              B: Iterator<Item=Block> {
        let data = match nodes.next().unwrap() {
            NodeVariant::Empty => OctreeData::Empty,
            NodeVariant::Leaf =>
                OctreeData::Leaf(Arc::new(blocks.next().unwrap())),
            NodeVariant::Branch =>OctreeData::Node([
                Arc::new(construct_tree(nodes, blocks, height-1, Octant::LowLowLow.sub_octant_bounds(&bounds))),
                Arc::new(construct_tree(nodes, blocks, height-1, Octant::LowLowHigh.sub_octant_bounds(&bounds))),
                Arc::new(construct_tree(nodes, blocks, height-1, Octant::LowHighLow.sub_octant_bounds(&bounds))),
                Arc::new(construct_tree(nodes, blocks, height-1, Octant::LowHighHigh.sub_octant_bounds(&bounds))),
                Arc::new(construct_tree(nodes, blocks, height-1, Octant::HighLowLow.sub_octant_bounds(&bounds))),
                Arc::new(construct_tree(nodes, blocks, height-1, Octant::HighLowHigh.sub_octant_bounds(&bounds))),
                Arc::new(construct_tree(nodes, blocks, height-1, Octant::HighHighLow.sub_octant_bounds(&bounds))),
                Arc::new(construct_tree(nodes, blocks, height-1, Octant::HighHighHigh.sub_octant_bounds(&bounds))),
            ]),
            NodeVariant::Error => panic!("Attempted to reconstitute an erroneous node value. Something something the bounding is fucked."),
        };
        Octree::with_fields(data, bounds, height)
    }
    let (nodes, blocks) = bytes_to_chunk_lists(bytes);
    let (mut nodes, mut blocks) = (nodes.into_iter(), blocks.into_iter());
    let root_dims = OctantDimensions::new(Point3::new(0, 0, 0), 256);
    let root = construct_tree(&mut nodes, &mut blocks, 8, root_dims);
    Chunk {pos: chunk_pos, octree: root}
}