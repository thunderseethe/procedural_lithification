use crate::dimension::Dimension;
use amethyst::{
    assets::AssetLoaderSystemData,
    core::{
        bundle::{Result, SystemBundle},
        nalgebra::Point3,
        shred::Fetch,
        specs::DispatcherBuilder,
        Transform,
    },
    ecs::{Entities, Read, ReadExpect, Resources, System, WriteStorage},
    prelude::*,
    renderer::*,
};
use rayon::iter::ParallelIterator;
use tokio;
use tokio::{prelude::*, runtime::Runtime};

struct RenderDimensionSystem {
    material: Option<Material>,
    run: bool,
}

impl RenderDimensionSystem {
    pub fn new() -> Self {
        RenderDimensionSystem {
            run: false,
            material: None,
        }
    }
}

impl<'a> System<'a> for RenderDimensionSystem {
    type SystemData = (
        Entities<'a>,
        Read<'a, Dimension>,
        ReadExpect<'a, MaterialDefaults>,
        WriteStorage<'a, Transform>,
        WriteStorage<'a, MeshHandle>,
        WriteStorage<'a, Material>,
        AssetLoaderSystemData<'a, Mesh>,
        AssetLoaderSystemData<'a, Texture>,
    );

    fn setup(&mut self, res: &mut Resources) {
        let mut dimension: Dimension = Dimension::default();
        dimension.create_or_load_chunk(Point3::new(0, 0, 0));
        dimension.create_or_load_chunk(Point3::new(1, 0, 0));
        dimension.create_or_load_chunk(Point3::new(0, 0, 1));
        let mut runtime = Runtime::new().unwrap();
        {
            dimension.store(&mut runtime);
        }
        runtime.shutdown_on_idle().wait();
        res.insert(dimension);

        self.run = false;
    }

    fn run(
        &mut self,
        (
            entities,
            dimension,
            material_defaults,
            mut transforms,
            mut meshes,
            mut materials,
            mesh_loader,
            texture_loader,
        ): Self::SystemData,
    ) {
        if !self.run {
            self.run = true;
            let albedo = texture_loader.load(
                "textures/dirt.png",
                PngFormat,
                TextureMetadata::srgb()
                    .with_sampler(SamplerInfo::new(FilterMethod::Trilinear, WrapMode::Tile)),
                (),
            );
            let default = material_defaults.0.clone();
            self.material = Some(Material { albedo, ..default });
            (&*dimension)
                .into_iter()
                .map(|mtx_chunk| {
                    let chunk = mtx_chunk.lock();
                    (chunk.pos, chunk.generate_mesh())
                })
                .for_each(|(point, mesh_data)| {
                    let mut pos: Transform = Transform::default();
                    pos.set_xyz(
                        point.x as f32 * 256.0,
                        point.y as f32 * 256.0,
                        point.z as f32 * 256.0,
                    );
                    entities
                        .build_entity()
                        .with(pos, &mut transforms)
                        .with(mesh_loader.load_from_data(mesh_data, ()), &mut meshes)
                        .with(self.material.clone().unwrap(), &mut materials)
                        .build();
                })
        }
    }
}

pub struct DimensionBundle();
impl DimensionBundle {
    pub fn new() -> Self {
        DimensionBundle()
    }
}

impl<'a, 'b> SystemBundle<'a, 'b> for DimensionBundle {
    fn build(self, builder: &mut DispatcherBuilder<'a, 'b>) -> Result<()> {
        builder.add(RenderDimensionSystem::new(), "render_dimension", &[]);
        Ok(())
    }
}
