extern crate amethyst;
extern crate cubes_lib;
extern crate tokio;

use amethyst::{
    core::{
        nalgebra::{Point3, Vector3},
        Transform, TransformBundle,
    },
    input::InputBundle,
    prelude::*,
    renderer::*,
    utils::application_root_dir,
};
use cubes_lib::{
    dimension::Dimension,
    systems::{
        dimension_generation::DimensionBundle,
        player::{PlayerControlBundle, PlayerControlTag},
    },
    volume::Sphere,
};
use std::path::PathBuf;
use tokio::prelude::Future;
use tokio::runtime::Runtime;

struct Gameplay {
    dimension_dir: PathBuf,
    radius: i32,
}

impl Gameplay {
    pub fn new(dimension_dir: PathBuf, radius: i32) -> Self {
        Gameplay {
            dimension_dir,
            radius,
        }
    }

    pub fn init_dimension(&self, center: Point3<i32>) -> Dimension {
        std::fs::create_dir_all(&self.dimension_dir).expect("Unable to create dimension directory");
        let mut dimension = Dimension::new(self.dimension_dir.clone());
        let sphere = Sphere::new(center, self.radius);
        for point in sphere.iter() {
            match dimension.create_or_load_chunk(point) {
                Ok(_) => {}
                Err(err) => println!("{:?}", err),
            };
        }
        dimension
    }
}

impl SimpleState for Gameplay {
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

        println!("Put camera");
        let mut transform = Transform::default();
        transform.set_xyz(128.0, 256.0, 128.0);
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

        let runtime = Runtime::new().expect("Unable to create Tokio Runtime");
        world.add_resource(runtime);

        // Initialize a sphere of chunks around the origin.
        let dimension = self.init_dimension(Point3::new(0, 0, 0));
        {
            dimension.store(&mut world.write_resource::<Runtime>());
        }
        world.add_resource(dimension);
    }

    fn handle_event(
        &mut self,
        _data: StateData<'_, GameData<'_, '_>>,
        event: StateEvent,
    ) -> SimpleTrans {
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
}

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let app_root = application_root_dir();
    let resources = format!("{}/resources", app_root);
    let display_config = DisplayConfig::load(format!("{}/display_config.ron", resources));
    let key_bindings_path = format!("{}/input.ron", resources);
    let dimension_dir = format!("{}/dimension/", resources);

    let pipeline_builder = Pipeline::build().with_stage(
        Stage::with_backbuffer()
            .clear_target([0.0, 0.0, 0.0, 1.0], 1.0)
            .with_pass(DrawSkybox::new())
            .with_pass(DrawShaded::<PosNormTangTex>::new()),
    );

    let game_data = GameDataBuilder::default()
        .with_bundle(
            PlayerControlBundle::<String, String>::new(
                Some(String::from("move_x")),
                Some(String::from("move_y")),
                Some(String::from("move_z")),
            )
            .with_speed(124.0)
            .with_sensitivity(0.1, 0.1),
        )?
        .with_bundle(TransformBundle::new().with_dep(&["player_movement"]))?
        .with_bundle(
            InputBundle::<String, String>::new().with_bindings_from_file(&key_bindings_path)?,
        )?
        .with_bundle(RenderBundle::new(pipeline_builder, Some(display_config)))?
        .with_bundle(DimensionBundle::new())?;
    let mut game = Application::new(
        &resources,
        Gameplay::new(PathBuf::from(dimension_dir), 4),
        game_data,
    )?;
    game.run();
    Ok(())
}
