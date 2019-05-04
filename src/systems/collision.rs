use crate::collision::CollisionDetection;
use crate::systems::player::PlayerControlTag;
use amethyst::{
    core::Transform,
    ecs::{ReadStorage, System, WriteExpect, WriteStorage},
};
use parking_lot::Mutex;
use std::sync::Arc;

pub struct CheckPlayerCollisionSystem;
impl<'a> System<'a> for CheckPlayerCollisionSystem {
    type SystemData = (
        WriteExpect<'a, Arc<Mutex<CollisionDetection>>>,
        WriteStorage<'a, Transform>,
        ReadStorage<'a, PlayerControlTag>,
    );

    fn run(&mut self, (collision_mutex, mut transform, tag): Self::SystemData) {
        let mut collision = collision_mutex.lock();
        collision.update();
        for event in collision.proximity_events() {
            println!("{:?}", event);
        }
    }
}
