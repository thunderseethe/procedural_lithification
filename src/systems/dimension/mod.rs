use crate::dimension::morton_code::MortonCode;
use amethyst::{
    core::{
        bundle::{Result, SystemBundle},
        specs::DispatcherBuilder,
    },
    ecs::{Component, VecStorage},
};

pub mod render_dimension;
use crate::systems::dimension::render_dimension::RenderDimensionSystem;

pub enum DimensionChunkEvent {
    NewChunkAt(MortonCode),
}

pub struct ChunkTag(MortonCode);
impl Component for ChunkTag {
    type Storage = VecStorage<Self>;
}

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
