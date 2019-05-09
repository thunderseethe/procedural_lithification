use amethyst::core::nalgebra::Point3;
use array_init;
use num_traits::{FromPrimitive, ToPrimitive};
use std::borrow::Borrow;
use std::iter::Iterator;

use crate::chunk::block::Block;
use crate::chunk::{Chunk, OctreeOf, HasOctree};
use crate::octree::new_octree::*;
use crate::octree::{
    octant::OctantId
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

trait TranslateOctree: ElementType {
    fn translate(&self, nodes: &mut Vec<NodeVariant>, elements: &mut Vec<Self::Element>);
}
impl<O> TranslateOctree for OctreeLevel<O>
where
    O: OctreeTypes + TranslateOctree,
    ElementOf<Self>: Clone,
{
    fn translate(&self, nodes: &mut Vec<NodeVariant>, elements: &mut Vec<ElementOf<Self>>) {
        use LevelData::*;
        match self.data() {
            Empty => {
                nodes.push(NodeVariant::Empty);
                // No element to append
            }
            Leaf(ref elem) => {
                nodes.push(NodeVariant::Leaf);
                elements.push(elem.clone());
            }
            Node(ref children) => {
                nodes.push(NodeVariant::Branch);
                for child in children {
                    child.translate(nodes, elements);
                }
            }
        }
    }
}
impl<E: Clone, N: Number> TranslateOctree for OctreeBase<E, N> {
    fn translate(&self, nodes: &mut Vec<NodeVariant>, elements: &mut Vec<E>) {
        match self.data() {
            None => nodes.push(NodeVariant::Empty),
            Some(elem) => {
                nodes.push(NodeVariant::Leaf);
                elements.push(elem.clone());
            }
        }
    }
}

pub fn chunk_to_bytes(chunk: &Chunk) -> Vec<u8> {
    let (mut vars, mut blocks) = (Vec::new(), Vec::new());
    chunk.octree.translate(&mut vars, &mut blocks);
    chunk_lists_to_bytes(vars, blocks)
}

trait ConstructTree: OctreeTypes {
    fn construct_tree<N, E>(nodes: &mut N, elements: &mut E, pos: Point3<Self::Field>) -> Self
    where
        N: Iterator<Item = NodeVariant>,
        E: Iterator<Item = Self::Element>;
}
impl<O> ConstructTree for OctreeLevel<O> 
where
    O: OctreeTypes + ConstructTree + Diameter,
{
    fn construct_tree<N, E>(nodes: &mut N, elements: &mut E, pos: Point3<FieldOf<Self>>) -> Self
    where
        N: Iterator<Item = NodeVariant>,
        E: Iterator<Item = ElementOf<Self>>
    {
        let node = nodes.next().unwrap();
        let data = match node {
            NodeVariant::Empty => LevelData::Empty,
            NodeVariant::Leaf => LevelData::Leaf(elements.next().unwrap()),
            NodeVariant::Branch => {
                LevelData::Node(array_init::from_iter(OctantId::iter().map(|octant| 
                    Ref::new(O::construct_tree(
                        nodes, 
                        elements, 
                        octant.sub_octant_bottom_left(pos, O::diameter()))) 
                )).unwrap())
            },
            NodeVariant::Error => 
                panic!("Attempted to reconstitute an erroneous node value. Something something the bounding is fucked."),
        };
        Self::new(data, pos)
    }
}
impl<E, N: Number> ConstructTree for OctreeBase<E, N> {
    fn construct_tree<NIter, EIter>(nodes: &mut NIter, elements: &mut EIter, pos: Point3<FieldOf<Self>>) -> Self
    where
        NIter: Iterator<Item = NodeVariant>,
        EIter: Iterator<Item = ElementOf<Self>>
    {
        let node = nodes.next().unwrap();
        let data = match node {
            NodeVariant::Empty => None,
            NodeVariant::Leaf => elements.next(), // micro optimization ^_^
            NodeVariant::Branch =>
                panic!("Encountered Branch node variant while constructing OctreeBase."),
            NodeVariant::Error =>
                panic!("Attempted to reconstitute an erroneous node value. Something something the bounding is fucked."),
        };
        Self::new(data, pos)
    }
}
pub fn bytes_to_chunk(bytes: &Vec<u8>, chunk_pos: Point3<i32>) -> Chunk {
    let (nodes, blocks) = bytes_to_chunk_lists(bytes);
    let (mut nodes, mut blocks) = (nodes.into_iter(), blocks.into_iter());
    let root: OctreeOf<Chunk> = <Chunk as HasOctree>::Octree::construct_tree(&mut nodes, &mut blocks, Point3::origin());
    Chunk {
        pos: chunk_pos,
        octree: root,
    }
}

pub struct ChunkDeserialize;
impl ChunkDeserialize {
    pub fn from<R>(mut reader: R, pos: Point3<i32>) -> std::io::Result<Chunk>
    where
        R: std::io::Read,
    {
        let mut bytes: Vec<u8> = Vec::new();
        reader
            .read_to_end(&mut bytes)
            .map(|_| bytes_to_chunk(&bytes, pos))
    }
}

pub struct ChunkSerialize;
impl ChunkSerialize {
    pub fn into<W>(mut writer: W, chunk: &Chunk) -> std::io::Result<()>
    where
        W: std::io::Write,
    {
        let bytes = chunk_to_bytes(chunk);
        writer.write_all(&bytes).and_then(|_| writer.flush())
    }
}

#[cfg(test)]
mod test {
    use amethyst::core::nalgebra::Point3;

    use super::{bytes_to_chunk, chunk_to_bytes};
    use crate::terrain::Terrain;
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
