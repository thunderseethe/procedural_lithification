use crate::{
    chunk::Chunk,
    field::*,
    morton_code::MortonCode,
    terrain::{DefaultGenerateBlock, Terrain},
};
use amethyst::core::nalgebra::Point3;
use parking_lot::{Mutex, MutexGuard};
use std::{
    borrow::Borrow,
    path::{Path, PathBuf},
};
use tokio::runtime::Runtime;

mod storage;

use storage::DimensionStorage;

pub struct DimensionConfig {
    pub directory: PathBuf,
    pub generate_radius: usize,
}
impl Default for DimensionConfig {
    fn default() -> Self {
        DimensionConfig {
            directory: PathBuf::from("./resources/dimension/"),
            generate_radius: 4,
        }
    }
}
impl DimensionConfig {
    pub fn new(directory: PathBuf, generate_radius: usize) -> Self {
        DimensionConfig {
            directory,
            generate_radius,
        }
    }
}

pub struct Dimension {
    terrain: Terrain<DefaultGenerateBlock>,
    storage: DimensionStorage,
}

unsafe impl Sync for Dimension {}

impl Default for Dimension {
    fn default() -> Self {
        Dimension {
            terrain: Terrain::default(),
            storage: DimensionStorage::new(),
        }
    }
}

impl Dimension {
    pub fn new() -> Self {
        Dimension::default()
    }

    pub fn chunk_exists<M: Into<MortonCode>>(&self, pos: M) -> bool {
        self.storage.get(pos.into()).is_some()
    }

    pub fn chunk_file_exists<P, PATH: AsRef<Path>>(&self, dimension_dir: PATH, pos: P) -> bool
    where
        P: Into<MortonCode>,
    {
        let morton: MortonCode = pos.into();
        self.storage.chunk_exists(dimension_dir, morton)
    }

    pub fn _create_or_load_chunk<'a, P, PATH: AsRef<Path>>(
        &'a mut self,
        dimension_dir: PATH,
        morton: MortonCode,
        point: P,
    ) -> std::io::Result<MutexGuard<'a, Chunk>>
    where
        P: Borrow<Point3<FieldOf<Chunk>>>,
    {
        if self.chunk_file_exists(dimension_dir.as_ref(), morton) {
            self.storage.load(dimension_dir, morton)
        } else {
            let (chunk, _) = self
                .storage
                .insert(morton, self.terrain.generate_chunk(point));
            Ok(chunk)
        }
    }

    pub fn get_chunk<M>(&self, morton: M) -> Option<&Mutex<Chunk>>
    where
        M: Into<MortonCode>,
    {
        self.storage.get(morton)
    }

    pub fn create<P>(&mut self, pos: P)
    where
        P: Borrow<Point3<FieldOf<Chunk>>>,
    {
        let point = pos.borrow();
        let morton: MortonCode = pos.borrow().into();
        let chunk = self.terrain.generate_chunk(point);
        self.storage.insert(morton, chunk);
    }

    pub fn store<P: AsRef<Path>>(&self, dir: P, runtime: &mut Runtime) {
        let dir = std::fs::canonicalize(dir).unwrap();
        self.storage.write_to_dir(runtime, dir);
    }

    pub fn iter(&self) -> impl Iterator<Item = &Mutex<Chunk>> {
        (&self.storage).into_iter()
    }
}

impl IntoIterator for Dimension {
    type Item = <DimensionStorage as IntoIterator>::Item;
    type IntoIter = <DimensionStorage as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.storage.into_iter()
    }
}

impl<'a> IntoIterator for &'a Dimension {
    type Item = <&'a DimensionStorage as IntoIterator>::Item;
    type IntoIter = <&'a DimensionStorage as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        (&self.storage).into_iter()
    }
}

impl<'a> IntoIterator for &'a mut Dimension {
    type Item = <&'a mut DimensionStorage as IntoIterator>::Item;
    type IntoIter = <&'a mut DimensionStorage as IntoIterator>::IntoIter;
    fn into_iter(self) -> Self::IntoIter {
        (&mut self.storage).into_iter()
    }
}
