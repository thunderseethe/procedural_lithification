extern crate amethyst;
extern crate cubes_lib;
extern crate dirs;
extern crate morton_code;
extern crate parking_lot;
extern crate tokio;

use amethyst::{
    core::{
        nalgebra::{Point3, Vector3},
        Transform, TransformBundle,
    },
    input::InputBundle,
    network::{NetConnection, NetworkBundle},
    prelude::*,
    renderer::{
        AmbientColor, Camera, DirectionalLight, DisplayConfig, DrawShaded, DrawSkybox, Event,
        KeyboardInput, Light, Pipeline, PosNormTex, Projection, RenderBundle, Rgba, Stage,
        VirtualKeyCode, WindowEvent,
    },
    ui::{DrawUi, UiBundle, UiCreator},
    utils::application_root_dir,
};
use cubes_lib::{
    collision::{CollisionDetection, CollisionId},
    protocol::ClientProtocol,
    systems::{
        collision::CheckPlayerCollisionSystem,
        player::{PlayerControlBundle, PlayerControlTag, PlayerEntityTag},
    },
};
use std::path::PathBuf;

const SERVER: &str = "127.0.0.1:3455";

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let app_root = PathBuf::from(application_root_dir());
    let resources = app_root.join("resources");
    let display_config = DisplayConfig::load(resources.join("display_config.ron"));
    let key_bindings_path = resources.join("input.ron");

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
        .with_bundle(NetworkBundle::<ClientProtocol>::new(
            SERVER.parse().unwrap(),
            vec![],
        ))?
        .with_bundle(TransformBundle::new().with_dep(&["player_movement"]))?
        .with_bundle(UiBundle::<String, String>::new())?
        .with_bundle(
            InputBundle::<String, String>::new().with_bindings_from_file(&key_bindings_path)?,
        )?
        .with_bundle(RenderBundle::new(pipeline_builder, Some(display_config)))?
        .with(CheckPlayerCollisionSystem, "check_player_collision", &[]);

    let mut game = Application::build(&resources, ClientDimensionState::new())?.build(game_data)?;

    game.run();
    Ok(())
}

struct ClientDimensionState;

impl ClientDimensionState {
    pub fn new() -> Self {
        ClientDimensionState
    }

    fn register_components(&self, world: &mut World) {
        world.register::<PlayerEntityTag>();
        world.register::<NetConnection<ClientProtocol>>();
    }
}

impl<'a, 'b> State<GameData<'a, 'b>, StateEvent> for ClientDimensionState {
    fn on_start(&mut self, data: StateData<GameData>) {
        let StateData { mut world, .. } = data;
        self.register_components(&mut world);

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
            .with(PlayerEntityTag::default())
            .with(CollisionId::new(player_handle))
            .build();

        world.add_resource(collision);

        world
            .create_entity()
            .with(NetConnection::<ClientProtocol>::new(
                SERVER.parse().unwrap(),
            ))
            .build();

        world.exec(|mut creator: UiCreator<'_>| {
            creator.create("ui/position.ron", ());
        })
    }

    fn update(
        &mut self,
        data: StateData<'_, GameData<'a, 'b>>,
    ) -> Trans<GameData<'a, 'b>, StateEvent> {
        data.data.update(data.world);
        Trans::None
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
}
