use criterion::{criterion_group, criterion_main, Criterion};

pub fn criterion_benchmark(c: &mut Criterion) {
    c.bench_function("log msg empty", |b| b.iter(|| log::info!("")));
    c.bench_function("log msg empty prop 10 int", |b| {
        b.iter(|| {
            log::info!(
                f0 = 0,
                f1 = 1,
                f2 = 2,
                f4 = 3,
                f5 = 4,
                f6 = 5,
                f7 = 6,
                f8 = 7,
                f9 = 9;
                ""
            );
        })
    });
    c.bench_function("log msg empty prop 10 debug", |b| {
        b.iter(|| {
            log::info!(
                f0:? = 0,
                f1:? = 1,
                f2:? = 2,
                f4:? = 3,
                f5:? = 4,
                f6:? = 5,
                f7:? = 6,
                f8:? = 7,
                f9:? = 9;
                ""
            );
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
