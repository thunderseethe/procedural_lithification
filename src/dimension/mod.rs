use crate::{chunk::Chunk, terrain::Terrain};
use amethyst::{core::nalgebra::Point3, renderer::MeshData};
use rayon::prelude::*;
use std::{borrow::Borrow, path::PathBuf};
use tokio::{prelude::*, runtime::Runtime};

mod morton_code;
mod storage;

use morton_code::MortonCode;
use storage::DimensionStorage;

pub struct Dimension {
    directory: PathBuf,
    terrain: Terrain,
    storage: DimensionStorage,
}

unsafe impl Sync for Dimension {}

impl Default for Dimension {
    fn default() -> Self {
        Dimension {
            directory: PathBuf::from("./resources/dimension/"),
            terrain: Terrain::new(),
            storage: DimensionStorage::new(),
        }
    }
}

impl Dimension {
    pub fn new(directory: PathBuf) -> Self {
        Dimension {
            directory,
            terrain: Terrain::new(),
            storage: DimensionStorage::new(),
        }
    }

    pub fn create_or_load_chunk<P>(&mut self, pos: P) -> Result<(), bincode::Error>
    where
        P: Borrow<Point3<i32>>,
    {
        let point = pos.borrow();
        let morton: MortonCode = pos.borrow().into();
        if self.storage.chunk_exists(self.directory.as_path(), morton) {
            self.storage.load(self.directory.as_path(), morton)
        } else {
            self.storage
                .insert(morton, self.terrain.generate_chunk(point));
            Ok(())
        }
    }

    pub fn store(&self, runtime: &mut Runtime) {
        let dir = std::fs::canonicalize(&self.directory).unwrap();
        self.storage.write_to_dir(runtime, dir);
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
