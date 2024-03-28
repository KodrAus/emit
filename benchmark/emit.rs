#![feature(stmt_expr_attributes, proc_macro_hygiene)]

use std::hint::black_box;

use criterion::{criterion_group, criterion_main, Criterion};

pub fn criterion_benchmark(c: &mut Criterion) {
    #[cfg(feature = "full")]
    {
        let _rt = emit::setup().init();
    }

    c.bench_function("emit msg empty", |b| b.iter(|| emit::info!("")));
    c.bench_function("emit msg empty prop 10 int", |b| {
        b.iter(|| {
            emit::info!("",
                f0: 0,
                f1: 1,
                f2: 2,
                f4: 3,
                f5: 4,
                f6: 5,
                f7: 6,
                f8: 7,
                f9: 9,
            );
        })
    });
    c.bench_function("emit msg empty prop 10 debug", |b| {
        b.iter(|| {
            emit::info!("",
                #[emit::as_debug]
                f0: 0,
                #[emit::as_debug]
                f1: 1,
                #[emit::as_debug]
                f2: 2,
                #[emit::as_debug]
                f4: 3,
                #[emit::as_debug]
                f5: 4,
                #[emit::as_debug]
                f6: 5,
                #[emit::as_debug]
                f7: 6,
                #[emit::as_debug]
                f8: 7,
                #[emit::as_debug]
                f9: 9,
            );
        })
    });
    c.bench_function("emit msg 10 int", |b| {
        b.iter(|| {
            emit::info!("{f0: 0}{f1: 1}{f2: 2}{f3: 3}{f4: 4}{f5: 5}{f6: 6}{f7: 7}{f8: 8}{f9: 9}");
        })
    });
    c.bench_function("emit span msg empty", |b| {
        b.iter(|| {
            #[emit::span("")]
            {
                black_box(());
            }
        })
    });
    c.bench_function("emit span msg empty prop 10 int", |b| {
        b.iter(|| {
            #[emit::span("",
            f0: 0,
            f1: 1,
            f2: 2,
            f4: 3,
            f5: 4,
            f6: 5,
            f7: 6,
            f8: 7,
            f9: 9,
        )]
            {
                black_box(());
            }
        })
    });
    c.bench_function("emit span msg empty prop 10 debug", |b| {
        b.iter(|| {
            #[emit::span("",
            #[emit::as_debug]
            f0: 0,
            #[emit::as_debug]
            f1: 1,
            #[emit::as_debug]
            f2: 2,
            #[emit::as_debug]
            f4: 3,
            #[emit::as_debug]
            f5: 4,
            #[emit::as_debug]
            f6: 5,
            #[emit::as_debug]
            f7: 6,
            #[emit::as_debug]
            f8: 7,
            #[emit::as_debug]
            f9: 9,
        )]
            {
                black_box(());
            }
        })
    });
    c.bench_function("emit span msg 10 int", |b| {
        b.iter(|| {
            #[emit::span("{f0: 0}{f1: 1}{f2: 2}{f3: 3}{f4: 4}{f5: 5}{f6: 6}{f7: 7}{f8: 8}{f9: 9}")]
            {
                black_box(());
            }
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
