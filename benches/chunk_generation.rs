#[macro_use]
extern crate criterion;
extern crate cubes_lib;

use criterion::{Criterion, Fun};
use cubes_lib::terrain::Terrain;
use std::time::Duration;

fn chunk_generation(c: &mut Criterion) {
    let safe_gen = Fun::new("ChunkBuilder", |b, _| {
        b.iter(|| Terrain::new(0.0).generate_chunk())
    });
    let unsafe_gen = Fun::new("Bare Rayon", |b, _| {
        b.iter(|| Terrain::new(0.0).old_generate_chunk())
    });
    c.bench_functions("Chunk Generation", vec![safe_gen, unsafe_gen], &5);
}

criterion_group! {
    name = benches;
    config = Criterion::default().sample_size(10).warm_up_time(Duration::new(10, 0));
    targets = chunk_generation
}
criterion_main!(benches);
