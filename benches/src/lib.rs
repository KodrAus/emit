#![feature(test, proc_macro_hygiene, stmt_expr_attributes)]

extern crate test;

#[bench]
fn emit_empty(b: &mut test::Bencher) {
    b.iter(|| emit::info!(""))
}

#[bench]
fn emit_10_int(b: &mut test::Bencher) {
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
}

#[bench]
fn emit_10_int_interpolated(b: &mut test::Bencher) {
    b.iter(|| {
        emit::info!("{f0: 0}{f1: 1}{f2: 2}{f3: 3}{f4: 4}{f5: 5}{f6: 6}{f7: 7}{f8: 8}{f9: 9}");
    })
}

#[bench]
fn emit_10_as_debug(b: &mut test::Bencher) {
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
}

#[bench]
fn emit_10_int_get_cast(b: &mut test::Bencher) {
    b.iter(|| {
        emit::info!(
            target: |r| {
                test::black_box(r.get("f5").unwrap().to_i64().unwrap());
            },
            "",
            f0: 0,
            f1: 1,
            f2: 2,
            f4: 3,
            f5: 4,
            f6: 5,
            f7: 6,
            f8: 7,
            f9: 9,
        )
    })
}

#[bench]
fn emit_10_int_get_missing(b: &mut test::Bencher) {
    b.iter(|| {
        emit::info!(
            target: |r| {
                test::black_box(r.get("f10"));
            },
            "",
            f0: 0,
            f1: 1,
            f2: 2,
            f4: 3,
            f5: 4,
            f6: 5,
            f7: 6,
            f8: 7,
            f9: 9,
        )
    })
}