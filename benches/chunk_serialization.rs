#[macro_use]
extern crate criterion;
extern crate cubes_lib;

use amethyst::core::nalgebra::Point3;
use criterion::Criterion;
use cubes_lib::chunk::file_format;
use cubes_lib::terrain::{Terrain, DefaultGenerateBlock};
use std::time::Duration;

fn serialization_bench(c: &mut Criterion){
    c.bench_function("Chunk Serialization", |b|{
        let chunk = Terrain::default().generate_chunk(Point3::origin());
        b.iter(||file_format::chunk_to_bytes(&chunk))
    });

    c.bench_function("Chunk Deserialization", |b|{
        let center = Point3::origin();
        let chunk = Terrain::default().generate_chunk(center);
        let bytes = file_format::chunk_to_bytes(&chunk);
        b.iter(||file_format::bytes_to_chunk(&bytes, center))
    });
}

criterion_group!{
    name = serialization_benchmarks;
    config = Criterion::default()
                       .sample_size(10)
                       .warm_up_time(Duration::new(10, 0));
    targets = serialization_bench
}

criterion_main!(serialization_benchmarks);