use crate::chunk::Chunk;
use crate::dimension::ChunkMortonCode;
use crate::octree::{octant::OctantId, ElementOf};
use amethyst::core::nalgebra::Point3;

/// Messages that are sent by client and received by server
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ClientProtocol {
    UpdatePlayerPos(Point3<f32>),
}

// Messages taht are sent by server and received by client
#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum ServerProtocol {
    NewChunk(NewChunk),
}

#[derive(Clone, PartialEq, Serialize, Deserialize)]
pub enum NewChunk {
    /// Represents a chunk that is all empty or all one element, so we only need to send that element and position.
    UniformChunk(ChunkMortonCode, Option<ElementOf<Chunk>>),
}
