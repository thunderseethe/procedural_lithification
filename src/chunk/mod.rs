use crate::octree::new_octree::{Map, *};
use amethyst::{
    core::nalgebra::{convert, Point3, Vector2, Vector3},
    renderer::{MeshData, PosNormTex},
};
use rayon::iter::{plumbing::*, *};
use serde::{Deserialize, Deserializer, Serialize};
use std::borrow::Borrow;

pub mod block;
pub mod chunk_builder;
pub mod file_format;
pub mod mesher;

use block::Block;
use mesher::Mesher;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub struct Chunk {
    pub pos: Point3<i32>,
    octree: OctreeOf<Self>,
}

pub trait HasOctree {
    type Octree: OctreeTypes + HasPosition;
}
impl HasOctree for Chunk {
    type Octree = Octree8<Block, u8>;
}
pub type OctreeOf<T> = <T as HasOctree>::Octree;

impl ElementType for Chunk {
    type Element = ElementOf<OctreeOf<Chunk>>;
}
impl FieldType for Chunk {
    type Field = FieldOf<OctreeOf<Chunk>>;
}

impl Chunk {
    pub fn new(pos: Point3<i32>, octree: Octree8<Block, u8>) -> Self {
        Chunk { pos, octree }
    }

    pub fn with_block(pos: Point3<i32>, block: Block) -> Self {
        Chunk {
            pos,
            octree: Octree8::at_origin(Some(block)),
        }
    }

    pub fn with_empty(pos: Point3<i32>) -> Self {
        Chunk {
            pos,
            octree: Octree8::at_origin(None),
        }
    }

    pub fn get_block<P>(&self, pos: P) -> Option<Block>
    where
        P: Borrow<PositionOf<OctreeOf<Self>>>,
    {
        self.octree.get(pos).map(|arc_block| *arc_block)
    }

    pub fn place_block<P>(&mut self, pos: P, block: Block) -> &mut Self
    where
        P: Borrow<PositionOf<OctreeOf<Self>>>,
    {
        self.octree = self.octree.insert(pos, block);
        self
    }

    pub fn generate_mesh(&self) -> Option<Vec<(Point3<f32>, MeshData)>> {
        let chunk_render_pos: Point3<f32> = convert(self.pos * 256);
        self.octree.map(
            || None,
            |_| {
                // Trivial cube
                let mesh = cube_mesh(256.0).into();
                Some(vec![(chunk_render_pos, mesh)])
            },
            |children| {
                Some(
                    children
                        .par_iter()
                        .flat_map(|octree| {
                            octree.map(
                                || vec![],
                                |_| {
                                    let octree_offset: Vector3<f32> =
                                        convert(octree.root_point().coords);
                                    let mesh = cube_mesh(octree.get_diameter() as f32).into();
                                    vec![(chunk_render_pos + octree_offset, mesh)]
                                },
                                |children| {
                                    children
                                        .par_iter()
                                        .filter_map(|octree| {
                                            use typenum::U64;
                                            let octree_root_offset: Vector3<f32> =
                                                convert(octree.root_point().coords);

                                            octree.map(
                                                || None,
                                                |_| {
                                                    Some((
                                                        chunk_render_pos + octree_root_offset,
                                                        cube_mesh(octree.get_diameter() as f32)
                                                            .into(),
                                                    ))
                                                },
                                                |_| {
                                                    let mesher =
                                                        Mesher::<Octree<Block, u8, U64>>::new(
                                                            octree.as_ref(),
                                                        );
                                                    let quads = mesher.generate_quads_array();
                                                    let mut mesh_data: Vec<PosNormTex> =
                                                        Vec::with_capacity(quads.len() * 6);
                                                    mesh_data.extend(
                                                        quads
                                                            .into_iter()
                                                            .flat_map(|quad| quad.mesh_coords()),
                                                    );
                                                    Some((
                                                        chunk_render_pos + octree_root_offset,
                                                        mesh_data.into(),
                                                    ))
                                                },
                                            )
                                        })
                                        .collect::<Vec<(Point3<f32>, MeshData)>>()
                                },
                            )
                        })
                        .collect(),
                )
            },
        )
    }

    pub fn iter<'a>(&'a self) -> <&'a OctreeOf<Self> as IntoIterator>::IntoIter {
        self.octree.into_iter()
    }
}

impl<'de> Deserialize<'de> for Chunk
where
    OctreeOf<Chunk>: Deserialize<'de>,
{
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        use serde::de::*;
        use std::fmt;
        #[derive(Deserialize)]
        #[serde(field_identifier, rename_all = "lowercase")]
        enum Field {
            Pos,
            Octree,
        }

        struct ChunkVisitor;
        impl<'de> Visitor<'de> for ChunkVisitor {
            type Value = Chunk;

            fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
                formatter.write_str("struct Chunk")
            }

            fn visit_seq<V>(self, mut seq: V) -> Result<Self::Value, V::Error>
            where
                V: SeqAccess<'de>,
            {
                let pos = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(0, &self))?;
                let octree = seq
                    .next_element()?
                    .ok_or_else(|| Error::invalid_length(1, &self))?;
                Ok(Chunk::new(pos, octree))
            }

            fn visit_map<V>(self, mut map: V) -> Result<Self::Value, V::Error>
            where
                V: MapAccess<'de>,
            {
                let mut pos = None;
                let mut octree = None;
                while let Some(key) = map.next_key()? {
                    match key {
                        Field::Pos => {
                            if pos.is_some() {
                                return Err(Error::duplicate_field("pos"));
                            }
                            pos = Some(map.next_value()?);
                        }
                        Field::Octree => {
                            if octree.is_some() {
                                return Err(Error::duplicate_field("octree"));
                            }
                            octree = Some(map.next_value()?);
                        }
                    }
                }
                let pos = pos.ok_or_else(|| Error::missing_field("pos"))?;
                let octree = octree.ok_or_else(|| Error::missing_field("octree"))?;
                Ok(Chunk::new(pos, octree))
            }
        }

        const FIELDS: &'static [&'static str] = &["pos", "octree"];
        deserializer.deserialize_struct("OctreeLevel", FIELDS, ChunkVisitor)
    }
}

pub fn cube_mesh(size: f32) -> Vec<PosNormTex> {
    // normal
    let n = [
        Vector3::new(0.0, 0.0, 1.0),
        Vector3::new(0.0, 1.0, 0.0),
        Vector3::new(0.0, 0.0, -1.0),
        Vector3::new(0.0, -1.0, 0.0),
        Vector3::new(1.0, 0.0, 0.0),
        Vector3::new(-1.0, 0.0, 0.0),
    ];

    let mut vec = Vec::with_capacity(36);

    // textures
    let tx = [
        Vector2::new(0.0, 0.0),
        Vector2::new(size, 0.0),
        Vector2::new(0.0, size),
        Vector2::new(size, size),
    ];
    // vertices
    let v = [
        /*base +*/ Vector3::new(0.0, 0.0, size), // 0
        /*base +*/ Vector3::new(size, 0.0, size), // 1
        /*base +*/ Vector3::new(0.0, size, size), // 2
        /*base +*/ Vector3::new(size, size, size), // 3
        /*base +*/ Vector3::new(0.0, size, 0.0), // 4
        /*base +*/ Vector3::new(size, size, 0.0), // 5
        /*base +*/ Vector3::new(0.0, 0.0, 0.0), // 6
        /*base +*/ Vector3::new(size, 0.0, 0.0), // 7
    ];
    // Back
    vec.push(pos_norm_tex(v[0], n[0], tx[0])); // (0, 0, 1)
    vec.push(pos_norm_tex(v[1], n[0], tx[1])); // (1, 0, 1)
    vec.push(pos_norm_tex(v[2], n[0], tx[2])); // (0, 1, 1)
    vec.push(pos_norm_tex(v[2], n[0], tx[2])); // (0, 1, 1)
    vec.push(pos_norm_tex(v[1], n[0], tx[1])); // (1, 0, 1)
    vec.push(pos_norm_tex(v[3], n[0], tx[3])); // (1, 1, 1)
                                               // Up
    vec.push(pos_norm_tex(v[2], n[1], tx[0])); // (0, 1, 1)
    vec.push(pos_norm_tex(v[3], n[1], tx[1])); // (1, 1, 1)
    vec.push(pos_norm_tex(v[4], n[1], tx[2])); // (0, 1, 0)
    vec.push(pos_norm_tex(v[4], n[1], tx[2])); // (0, 1, 0)
    vec.push(pos_norm_tex(v[3], n[1], tx[1])); // (1, 1, 1)
    vec.push(pos_norm_tex(v[5], n[1], tx[3])); // (1, 1, 0)
                                               // Front
    vec.push(pos_norm_tex(v[4], n[2], tx[3])); // (0, 1, 0)
    vec.push(pos_norm_tex(v[5], n[2], tx[2])); // (1, 1, 0)
    vec.push(pos_norm_tex(v[6], n[2], tx[1])); // (0, 0, 0)
    vec.push(pos_norm_tex(v[6], n[2], tx[1])); // (0, 0, 0)
    vec.push(pos_norm_tex(v[5], n[2], tx[2])); // (1, 1, 0)
    vec.push(pos_norm_tex(v[7], n[2], tx[0])); // (1, 0, 0)
                                               // Down
    vec.push(pos_norm_tex(v[6], n[3], tx[0])); // (0, 0, 0)
    vec.push(pos_norm_tex(v[7], n[3], tx[1])); // (1, 0, 0)
    vec.push(pos_norm_tex(v[0], n[3], tx[2])); // (0, 0, 1)
    vec.push(pos_norm_tex(v[0], n[3], tx[2])); // (0, 0, 1)
    vec.push(pos_norm_tex(v[7], n[3], tx[1])); // (1, 0, 0)
    vec.push(pos_norm_tex(v[1], n[3], tx[3])); // (1, 0, 1)
                                               // Right
    vec.push(pos_norm_tex(v[1], n[4], tx[0])); // (1, 0, 1)
    vec.push(pos_norm_tex(v[7], n[4], tx[1])); // (1, 0, 0)
    vec.push(pos_norm_tex(v[3], n[4], tx[2])); // (1, 1, 1)
    vec.push(pos_norm_tex(v[3], n[4], tx[2])); // (1, 1, 1)
    vec.push(pos_norm_tex(v[7], n[4], tx[1])); // (1, 0, 0)
    vec.push(pos_norm_tex(v[5], n[4], tx[3])); // (1, 1, 0)
                                               // Left
    vec.push(pos_norm_tex(v[6], n[5], tx[0])); // (0, 0, 0)
    vec.push(pos_norm_tex(v[0], n[5], tx[1])); // (0, 0, 1)
    vec.push(pos_norm_tex(v[4], n[5], tx[2])); // (0, 1, 0)
    vec.push(pos_norm_tex(v[4], n[5], tx[2])); // (0, 1, 0)
    vec.push(pos_norm_tex(v[0], n[5], tx[1])); // (0, 0, 1)
    vec.push(pos_norm_tex(v[2], n[5], tx[3])); // (0, 1, 1)
    return vec;
}

fn pos_norm_tex(
    position: Vector3<f32>,
    normal: Vector3<f32>,
    tex_coord: Vector2<f32>,
) -> PosNormTex {
    PosNormTex {
        position,
        normal,
        tex_coord,
    }
}

#[cfg(test)]
mod test {
    use super::{Chunk, Point3};
    use crate::octree::Octree;
    use std::collections::HashSet;

    #[test]
    fn test_chunk_insertions() {
        let mut points = HashSet::new();
        let mut chunk = Chunk::with_empty(Point3::origin());
        for _ in 0..1000 {
            let p = Point3::new(
                rand::random::<u8>().into(),
                rand::random::<u8>().into(),
                rand::random::<u8>().into(),
            );
            chunk.place_block(&p, 1234);
            points.insert(p);
        }

        for point in points {
            assert_eq!(chunk.get_block(point), Some(1234));
        }
    }

}
