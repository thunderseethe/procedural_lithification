#[macro_use]
extern crate criterion;
extern crate cubes_lib;

use amethyst::core::nalgebra::Point3;
use criterion::{Criterion, ParameterizedBenchmark};
use cubes_lib::octree::*;
use rand::random;

fn octree_comparison(c: &mut Criterion) {
    let points: Vec<(Point3<u8>, u32)> = (0..8000000)
        .map(|_| {
            (
                Point3::<u8>::new(random(), random(), random()),
                random::<u32>(),
            )
        })
        .collect();
    let octrees: Octree8<u32, u8> = points.iter().fold(
        Octree8::new(LevelData::Empty, Point3::origin()),
        |acc, (p, i)| acc.insert(p, *i),
    );
    c.bench(
        "octree_insert",
        ParameterizedBenchmark::new(
            "bounded_recursion",
            |b, octree| {
                b.iter(|| {
                    octree.insert(
                        Point3::<u8>::new(random(), random(), random()),
                        random::<u32>(),
                    )
                })
            },
            vec![octrees.clone()],
        ),
    );
    c.bench(
        "octree_delete",
        ParameterizedBenchmark::new(
            "bounded_recursion",
            |b, octree| {
                b.iter(|| {
                    octree.delete(Point3::new(random(), random(), random()));
                })
            },
            vec![octrees.clone()],
        ),
    );
    c.bench(
        "octree_get",
        ParameterizedBenchmark::new(
            "bounded_recursion",
            |b, octree| b.iter(|| octree.get(Point3::new(random(), random(), random()))),
            vec![octrees],
        ),
    );
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = octree_comparison
}

criterion_main!(benches);
