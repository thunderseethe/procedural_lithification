#[macro_use]
extern crate criterion;
extern crate cubes_lib;

use amethyst::core::nalgebra::Point3;
use criterion::{Criterion, ParameterizedBenchmark};
use cubes_lib::octree::new_octree::*;
use cubes_lib::octree::Octree;
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
    let octrees: (Octree8<u32, u8>, Octree<u32>) = (
        points.iter().fold(
            Octree8::new(LevelData::Empty, Point3::origin()),
            |acc, (p, i)| acc.insert(p, *i),
        ),
        points
            .iter()
            .fold(Octree::new(Point3::origin(), None, 8), |acc, (p, i)| {
                acc.insert(p, *i)
            }),
    );
    c.bench(
        "octree_insert",
        ParameterizedBenchmark::new(
            "bounded_recursion",
            |b, (octree, _)| {
                b.iter(|| {
                    octree.insert(
                        Point3::<u8>::new(random(), random(), random()),
                        random::<u32>(),
                    )
                })
            },
            vec![octrees.clone()],
        )
        .with_function("general_recursion", |b, (_, octree)| {
            b.iter(|| {
                octree.insert(
                    Point3::<u8>::new(random(), random(), random()),
                    random::<u32>(),
                )
            })
        }),
    );
    c.bench(
        "octree_delete",
        ParameterizedBenchmark::new(
            "bounded_recursion",
            |b, (octree, _)| {
                b.iter(|| {
                    octree.delete(Point3::new(random(), random(), random()));
                })
            },
            vec![octrees.clone()],
        )
        .with_function("general_recursion", |b, (_, octree)| {
            b.iter(|| {
                octree.delete(Point3::new(random(), random(), random()));
            })
        }),
    );
    c.bench(
        "octree_get",
        ParameterizedBenchmark::new(
            "bounded_recursion",
            |b, (octree, _)| b.iter(|| octree.get(Point3::new(random(), random(), random()))),
            vec![octrees],
        )
        .with_function("general_recursion", |b, (_, octree)| {
            b.iter(|| octree.get(Point3::new(random(), random(), random())))
        }),
    );
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = octree_comparison
}

criterion_main!(benches);
