#[macro_use]
extern crate criterion;
extern crate cubes_lib;

use amethyst::core::nalgebra::Point3;
use criterion::{Criterion};
use cubes_lib::terrain::Terrain;
use std::time::Duration;

fn chunk_generation(c: &mut Criterion) {
    c.bench_function("Chunk Generation", |b| {
        b.iter(|| Terrain::new().generate_chunk(Point3::new(0, 0, 0)));
    });
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10).warm_up_time(Duration::new(10, 0));
    targets = chunk_generation
}
criterion_main!(benches);
