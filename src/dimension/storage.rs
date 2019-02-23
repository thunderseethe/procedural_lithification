use crate::{chunk::Chunk, dimension::morton_code::MortonCode};
use bincode::{deserialize_from, serialize_into};
use flate2::{read::DeflateDecoder, write::DeflateEncoder, Compression};
use parking_lot::{Mutex, RwLock};
use rayon::iter::{
    IndexedParallelIterator, IntoParallelIterator, IntoParallelRefIterator,
    IntoParallelRefMutIterator, ParallelIterator,
};
use std::{borrow::Borrow, mem, path::Path, vec::Vec};
use tokio::{
    fs::{File, OpenOptions},
    prelude::*,
    runtime::Runtime,
};

const CHUNK_DIR: &'static str = "chunk";

pub struct DimensionStorage {
    len: usize,
    indices: RwLock<Vec<MortonCode>>,
    data: Vec<Mutex<Chunk>>,
}

impl DimensionStorage {
    pub fn with_capacity(capacity: usize) -> Self {
        DimensionStorage {
            len: capacity,
            indices: RwLock::new(Vec::with_capacity(capacity)),
            data: Vec::with_capacity(capacity),
        }
    }

    pub fn len(&self) -> usize {
        self.data.len()
    }

    /// Default to a capacity of 9
    pub fn new() -> Self {
        DimensionStorage::with_capacity(9)
    }

    /// Insert a new chunk at pos, if index is empty None is returned. Otherwise the previous chunk is returned.
    pub fn insert<M>(&mut self, pos: M, chunk: Chunk) -> Option<Chunk>
    where
        M: Into<MortonCode>,
    {
        let morton: MortonCode = pos.into();
        let mut indices = self.indices.write();
        match indices.binary_search(&morton) {
            Err(indx) => {
                indices.insert(indx, morton);
                self.data.insert(indx, Mutex::new(chunk));
                return None;
            }
            Ok(indx) => {
                let old_mutex = mem::replace(&mut self.data[indx], Mutex::new(chunk));
                return Some(old_mutex.into_inner());
            }
        }
    }

    pub fn chunk_exists<P, M>(&self, dir: P, pos: M) -> bool
    where
        P: AsRef<Path>,
        M: Into<MortonCode>,
    {
        dir.as_ref()
            .join(CHUNK_DIR)
            .join(format!("{}", pos.into()))
            .exists()
    }

    pub fn load<P, M>(&mut self, dir: P, pos: M) -> Result<(), bincode::Error>
    where
        P: AsRef<Path>,
        M: Into<MortonCode>,
    {
        let morton = pos.into();
        let chunk_path = dir.as_ref().join(CHUNK_DIR).join(format!("{}", morton));
        File::open(chunk_path)
            .then(|file_res| match file_res {
                Err(e) => Err(Box::new(bincode::ErrorKind::Io(e))),
                Ok(file) => {
                    let decoder = DeflateDecoder::new(file);
                    deserialize_from(decoder)
                }
            })
            .map(|chunk| {
                // We're overwriting whatever was previously present at this index.
                let indx = match self.indices.read().binary_search(&morton) {
                    Ok(indx) => indx,
                    Err(indx) => indx,
                };
                let mut index_lock = self.indices.write();
                self.data.insert(indx, Mutex::new(chunk));
                index_lock.insert(indx, morton);
            })
            .wait()
    }

    pub fn get<'a, M>(&'a self, pos: M) -> Option<&'a Mutex<Chunk>>
    where
        M: Into<MortonCode>,
    {
        let morton: MortonCode = pos.into();
        let indices = self.indices.read();
        match indices.binary_search(&morton) {
            Err(_) => None,
            Ok(indx) => self.data.get(indx),
        }
    }

    pub fn write_to_dir<P>(&self, runtime: &mut Runtime, path_ref: P)
    where
        P: Borrow<Path>,
    {
        let path = path_ref.borrow().join(CHUNK_DIR);
        std::fs::create_dir_all(&path).expect("Unable to created dimension chunk directory");
        let indices = self.indices.read();
        indices
            .iter()
            .zip(self.data.iter())
            .for_each(|(morton, mutex_chunk)| {
                let chunk_file = path.join(format!("{}", morton));
                let chunk = mutex_chunk.lock().clone();
                let file_fut = OpenOptions::new().write(true).create(true).open(chunk_file);
                runtime.spawn(future::lazy(move || {
                    file_fut
                        .then(move |file_res| match file_res {
                            Err(err) => Err(Box::new(bincode::ErrorKind::Io(err))),
                            Ok(file) => {
                                let encoder = DeflateEncoder::new(file, Compression::best());
                                serialize_into(encoder, &chunk)
                            }
                        })
                        .map_err(|err| {
                            println!("{:?}", err);
                            ()
                        })
                }));
            });
    }
}

impl IntoIterator for DimensionStorage {
    type Item = Chunk;
    type IntoIter = std::iter::Map<std::vec::IntoIter<Mutex<Chunk>>, fn(Mutex<Chunk>) -> Chunk>;

    fn into_iter(self) -> Self::IntoIter {
        self.data
            .into_iter()
            .map((|mutex| mutex.into_inner()) as fn(Mutex<Chunk>) -> Chunk)
    }
}

impl<'a> IntoIterator for &'a DimensionStorage {
    type Item = &'a Mutex<Chunk>;
    type IntoIter = std::slice::Iter<'a, Mutex<Chunk>>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter()
    }
}

impl<'a> IntoIterator for &'a mut DimensionStorage {
    type Item = &'a mut Mutex<Chunk>;
    type IntoIter = std::slice::IterMut<'a, Mutex<Chunk>>;

    fn into_iter(self) -> Self::IntoIter {
        self.data.iter_mut()
    }
}

impl IntoParallelIterator for DimensionStorage {
    type Item = Chunk;
    type Iter = rayon::iter::Map<rayon::vec::IntoIter<Mutex<Chunk>>, fn(Mutex<Chunk>) -> Chunk>;

    fn into_par_iter(self) -> Self::Iter {
        self.data
            .into_par_iter()
            .map((|mutex| mutex.into_inner()) as fn(Mutex<Chunk>) -> Chunk)
    }
}

impl<'a> IntoParallelIterator for &'a DimensionStorage {
    type Item = &'a Mutex<Chunk>;
    type Iter = rayon::slice::Iter<'a, Mutex<Chunk>>;

    fn into_par_iter(self) -> Self::Iter {
        self.data.par_iter()
    }
}

impl<'a> IntoParallelIterator for &'a mut DimensionStorage {
    type Item = &'a mut Mutex<Chunk>;
    type Iter = rayon::slice::IterMut<'a, Mutex<Chunk>>;

    fn into_par_iter(self) -> Self::Iter {
        self.data.par_iter_mut()
    }
}
