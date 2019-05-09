#[macro_use]
extern crate criterion;
extern crate cubes_lib;

use amethyst::core::nalgebra::Point3;
use criterion::Criterion;
use cubes_lib::chunk::file_format::{ChunkDeserialize, ChunkSerialize};
use cubes_lib::terrain::Terrain;
use std::time::Duration;

fn serialization_bench(c: &mut Criterion) {
    c.bench_function("Chunk Serialization", |b| {
        let chunk = Terrain::default().generate_chunk(Point3::origin());
        b.iter(|| ChunkSerialize::into(&mut std::io::sink(), &chunk))
    });

    c.bench_function("Chunk Deserialization", |b| {
        let center = Point3::origin();
        let chunk = Terrain::default().generate_chunk(center);
        let mut bytes: Vec<u8> = Vec::new();
        ChunkSerialize::into(&mut bytes, &chunk).expect("Failed to serialize chunk into bytes");
        b.iter(|| ChunkDeserialize::from(&bytes[..], center))
    });
}

criterion_group! {
    name = serialization_benchmarks;
    config = Criterion::default()
                       .sample_size(10)
                       .warm_up_time(Duration::new(10, 0));
    targets = serialization_bench
}

criterion_main!(serialization_benchmarks);
