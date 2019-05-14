extern crate amethyst;
extern crate cubes_lib;
extern crate dirs;
extern crate morton_code;
extern crate parking_lot;
extern crate tokio;

use amethyst::{
    core::{
        nalgebra::{Point3, Vector3},
        ArcThreadPool, Transform, TransformBundle,
    },
    ecs::Join,
    input::InputBundle,
    prelude::*,
    renderer::{
        AmbientColor, Camera, DirectionalLight, DisplayConfig, DrawShaded, DrawSkybox, Event,
        KeyboardInput, Light, Pipeline, PosNormTex, Projection, RenderBundle, Rgba, Stage,
        VirtualKeyCode, WindowEvent,
    },
    shrev::EventChannel,
    ui::{DrawUi, UiBundle, UiCreator},
    utils::application_root_dir,
};
use cubes_lib::{
    chunk::Chunk,
    collision::{CollisionDetection, CollisionId},
    dimension::{ChunkMortonCode, Dimension, DimensionConfig},
    field::*,
    systems::{
        collision::CheckPlayerCollisionSystem,
        dimension::{DimensionBundle, DimensionChunkEvent, DimensionChunkEvent::NewChunkAt},
        player::{PlayerControlBundle, PlayerControlTag},
    },
    volume::Sphere,
};
use morton_code::MortonCode;
use parking_lot::Mutex;
use std::{collections::HashSet, path::PathBuf, sync::Arc};
use tokio::runtime::Runtime;

struct Gameplay {
    dimension_config: DimensionConfig,
    // Holds points that have been queued for generation so we don't re generate them
    generate_queue_set: Arc<Mutex<HashSet<ChunkMortonCode>>>,
}

impl Gameplay {
    pub fn new(dimension_config: DimensionConfig) -> Self {
        Gameplay {
            dimension_config,
            generate_queue_set: Arc::new(Mutex::new(HashSet::new())),
        }
    }

    pub fn init_dimension(
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
}

impl<'a, 'b> State<GameData<'a, 'b>, StateEvent> for Gameplay {
    fn on_start(&mut self, data: StateData<GameData>) {
        let StateData { world, .. } = data;
        world.add_resource(AmbientColor(Rgba::from([0.5; 3])));

        println!("Creating lights...");
        let light: Light = DirectionalLight {
            color: Rgba::WHITE,
            direction: [-1.0, 1.0, -1.0],
        }
        .into();

        world.create_entity().with(light).build();

        let mut collision = CollisionDetection::new();
        println!("Put camera");
        let mut transform = Transform::default();
        let player_pos = Point3::new(128.0, 128.0, 128.0);
        let player_handle = collision.add_player(player_pos);
        transform.set_position(player_pos.coords);
        transform.rotate_local(Vector3::y_axis(), std::f32::consts::PI);
        world
            .create_entity()
            .with(Camera::from(Projection::perspective(
                1.3,
                std::f32::consts::FRAC_PI_3,
            )))
            .with(transform)
            .with(PlayerControlTag::default())
            .with(CollisionId::new(player_handle))
            .build();

        let dimension = {
            let mut runtime = world.write_resource::<Runtime>();
            let mut chunk_channel = world.write_resource::<EventChannel<DimensionChunkEvent>>();
            self.init_dimension(&mut runtime, &mut chunk_channel)
        };
        world.add_resource(Arc::new(Mutex::new(dimension)));
        world.add_resource(collision);

        world.exec(|mut creator: UiCreator<'_>| {
            creator.create("ui/position.ron", ());
        })
    }

    fn on_stop(&mut self, data: StateData<GameData>) {
        let StateData { world, .. } = data;
        let dimension = world.write_resource::<Arc<Mutex<Dimension>>>();
        let mut runtime = world.write_resource::<Runtime>();
        dimension
            .lock()
            .store(self.dimension_config.directory.as_path(), &mut runtime);
    }

    fn handle_event(
        &mut self,
        _data: StateData<'_, GameData<'a, 'b>>,
        event: StateEvent,
    ) -> Trans<GameData<'a, 'b>, StateEvent> {
        if let StateEvent::Window(event) = &event {
            match event {
                Event::WindowEvent { event, .. } => match event {
                    WindowEvent::KeyboardInput {
                        input:
                            KeyboardInput {
                                virtual_keycode: Some(VirtualKeyCode::Escape),
                                ..
                            },
                        ..
                    }
                    | WindowEvent::CloseRequested => Trans::Quit,
                    _ => Trans::None,
                },
                _ => Trans::None,
            }
        } else {
            Trans::None
        }
    }

    fn update(&mut self, data: StateData<GameData>) -> Trans<GameData<'a, 'b>, StateEvent> {
        data.data.update(&data.world);
        Trans::None
    }

    fn fixed_update(&mut self, data: StateData<GameData>) -> Trans<GameData<'a, 'b>, StateEvent> {
        for (_, transform) in (
            &data.world.read_storage::<PlayerControlTag>(),
            &data.world.read_storage::<Transform>(),
        )
            .join()
        {
            let player_chunk = Chunk::absl_to_chunk_coords(Point3::from(*transform.translation()));
            let thread_pool = data.world.read_resource::<ArcThreadPool>().clone();
            let dimension = data.world.write_resource::<Arc<Mutex<Dimension>>>();
            let channel = Arc::new(Mutex::new(
                data.world
                    .write_resource::<EventChannel<DimensionChunkEvent>>(),
            ));
            thread_pool.scope(|s| {
                let dimension_ref = &dimension;
                let generate_queue_set_ref = &self.generate_queue_set;
                let channel_ref = &channel;
                let dimension_dir = self.dimension_config.directory.as_path();
                for point in Sphere::new(
                    player_chunk,
                    self.dimension_config.generate_radius as FieldOf<Chunk>,
                )
                .into_iter()
                {
                    s.spawn(move |_| {
                        let dimension = Arc::clone(dimension_ref);
                        let generate_queue_set = Arc::clone(generate_queue_set_ref);
                        let channel = Arc::clone(channel_ref);
                        let morton = MortonCode::from(&point);
                        if !dimension.lock().chunk_exists(morton)
                            && !generate_queue_set.lock().contains(&morton)
                        {
                            println!("Generating chunk for {:?}", point);
                            generate_queue_set.lock().insert(morton);
                            if let Ok(_) =
                                dimension
                                    .lock()
                                    ._create_or_load_chunk(dimension_dir, morton, point)
                            {
                                channel.lock().single_write(NewChunkAt(morton));
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

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let app_root = PathBuf::from(application_root_dir());
    let resources = app_root.join("resources");
    let display_config = DisplayConfig::load(resources.join("display_config.ron"));
    let key_bindings_path = resources.join("input.ron");
    let dimension_dir = resources.join("dimension");

    let pipeline_builder = Pipeline::build().with_stage(
        Stage::with_backbuffer()
            .clear_target([0.0, 0.0, 0.0, 1.0], 1.0)
            .with_pass(DrawSkybox::new())
            .with_pass(DrawShaded::<PosNormTex>::new())
            .with_pass(DrawUi::new()),
    );

    let game_data = GameDataBuilder::default()
        .with_bundle(
            PlayerControlBundle::<String, String>::new(
                Some(String::from("move_x")),
                Some(String::from("move_y")),
                Some(String::from("move_z")),
            )
            .with_speed(16.0)
            .with_sensitivity(0.1, 0.1),
        )?
        .with_bundle(TransformBundle::new().with_dep(&["player_movement"]))?
        .with_bundle(UiBundle::<String, String>::new())?
        .with_bundle(
            InputBundle::<String, String>::new().with_bindings_from_file(&key_bindings_path)?,
        )?
        .with_bundle(RenderBundle::new(pipeline_builder, Some(display_config)))?
        .with_bundle(DimensionBundle::new())?
        .with(CheckPlayerCollisionSystem, "check_player_collision", &[]);

    let mut game = Application::build(
        &resources,
        Gameplay::new(DimensionConfig::new(PathBuf::from(dimension_dir), 1)),
    )?
    .with_resource(Runtime::new().unwrap())
    .with_resource(EventChannel::<DimensionChunkEvent>::new())
    .build(game_data)?;

    game.run();
    Ok(())
}
