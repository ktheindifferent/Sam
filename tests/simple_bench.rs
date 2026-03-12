use criterion::{black_box, criterion_group, criterion_main, Criterion};

fn simple_test(c: &mut Criterion) {
    c.bench_function("simple", |b| {
        b.iter(|| {
            // normalize_url doesn't exist, just do a simple operation
            black_box("https://example.com".to_string())
        })
    });
}

criterion_group!(benches, simple_test);
criterion_main!(benches);
