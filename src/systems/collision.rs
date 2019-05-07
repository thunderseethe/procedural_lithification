use crate::chunk::Chunk;
use crate::collision::CollisionDetection;
use crate::dimension::Dimension;
use crate::systems::dimension::DimensionChunkEvent;
use crate::systems::player::PlayerControlTag;
use amethyst::{
    core::{EventReader, Transform},
    ecs::{
        Read, ReadExpect, ReadStorage, Resources, System, SystemData, WriteExpect, WriteStorage,
    },
    shrev::{EventChannel, ReaderId},
};
use parking_lot::Mutex;
use std::sync::Arc;

pub struct CheckPlayerCollisionSystem;
impl<'a> System<'a> for CheckPlayerCollisionSystem {
    type SystemData = (
        WriteExpect<'a, CollisionDetection>,
        WriteStorage<'a, Transform>,
        ReadStorage<'a, PlayerControlTag>,
    );

    fn run(&mut self, (mut collision, mut transform, tag): Self::SystemData) {
        collision.update();
        for event in collision.proximity_events() {
            println!("{:?}", event);
        }
    }
}

pub struct ChunkCollisionMangementSystem {
    reader: Option<ReaderId<DimensionChunkEvent>>,
}
impl<'a> Default for ChunkCollisionMangementSystem {
    fn default() -> Self {
        ChunkCollisionMangementSystem { reader: None }
    }
}
impl<'a> System<'a> for ChunkCollisionMangementSystem {
    type SystemData = (
        Read<'a, EventChannel<DimensionChunkEvent>>,
        ReadExpect<'a, Arc<Mutex<Dimension>>>,
        WriteExpect<'a, CollisionDetection>,
    );

    fn setup(&mut self, res: &mut Resources) {
        Self::SystemData::setup(res);
        self.reader = Some(
            res.fetch_mut::<EventChannel<DimensionChunkEvent>>()
                .register_reader(),
        );
    }

    fn run(&mut self, (channel_reader, dimension, mut collision): Self::SystemData) {
        for event in channel_reader.read(self.reader.as_mut().unwrap()) {
            match event {
                DimensionChunkEvent::NewChunkAt(morton) => {
                    if let Some(chunk_mutex) = dimension.lock().get_chunk(*morton) {
                        collision
                            .add_chunk(&chunk_mutex.lock())
                            .unwrap_or_else(|err| {
                                println!(
                                    "Encountered error adding chunk to collision detection: {:?}",
                                    err
                                );
                            });
                    }
                }
            }
        }
    }
}
