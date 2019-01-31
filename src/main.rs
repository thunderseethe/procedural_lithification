extern crate amethyst;
extern crate cubes_lib;

use amethyst::{
    assets::AssetLoaderSystemData,
    core::{
        nalgebra::{Point3, Vector2, Vector3},
        Transform, TransformBundle,
    },
    input::InputBundle,
    prelude::*,
    renderer::*,
    utils::application_root_dir,
};

use cubes_lib::chunk::{Block, Chunk, DIRT_BLOCK};
use cubes_lib::octree::octant_dimensions::OctantDimensions;
use cubes_lib::systems::{PlayerControlBundle, PlayerControlTag};
use cubes_lib::terrain::Terrain;

fn create_cube(world: &mut World, mesh: MeshHandle, material: Material, point: Point3<u16>) {
    let mut pos: Transform = Transform::default();
    pos.set_xyz(point.x as f32, point.y as f32, point.z as f32);

    world
        .create_entity()
        .with(pos)
        .with(mesh)
        .with(material)
        .build();
}

fn pos_norm_tex(
    position: Vector3<f32>,
    tex_coord: Vector2<f32>,
    normal: Vector3<f32>,
) -> PosNormTex {
    PosNormTex {
        position,
        tex_coord,
        normal,
    }
}

fn cube_mesh(size_u32: u32) -> MeshData {
    let size: f32 = size_u32 as f32;
    // vertices
    let v = [
        Vector3::new(0.0, 0.0, size),
        Vector3::new(size, 0.0, size),
        Vector3::new(0.0, size, size),
        Vector3::new(size, size, size),
        Vector3::new(0.0, size, 0.0),
        Vector3::new(size, size, 0.0),
        Vector3::new(0.0, 0.0, 0.0),
        Vector3::new(size, 0.0, 0.0),
    ];
    // textures
    let t = [
        Vector2::new(0.0, 0.0),
        Vector2::new(size, 0.0),
        Vector2::new(0.0, size),
        Vector2::new(size, size),
    ];
    // normal
    let n = [
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(0.0, -1.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(-1.0, 0.0, 0.0),
    ];
    vec![
        // face 1
        pos_norm_tex(v[0], t[0], n[0]),
        pos_norm_tex(v[1], t[1], n[0]),
        pos_norm_tex(v[2], t[2], n[0]),
        pos_norm_tex(v[2], t[2], n[0]),
        pos_norm_tex(v[1], t[1], n[0]),
        pos_norm_tex(v[3], t[3], n[0]),
        // face 2
        pos_norm_tex(v[2], t[0], n[1]),
        pos_norm_tex(v[3], t[1], n[1]),
        pos_norm_tex(v[4], t[2], n[1]),
        pos_norm_tex(v[4], t[2], n[1]),
        pos_norm_tex(v[3], t[1], n[1]),
        pos_norm_tex(v[5], t[3], n[1]),
        // face 3
        pos_norm_tex(v[4], t[3], n[2]),
        pos_norm_tex(v[5], t[2], n[2]),
        pos_norm_tex(v[6], t[1], n[2]),
        pos_norm_tex(v[6], t[1], n[2]),
        pos_norm_tex(v[5], t[2], n[2]),
        pos_norm_tex(v[7], t[0], n[2]),
        // face 4
        pos_norm_tex(v[6], t[0], n[3]),
        pos_norm_tex(v[7], t[1], n[3]),
        pos_norm_tex(v[0], t[2], n[3]),
        pos_norm_tex(v[0], t[2], n[3]),
        pos_norm_tex(v[7], t[1], n[3]),
        pos_norm_tex(v[1], t[3], n[3]),
        // face 5
        pos_norm_tex(v[1], t[0], n[4]),
        pos_norm_tex(v[7], t[1], n[4]),
        pos_norm_tex(v[3], t[2], n[4]),
        pos_norm_tex(v[3], t[2], n[4]),
        pos_norm_tex(v[7], t[1], n[4]),
        pos_norm_tex(v[5], t[3], n[4]),
        // face 6
        pos_norm_tex(v[6], t[0], n[5]),
        pos_norm_tex(v[0], t[1], n[5]),
        pos_norm_tex(v[4], t[2], n[5]),
        pos_norm_tex(v[4], t[2], n[5]),
        pos_norm_tex(v[0], t[1], n[5]),
        pos_norm_tex(v[2], t[3], n[5]),
    ]
    .into()
}

fn render_chunk(world: &mut World, meshes: &[MeshHandle], material: &Material, chunk: &Chunk) {
    for (octant_dimensions, _) in chunk.iter() {
        let height = (octant_dimensions.diameter() as f32).log2() as usize;
        create_cube(
            world,
            meshes[height].clone(),
            material.clone(),
            octant_dimensions.bottom_left(),
        );
    }
}

struct Cubes;

impl SimpleState for Cubes {
    fn on_start(&mut self, data: StateData<GameData>) {
        let StateData { world, .. } = data;

        println!("Load mesh");

        let meshes = [
            world.exec(|loader: AssetLoaderSystemData<'_, Mesh>| {
                loader.load_from_data(cube_mesh(1), ())
            }),
            world.exec(|loader: AssetLoaderSystemData<'_, Mesh>| {
                loader.load_from_data(cube_mesh(2), ())
            }),
            world.exec(|loader: AssetLoaderSystemData<'_, Mesh>| {
                loader.load_from_data(cube_mesh(4), ())
            }),
            world.exec(|loader: AssetLoaderSystemData<'_, Mesh>| {
                loader.load_from_data(cube_mesh(8), ())
            }),
            world.exec(|loader: AssetLoaderSystemData<'_, Mesh>| {
                loader.load_from_data(cube_mesh(16), ())
            }),
            world.exec(|loader: AssetLoaderSystemData<'_, Mesh>| {
                loader.load_from_data(cube_mesh(32), ())
            }),
            world.exec(|loader: AssetLoaderSystemData<'_, Mesh>| {
                loader.load_from_data(cube_mesh(64), ())
            }),
            world.exec(|loader: AssetLoaderSystemData<'_, Mesh>| {
                loader.load_from_data(cube_mesh(128), ())
            }),
            world.exec(|loader: AssetLoaderSystemData<'_, Mesh>| {
                loader.load_from_data(cube_mesh(256), ())
            }),
        ];

        let albedo = world.exec(|loader: AssetLoaderSystemData<'_, Texture>| {
            loader.load(
                "textures/dirt.png",
                PngFormat,
                TextureMetadata::srgb()
                    .with_sampler(SamplerInfo::new(FilterMethod::Trilinear, WrapMode::Tile)),
                (),
            )
        });

        let material = Material {
            albedo: albedo,
            ..world.read_resource::<MaterialDefaults>().0.clone()
        };

        let terrain = Terrain::new(0.0);
        let chunk = terrain.generate_chunk();
        render_chunk(world, &meshes, &material, &chunk);

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
    let terrain = Terrain::new(0.0);
    let chunk = terrain.generate_chunk();
    println!("{:#?}", chunk);
    return Ok(());
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
            .with_speed(12.0)
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
