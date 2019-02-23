extern crate amethyst;
extern crate cubes_lib;
extern crate tokio;

use amethyst::{
    core::{nalgebra::Vector3, Transform, TransformBundle},
    input::InputBundle,
    prelude::*,
    renderer::*,
    utils::application_root_dir,
};

use cubes_lib::systems::{
    dimension_generation::DimensionBundle,
    player::{PlayerControlBundle, PlayerControlTag},
};

use std::path::PathBuf;

struct Gameplay {
    dimension_dir: PathBuf,
}

impl Gameplay {
    pub fn new(dimension_dir: PathBuf) -> Self {
        Gameplay { dimension_dir }
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
            .with_speed(12.0)
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
        Gameplay::new(PathBuf::from(dimension_dir)),
        game_data,
    )?;
    game.run();
    Ok(())
}
