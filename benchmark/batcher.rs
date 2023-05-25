#![feature(proc_macro_hygiene, stmt_expr_attributes)]

use std::thread;

use criterion::{black_box, criterion_group, criterion_main, Criterion};

pub fn criterion_benchmark(c: &mut Criterion) {
    let (sender, receiver) = emit_batcher::bounded::<u64>(1024);

    thread::spawn(move || {
        receiver
            .blocking_exec(|batch| {
                black_box(batch);

                Ok(())
            })
            .unwrap();
    });

    c.bench_function("batch u64 thread 1 msg 10_000", |b| {
        b.iter(|| {
            for i in 0..10_000 {
                sender.send(i);
            }
        })
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
