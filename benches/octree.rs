#[macro_use]
extern crate criterion;
extern crate cubes_lib;

use amethyst::core::nalgebra::Point3;
use criterion::{Criterion, ParameterizedBenchmark};
use cubes_lib::octree::type_int_octree::*;
use cubes_lib::octree::Octree;
use rand::random;

fn insert_comparison(c: &mut Criterion) {
    let mut test_data = Vec::new();
    for _ in 0..1000 {
        test_data.push((
            Point3::<u8>::new(random(), random(), random()),
            random::<u32>(),
        ));
    }
    c.bench(
        "octree_insertion",
        ParameterizedBenchmark::new(
            "bounded_recursion",
            |b, points| {
                b.iter(|| {
                    let mut octree: OctreeLevel<
                        u32,
                        u8,
                        OctreeLevel<u32, u8, OctreeBase<u32, u8>>,
                    > = OctreeLevel::new(LevelData::Empty, Point3::origin());
                    for (p, i) in points {
                        octree = octree.insert(p, *i);
                    }
                })
            },
            vec![test_data],
        )
        .with_function("general_recursion", |b, points| {
            b.iter(|| {
                let mut octree: Octree<u32> = Octree::new(Point3::origin(), None, 8);
                for (p, i) in points {
                    octree = octree.insert(p, *i);
                }
            })
        }),
    );
}

criterion_group! {
    name = benches;
    config = Criterion::default();
    targets = insert_comparison
}

criterion_main!(benches);
