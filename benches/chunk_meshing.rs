#[macro_use]
extern crate criterion;
extern crate cubes_lib;

use amethyst::core::nalgebra::Point3;
use criterion::{Criterion, Fun, ParameterizedBenchmark};
use cubes_lib::terrain::Terrain;
use std::time::Duration;

fn chunk_meshing(c: &mut Criterion) {
    c.bench_function("chunk_meshing", |b| {
        let chunk = Terrain::new().generate_chunk(Point3::origin());
        b.iter(|| chunk.generate_mesh())
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10).warm_up_time(Duration::new(10, 0));
    targets = chunk_meshing
}

criterion_main!(benches);
