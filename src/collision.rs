use crate::chunk::Chunk;
use amethyst::core::nalgebra as na;
use amethyst::core::nalgebra::Point3;
use amethyst::ecs::{Component, VecStorage};
use ncollide3d::events::ProximityEvents;
use ncollide3d::math::{Isometry, Vector};
use ncollide3d::query::Ray;
use ncollide3d::shape::{Cuboid, ShapeHandle};
use ncollide3d::world::{
    CollisionGroups, CollisionGroupsPairFilter, CollisionObjectHandle, CollisionWorld,
    GeometricQueryType,
};
use std::borrow::Borrow;
use std::collections::HashMap;
use std::result::Result;

const TERRAIN_GROUP: usize = 1;
const ENTITY_GROUP: usize = 2;
const PLAYER_GROUP: usize = 3;

pub struct RayComponent<N: alga::general::RealField>(Ray<N>);
impl<N: alga::general::RealField> RayComponent<N> {
    pub fn new(ray: Ray<N>) -> Self {
        RayComponent(ray)
    }
}
impl<N: alga::general::RealField> Component for RayComponent<N> {
    type Storage = VecStorage<Self>;
}

#[derive(Debug)]
enum CollisionDetectionError {
    ChunkAlreadyPresent,
}

// better name
struct CollisionDetection {
    world: CollisionWorld<f32, ShapeHandle<f32>>,
    player_handle: CollisionObjectHandle,
    terrain_handles: HashMap<Point3<i32>, Vec<CollisionObjectHandle>>,
}

impl CollisionDetection {
    pub fn new(player_pos: Point3<f32>) -> Self {
        let mut world = CollisionWorld::new(0.2);
        let isometry = Isometry::translation(player_pos.x, player_pos.y, player_pos.z);
        let shape = ShapeHandle::new(Cuboid::new(Vector::new(0.5, 1.0, 0.5)));
        let player_handle = world
            .add(
                isometry,
                shape.clone(),
                CollisionGroups::new()
                    .with_membership(&[PLAYER_GROUP])
                    .with_blacklist(&[PLAYER_GROUP]),
                GeometricQueryType::Proximity(0.1),
                shape,
            )
            .handle();
        CollisionDetection {
            world: world,
            player_handle,
            terrain_handles: HashMap::new(),
        }
    }

    pub fn set_player_pos<P>(&mut self, pos: P)
    where
        P: Borrow<Point3<f32>>,
    {
        let p = pos.borrow();
        self.world
            .set_position(self.player_handle, Isometry::translation(p.x, p.y, p.z));
    }

    pub fn add_chunk(&mut self, chunk: &Chunk) -> Result<(), CollisionDetectionError> {
        if self.terrain_handles.contains_key(&chunk.pos) {
            return Err(CollisionDetectionError::ChunkAlreadyPresent);
        }
        let root = chunk.pos * 256;
        let terrain_handles = chunk
            .iter()
            .map(|octant| {
                let rel_pos: Point3<i32> = na::convert(*octant.bottom_left_front);
                let pos: Point3<f32> = na::convert(root + rel_pos.coords);
                let radius = (octant.diameter / 2) as f32;
                let isometry =
                    Isometry::translation(pos.x + radius, pos.y + radius, pos.z + radius);
                let shape = ShapeHandle::new(Cuboid::new(Vector::new(radius, radius, radius)));
                self.world
                    .add(
                        isometry,
                        shape.clone(),
                        CollisionGroups::new()
                            .with_membership(&[TERRAIN_GROUP])
                            .with_blacklist(&[TERRAIN_GROUP]),
                        GeometricQueryType::Proximity(0.2),
                        shape,
                    )
                    .handle()
            })
            .collect();
        self.terrain_handles.insert(chunk.pos, terrain_handles);
        Ok(())
    }

    pub fn remove_chunk(&mut self, chunk: &Chunk) {
        use std::collections::hash_map::Entry::*;
        if let Occupied(entry) = self.terrain_handles.entry(chunk.pos) {
            let (_, handles) = entry.remove_entry();
            self.world.remove(&handles);
        }
    }

    pub fn update(&mut self) {
        self.world.update()
    }

    pub fn proximity_events(&mut self) -> &ProximityEvents {
        self.world.proximity_events()
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::octree::{Number, Octree};
    use crate::terrain::{HeightMap, Terrain};
    use amethyst::core::nalgebra::Point3;
    use ncollide3d::math::{Point, Vector};

    #[test]
    fn test_proximity_event_created_for_player_near_chunk() {
        let mut world = CollisionDetection::new(Point3::origin());
        let chunk = Terrain::default()
            .with_block_generator(
                |_height_map: &HeightMap, p: &Point3<Number>| {
                    if p.y < 128 {
                        Some(1)
                    } else {
                        None
                    }
                },
            )
            .generate_chunk(Point3::origin());
        world
            .add_chunk(&chunk)
            .expect("Empty world contained a chunk");
        world.update();
        let player_ray = Ray::new(Point::new(64., 64., -2.), Vector::new(0., 0., 1.));
        let groups = CollisionGroups::new()
            .with_membership(&[PLAYER_GROUP])
            .with_blacklist(&[PLAYER_GROUP]);

        let intersections = world.world.interferences_with_ray(&player_ray, &groups);
        for (_, intersection) in intersections {
            println!("intersection: {:?}", intersection);
            println!(
                "point of contact: {}",
                player_ray.origin + player_ray.dir * intersection.toi
            );
        }
    }
}
