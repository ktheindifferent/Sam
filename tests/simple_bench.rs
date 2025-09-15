use criterion::{black_box, criterion_group, criterion_main, Criterion};
use libsam::services::crawler;

fn simple_test(c: &mut Criterion) {
    c.bench_function("simple", |b| {
        b.iter(|| {
            black_box(crawler::url_patterns::normalize_url("https://example.com"))
        })
    });
}

criterion_group!(benches, simple_test);
criterion_main!(benches);
