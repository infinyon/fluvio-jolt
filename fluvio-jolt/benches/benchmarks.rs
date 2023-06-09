use criterion::{black_box, criterion_group, criterion_main, Criterion};
use serde_json::Value;
use fluvio_jolt::{transform, TransformSpec};

pub fn transform_benchmark(c: &mut Criterion) {
    let spec: TransformSpec =
        serde_json::from_str(include_str!("spec.json")).expect("parsed transform spec");
    let input: Value = serde_json::from_str(include_str!("input.json")).expect("parsed spec");
    c.bench_function("default op", |b| {
        b.iter_with_large_setup(
            || input.clone(),
            |input| transform(black_box(input), black_box(&spec)),
        )
    });
}

criterion_group!(benches, transform_benchmark);
criterion_main!(benches);
