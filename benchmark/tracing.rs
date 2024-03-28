use criterion::{criterion_group, criterion_main, Criterion};

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("tracing msg empty", |b| b.iter(|| tracing::info!("")));
    c.bench_function("tracing msg empty prop 10 int", |b| {
        b.iter(|| {
            tracing::info!(
                f0 = 0,
                f1 = 1,
                f2 = 2,
                f4 = 3,
                f5 = 4,
                f6 = 5,
                f7 = 6,
                f8 = 7,
                f9 = 9,
                msg = ""
            );
        })
    });
    c.bench_function("tracing msg empty prop 10 debug", |b| {
        b.iter(|| {
            tracing::info!(
                f0 = ?0,
                f1 = ?1,
                f2 = ?2,
                f4 = ?3,
                f5 = ?4,
                f6 = ?5,
                f7 = ?6,
                f8 = ?7,
                f9 = ?9,
                msg = ""
            );
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
