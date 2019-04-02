use super::DimensionChunkEvent;
use amethyst::{
    assets::{AssetLoaderSystemData, AssetStorage, Loader},
    core::Transform,
    ecs::{Entities, Read, ReadExpect, Resources, System, SystemData, WriteStorage},
    renderer::{
        FilterMethod, Material, MaterialDefaults, Mesh, MeshHandle, PngFormat, SamplerInfo,
        Texture, TextureHandle, TextureMetadata, WrapMode,
    },
    shrev::{EventChannel, ReaderId},
};

pub struct RenderDimensionSystem {
    albedo: Option<TextureHandle>,
    reader: Option<ReaderId<DimensionChunkEvent>>,
}

impl RenderDimensionSystem {
    pub fn new() -> Self {
        RenderDimensionSystem {
            albedo: None,
            reader: None,
        }
    }
}

impl<'a> System<'a> for RenderDimensionSystem {
    type SystemData = (
        Entities<'a>,
        ReadExpect<'a, MaterialDefaults>,
        Read<'a, EventChannel<DimensionChunkEvent>>,
        WriteStorage<'a, Transform>,
        WriteStorage<'a, MeshHandle>,
        WriteStorage<'a, Material>,
        AssetLoaderSystemData<'a, Mesh>,
    );

    fn setup(&mut self, res: &mut Resources) {
        Self::SystemData::setup(res);
        Read::<'_, AssetStorage<Texture>>::setup(res);
        self.reader = Some(
            res.fetch_mut::<EventChannel<DimensionChunkEvent>>()
                .register_reader(),
        );
        let loader = res.fetch::<Loader>();
        let tex_storage = res.fetch();
        self.albedo = Some(
            loader.load(
                "textures/dirt.png",
                PngFormat,
                TextureMetadata::srgb()
                    .with_sampler(SamplerInfo::new(FilterMethod::Trilinear, WrapMode::Tile)),
                (),
                &tex_storage,
            ),
        );
    }

    fn run(
        &mut self,
        (
            entities,
            material_defaults,
            render_chunk_event_reader,
            mut transforms,
            mut meshes,
            mut materials,
            mesh_loader,
        ): Self::SystemData,
    ) {
        for event in render_chunk_event_reader.read(self.reader.as_mut().unwrap()) {
            match event {
                DimensionChunkEvent::GeneratedChunk(p, mesh_data) => {
                    let mut pos = Transform::default();
                    pos.set_xyz(p.x, p.y, p.z);
                    entities
                        .build_entity()
                        .with(pos, &mut transforms)
                        .with(
                            mesh_loader.load_from_data(mesh_data.clone(), ()),
                            &mut meshes,
                        )
                        .with(
                            Material {
                                albedo: self.albedo.clone().unwrap(),
                                ..material_defaults.0.clone()
                            },
                            &mut materials,
                        )
                        .build();
                }
            }
        }
    }
}
