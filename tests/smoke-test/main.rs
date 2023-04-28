#![feature(stmt_expr_attributes, proc_macro_hygiene)]

#[tokio::main]
async fn main() {
    println!(
        "{}",
        emit::tpl!(
            "something went wrong at {id} with {x}!",
            #[emit::fmt(flags: "04")]
            x,
        )
        .render()
        .with_props(emit::props! {
            #[emit::as_debug]
            id: 42,
            x: 15,
        })
    );

    emit::to(emit::target::from_fn(|evt| {
        println!("{:?}", evt);
    }));

    emit::with_linker(ctxt::ThreadLocalCtxt);

    in_ctxt(78).await;
}

#[emit::with("Hello!", a, ax: 13)]
async fn in_ctxt(a: i32) {
    in_ctxt2(5).await;

    emit::info!("an event!");
}

#[emit::with("Hello!", b, bx: 90)]
async fn in_ctxt2(b: i32) {
    // Emit an info event to the global receiver
    emit::info!(
        with: emit::props! {
            request_id: "abc",
        },
        "something went wrong at {#[emit::as_debug] id: 42} with {x}!",
        #[emit::fmt(flags: "04")]
        x: 15,
    );
}

mod ctxt {
    use std::{
        cell::RefCell,
        ops::ControlFlow::{self, *},
    };

    thread_local! {
        static ACTIVE: RefCell<ThreadLocalProps> = RefCell::new(ThreadLocalProps(Vec::new()));
    }

    pub struct ThreadLocalCtxt;

    pub struct ThreadLocalProps(Vec<(String, String)>);

    impl emit::Props for ThreadLocalProps {
        fn for_each<'a, F: FnMut(emit::Key<'a>, emit::Value<'a>) -> ControlFlow<()>>(
            &'a self,
            mut for_each: F,
        ) {
            for (k, v) in &self.0 {
                if let Break(()) = for_each(emit::Key::from(&**k), emit::Value::from(&**v)) {
                    break;
                }
            }
        }
    }

    impl emit::GetCtxt for ThreadLocalCtxt {
        type Props = ThreadLocalProps;

        fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
            ACTIVE.with(|props| with(&*props.borrow()))
        }
    }

    impl emit::LinkCtxt for ThreadLocalCtxt {
        type Link = ThreadLocalProps;

        fn prepare<P: emit::Props>(&self, props: P) -> Self::Link {
            let mut owned = ACTIVE.with(|props| props.borrow().0.clone());

            props.for_each(|k, v| {
                owned.push((k.to_string(), v.to_string()));
                Continue(())
            });

            ThreadLocalProps(owned)
        }

        fn link(&self, link: &mut Self::Link) {
            ACTIVE.with(|props| std::mem::swap(&mut link.0, &mut props.borrow_mut().0));
        }

        fn unlink(&self, link: &mut Self::Link) {
            ACTIVE.with(|props| std::mem::swap(&mut link.0, &mut props.borrow_mut().0));
        }
    }
}
