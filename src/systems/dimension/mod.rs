use crate::dimension::morton_code::MortonCode;
use amethyst::{
    core::{
        bundle::{Result, SystemBundle},
        nalgebra::Point3,
        specs::DispatcherBuilder,
    },
    renderer::MeshData,
};

pub mod render_dimension;
use crate::systems::dimension::render_dimension::RenderDimensionSystem;

pub enum DimensionChunkEvent {
    GeneratedChunk(Point3<f32>, MeshData),
}

pub struct ChunkTag(MortonCode);

pub struct DimensionBundle;
impl DimensionBundle {
    pub fn new() -> Self {
        DimensionBundle
    }
}

impl<'a, 'b> SystemBundle<'a, 'b> for DimensionBundle {
    fn build(self, builder: &mut DispatcherBuilder<'a, 'b>) -> Result<()> {
        builder.add(RenderDimensionSystem::new(), "render_dimension", &[]);
        Ok(())
    }
}
