extern crate amethyst;
extern crate cubes_lib;
extern crate dirs;
extern crate parking_lot;
extern crate tokio;

use amethyst::{
    assets::AssetLoaderSystemData,
    core::{
        nalgebra::{Point3, Vector3},
        ArcThreadPool, Transform, TransformBundle,
    },
    ecs::{Join, ReadExpect},
    input::InputBundle,
    prelude::*,
    renderer::{
        AmbientColor, Camera, DirectionalLight, DisplayConfig, DrawShaded, DrawSkybox, Event,
        FilterMethod, KeyboardInput, Light, Material, MaterialDefaults, Mesh, MeshHandle, Pipeline,
        PngFormat, PosNormTex, Projection, RenderBundle, Rgba, SamplerInfo, Stage, Texture,
        TextureMetadata, VirtualKeyCode, WindowEvent, WrapMode,
    },
    shrev::EventChannel,
    ui::{DrawUi, UiBundle, UiCreator},
    utils::application_root_dir,
};
use cubes_lib::{
    collision::{CollisionDetection, CollisionDetectionError},
    dimension::{morton_code::MortonCode, Dimension, DimensionConfig},
    systems::{
        collision::CheckPlayerCollisionSystem,
        dimension::{DimensionBundle, DimensionChunkEvent},
        player::{PlayerControlBundle, PlayerControlTag},
    },
    volume::Sphere,
};
use parking_lot::Mutex;
use std::{collections::HashSet, path::PathBuf, sync::Arc};
use tokio::runtime::Runtime;

struct Gameplay {
    dimension_config: DimensionConfig,
    // Holds points that have been queued for generation so we don't re generate them
    generate_queue_set: Arc<Mutex<HashSet<MortonCode>>>,
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
        collision: &mut CollisionDetection,
    ) -> Dimension {
        std::fs::create_dir_all(&self.dimension_config.directory)
            .expect("Unable to create dimension directory");
        let mut dimension = Dimension::default();
        //for point in Sphere::with_origin(4).into_iter() {
        //    dimension
        //        ._create_or_load_chunk(
        //            self.dimension_config.directory.as_path(),
        //            MortonCode::from(&point),
        //            point,
        //        )
        //        .expect(&format!("Failed to create intial chunk at {:?}", point));
        //}
        {
            let chunk = dimension
                ._create_or_load_chunk(
                    self.dimension_config.directory.as_path(),
                    MortonCode::from_raw(0),
                    Point3::origin(),
                )
                .expect("Failed to generate chunk at origin");
            collision
                .add_chunk(&chunk)
                .expect("Chunk at origin already present");
        }
        dimension.store(self.dimension_config.directory.as_path(), runtime);
        dimension
    }

    pub fn render_initial_dimension(world: &mut World) {
        let material = world.exec(
            |(texture_loader, material_defaults): (
                AssetLoaderSystemData<Texture>,
                ReadExpect<MaterialDefaults>,
            )| {
                let albedo = texture_loader.load(
                    "textures/dirt.png",
                    PngFormat,
                    TextureMetadata::srgb()
                        .with_sampler(SamplerInfo::new(FilterMethod::Trilinear, WrapMode::Tile)),
                    (),
                );
                let default = material_defaults.0.clone();
                Material { albedo, ..default }
            },
        );
        let meshes: Vec<(Point3<f32>, MeshHandle)> = world.exec(
            |(dimension, mesh_loader): (
                ReadExpect<Arc<Mutex<Dimension>>>,
                AssetLoaderSystemData<Mesh>,
            )| {
                dimension
                    .lock()
                    .iter()
                    .filter_map(|mtx_chunk| {
                        let chunk = mtx_chunk.lock();
                        chunk.generate_mesh()
                    })
                    .flatten()
                    .map(move |(point, mesh_data)| {
                        (point, mesh_loader.load_from_data(mesh_data, ()))
                    })
                    .collect()
            },
        );
        // I miss us
        for (point, mesh) in meshes {
            let mut pos: Transform = Transform::default();
            pos.set_xyz(point.x, point.y, point.z);
            world
                .create_entity()
                .with(pos)
                .with(mesh)
                .with(material.clone())
                .build();
        }
    }

    fn convert_to_chunk_coord(vec: &Vector3<f32>) -> Point3<i32> {
        let x = (vec.x / 256.0).floor() as i32;
        let y = (vec.y / 256.0).floor() as i32;
        let z = (vec.z / 256.0).floor() as i32;
        Point3::new(x, y, z)
    }
}

impl<'a, 'b> State<GameData<'a, 'b>, StateEvent> for Gameplay {
    fn on_start(&mut self, data: StateData<GameData>) {
        let StateData { mut world, .. } = data;
        world.add_resource(AmbientColor(Rgba::from([0.5; 3])));

        println!("Creating lights...");
        let light: Light = DirectionalLight {
            color: Rgba::WHITE,
            direction: [-1.0, 1.0, -1.0],
        }
        .into();

        world.create_entity().with(light).build();

        println!("Put camera");
        let mut transform = Transform::default();
        let player_pos = Point3::new(128.0, 128.0, 128.0);
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
            .build();

        let mut collision = CollisionDetection::new(player_pos);
        let dimension = {
            let mut runtime = world.write_resource::<Runtime>();
            self.init_dimension(&mut runtime, &mut collision)
        };
        world.add_resource(Arc::new(Mutex::new(dimension)));
        world.add_resource(Arc::new(Mutex::new(collision)));
        Gameplay::render_initial_dimension(&mut world);
        println!("Rendered Initial Dimension");

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
            let player_chunk = Gameplay::convert_to_chunk_coord(transform.translation());
            let thread_pool = data.world.read_resource::<ArcThreadPool>().clone();
            let dimension = data.world.write_resource::<Arc<Mutex<Dimension>>>();
            let collision = data
                .world
                .write_resource::<Arc<Mutex<CollisionDetection>>>();
            let channel = Arc::new(Mutex::new(
                data.world
                    .write_resource::<EventChannel<DimensionChunkEvent>>(),
            ));
            thread_pool.scope(|s| {
                let dimension_ref = &dimension;
                let collision_ref = &collision;
                let generate_queue_set_ref = &self.generate_queue_set;
                let channel_ref = &channel;
                let dimension_dir = self.dimension_config.directory.as_path();
                for point in Sphere::new(player_chunk, self.dimension_config.generate_radius as i32)
                    .into_iter()
                {
                    s.spawn(move |_| {
                        let collision = Arc::clone(collision_ref);
                        let dimension = Arc::clone(dimension_ref);
                        let generate_queue_set = Arc::clone(generate_queue_set_ref);
                        let channel = Arc::clone(channel_ref);
                        let morton = MortonCode::from(&point);
                        if !dimension.lock().chunk_exists(morton)
                            && !generate_queue_set.lock().contains(&morton)
                        {
                            println!("Generating chunk for {:?}", point);
                            generate_queue_set.lock().insert(morton);
                            if let Ok(chunk) =
                                dimension
                                    .lock()
                                    ._create_or_load_chunk(dimension_dir, morton, point)
                            {
                                collision.lock().add_chunk(&chunk).unwrap_or_else(
                                    |err| match err {
                                        CollisionDetectionError::ChunkAlreadyPresent => println!(
                                            "Chunk already loading into collision detection {}",
                                            point
                                        ),
                                    },
                                );
                                chunk.generate_mesh().map(|chunk_render_info| {
                                    channel.lock().iter_write(chunk_render_info.into_iter().map(
                                        |(point, mesh_data)| {
                                            DimensionChunkEvent::GeneratedChunk(point, mesh_data)
                                        },
                                    ))
                                });
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
