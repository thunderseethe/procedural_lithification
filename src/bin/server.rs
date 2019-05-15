extern crate amethyst;
extern crate cubes_lib;
extern crate morton_code;
extern crate parking_lot;
extern crate tokio;

use amethyst::{
    core::{nalgebra::Point3, ArcThreadPool, Transform, TransformBundle},
    ecs::Join,
    network::{NetConnection, NetworkBundle},
    prelude::*,
    shrev::EventChannel,
    utils::application_root_dir,
};
use cubes_lib::{
    chunk::Chunk,
    dimension::{ChunkMortonCode, Dimension, DimensionConfig},
    field::FieldOf,
    octree::Diameter,
    protocol::ServerProtocol,
    systems::dimension::DimensionChunkEvent,
    systems::player::PlayerEntityTag,
    volume::Sphere,
};
use morton_code::MortonCode;
use parking_lot::{Mutex, RwLock};
use std::{collections::HashSet, path::PathBuf, sync::Arc};
use tokio::runtime::Runtime;

const CLIENT: &str = "127.0.0.1:3456";

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let app_root = PathBuf::from(application_root_dir());
    let resources = app_root.join("resources");
    let dimension_dir = resources.join("dimension");
    let game_data = GameDataBuilder::default()
        .with_bundle(NetworkBundle::<ServerProtocol>::new(
            CLIENT.parse().unwrap(),
            vec![],
        ))?
        .with_bundle(TransformBundle::new())?;
    let mut game = Application::build(
        &resources,
        ServerDimensionState::new(DimensionConfig::new(dimension_dir, 2)),
    )?
    .with_resource(Runtime::new().unwrap())
    .build(game_data)?;
    game.run();
    Ok(())
}

struct ServerDimensionState {
    dimension_config: DimensionConfig,
    /// Holds points that have been queued for generation so we don't repeat work.
    generate_queue_set: Arc<Mutex<HashSet<ChunkMortonCode>>>,
}
impl ServerDimensionState {
    pub fn new(dimension_config: DimensionConfig) -> Self {
        ServerDimensionState {
            dimension_config,
            generate_queue_set: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    fn init_dimension(
        &self,
        runtime: &mut Runtime,
        channel: &mut EventChannel<DimensionChunkEvent>,
    ) -> Dimension {
        std::fs::create_dir_all(&self.dimension_config.directory)
            .expect("Unable to create dimension directory");
        let mut dimension = Dimension::default();
        {
            let morton = MortonCode::from_raw(0);
            dimension
                ._create_or_load_chunk(
                    self.dimension_config.directory.as_path(),
                    morton,
                    Point3::origin(),
                )
                .expect("Failed to generate chunk at origin");
            channel.single_write(DimensionChunkEvent::NewChunkAt(morton));
        }
        dimension.store(self.dimension_config.directory.as_path(), runtime);
        dimension
    }

    fn register_components(&mut self, world: &mut World) {
        world.register::<PlayerEntityTag>();
        world.register::<NetConnection<ServerProtocol>>();
    }
}
impl<'a, 'b> State<GameData<'a, 'b>, StateEvent> for ServerDimensionState {
    fn on_start(&mut self, data: StateData<GameData>) {
        let StateData { mut world, .. } = data;
        let mut channel = EventChannel::new();
        let dimension = {
            let mut runtime = world.write_resource::<Runtime>();
            self.init_dimension(&mut runtime, &mut channel)
        };
        world.add_resource(dimension);
        world.add_resource(channel);

        self.register_components(&mut world);

        let player_pos = Point3::new(
            Chunk::DIAMETER as f32,
            Chunk::DIAMETER as f32,
            Chunk::DIAMETER as f32,
        );
        let mut transform = Transform::default();
        transform.set_position(player_pos.coords);
        world
            .create_entity()
            .with(transform)
            .with(PlayerEntityTag::default())
            .build();


        // NetConnection to talk to Client
        world
            .create_entity()
            .with(NetConnection::<ServerProtocol>::new(
                CLIENT.parse().unwrap(),
            ))
            .build();
    }

    fn on_stop(&mut self, data: StateData<GameData>) {
        let StateData { world, .. } = data;
        let dimension = world.read_resource::<Dimension>();
        let mut runtime = world.write_resource::<Runtime>();
        dimension.store(self.dimension_config.directory.as_path(), &mut runtime);
    }

    fn fixed_update(&mut self, data: StateData<GameData>) -> Trans<GameData<'a, 'b>, StateEvent> {
        let StateData { world, .. } = data;
        let thread_pool = world.read_resource::<ArcThreadPool>().clone();
        let dimension = Arc::new(RwLock::new(world.write_resource::<Dimension>()));
        let channel = Arc::new(Mutex::new(
            world.write_resource::<EventChannel<DimensionChunkEvent>>(),
        ));
        for (_, transform) in (
            &world.read_storage::<PlayerEntityTag>(),
            &world.read_storage::<Transform>(),
        )
            .join()
        {
            let player_chunk = Chunk::absl_to_chunk_coords(Point3::from(*transform.translation()));
            thread_pool.scope(|s| {
                // Namespacing trick to get rust to move references into spawn() instaed of owned values.
                let dimension = &dimension;
                let generate_queue_set = &self.generate_queue_set;
                let channel = &channel;
                let dimension_dir = self.dimension_config.directory.as_path();
                for point in Sphere::new(
                    player_chunk,
                    self.dimension_config.generate_radius as FieldOf<Chunk>,
                ) {
                    s.spawn(move |_| {
                        let dimension = Arc::clone(&dimension);
                        let generate_queue_set = Arc::clone(&generate_queue_set);
                        let channel = Arc::clone(&channel);
                        let morton = MortonCode::from(point);
                        if !dimension.read().chunk_exists(morton)
                            && generate_queue_set.lock().contains(&morton)
                        {
                            println!("Generating chunk for {}", point);
                            {
                                generate_queue_set.lock().insert(morton);
                            }
                            if let Ok(_) = dimension.write()._create_or_load_chunk(
                                dimension_dir,
                                morton,
                                point,
                            ) {
                                channel
                                    .lock()
                                    .single_write(DimensionChunkEvent::NewChunkAt(morton));
                                generate_queue_set.lock().remove(&morton);
                            }
                        }
                    })
                }
            })
        }
        Trans::None
    }
}
