extern crate serde_derive;

extern crate amethyst;
extern crate noise;
extern crate rayon;
extern crate serde;

use amethyst::{
    assets::AssetLoaderSystemData,
    core::{
        nalgebra::{Point3, Vector3},
        Transform, TransformBundle,
    },
    input::InputBundle,
    prelude::*,
    renderer::*,
    utils::application_root_dir,
};

mod chunk;
mod octree;
mod systems;
mod terrain;

use crate::chunk::Chunk;
use crate::systems::{PlayerControlBundle, PlayerControlTag};

fn create_cube(world: &mut World, mesh: MeshHandle, texture: &TextureHandle, point: &Point3<i32>) {
    let mat_defaults = world.read_resource::<MaterialDefaults>().0.clone();
    let mut pos: Transform = Transform::default();
    pos.set_xyz(
        point.x as f32 * 2.0,
        point.y as f32 * 2.0,
        point.z as f32 * 2.0,
    );

    let material = Material {
        albedo: texture.clone(),
        ..mat_defaults.clone()
    };

    world
        .create_entity()
        .with(pos)
        .with(mesh)
        .with(material)
        .build();
}

fn render_chunk(world: &mut World, mesh: MeshHandle, texture: &TextureHandle, chunk: &Chunk) {
    for (pos, _) in chunk.iter() {
        create_cube(world, mesh.clone(), texture, &pos);
    }
}

fn create_chunk() -> Chunk {
    Chunk::default()
        .place_block(Point3::new(1, 0, 0), 0)
        .place_block(Point3::new(0, 1, 0), 0)
        .place_block(Point3::new(0, 0, 1), 0)
        .place_block(Point3::new(0, 0, -1), 0)
        .place_block(Point3::new(-1, 0, 0), 0)
        .place_block(Point3::new(0, -1, 0), 0)
}

struct Cubes;

impl SimpleState for Cubes {
    fn on_start(&mut self, data: StateData<GameData>) {
        let StateData { world, .. } = data;

        println!("Load mesh");
        let mesh = world.exec(|loader: AssetLoaderSystemData<'_, Mesh>| {
            loader.load("mesh/cube.obj", ObjFormat, (), ())
        });

        let albedo = world.exec(|loader: AssetLoaderSystemData<'_, Texture>| {
            loader.load("textures/dirt.png", PngFormat, TextureMetadata::srgb(), ())
        });

        let chunk = create_chunk();
        render_chunk(world, mesh.clone(), &albedo, &chunk);

        println!("Put camera");

        let mut transform = Transform::default();
        transform.set_xyz(0.0, 0.0, -12.0);
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
}

fn main() -> amethyst::Result<()> {
    amethyst::start_logger(Default::default());

    let app_root = application_root_dir();
    let resources = format!("{}/resources", app_root);
    let display_config = DisplayConfig::load(format!("{}/display_config.ron", resources));
    let key_bindings_path = format!("{}/input.ron", resources);

    let pipeline_builder = Pipeline::build().with_stage(
        Stage::with_backbuffer()
            .clear_target([0.0, 0.0, 0.0, 1.0], 1.0)
            .with_pass(DrawSkybox::new())
            .with_pass(DrawFlat::<PosNormTex>::new()),
    );

    let game_data = GameDataBuilder::default()
        .with_bundle(
            PlayerControlBundle::<String, String>::new(
                Some(String::from("move_x")),
                Some(String::from("move_y")),
                Some(String::from("move_z")),
            )
            .with_speed(4.0)
            .with_sensitivity(0.1, 0.1),
        )?
        .with_bundle(TransformBundle::new().with_dep(&["player_movement"]))?
        .with_bundle(
            InputBundle::<String, String>::new().with_bindings_from_file(&key_bindings_path)?,
        )?
        .with_bundle(RenderBundle::new(pipeline_builder, Some(display_config)))?;
    let mut game = Application::new(&resources, Cubes, game_data)?;
    game.run();
    Ok(())
}
