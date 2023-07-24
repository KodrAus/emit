use std::{
    cell::RefCell,
    collections::HashMap,
    ops::ControlFlow::{self, *},
};

use emit_core::{
    ctxt::Ctxt,
    key::{Key, OwnedKey},
    props::Props,
    value::{OwnedValue, Value},
};

// TODO: Optimize this
thread_local! {
    static ACTIVE: RefCell<ThreadLocalSpan> = RefCell::new(ThreadLocalSpan {
        props: HashMap::new(),
    });
}

#[derive(Default, Debug, Clone, Copy)]
pub struct ThreadLocalCtxt;

#[derive(Clone)]
pub struct ThreadLocalSpan {
    props: HashMap<OwnedKey, OwnedValue>,
}

impl Props for ThreadLocalSpan {
    fn for_each<'a, F: FnMut(Key<'a>, Value<'a>) -> ControlFlow<()>>(&'a self, mut for_each: F) {
        for (k, v) in &self.props {
            if let Break(()) = for_each(Key::from(&*k), Value::from(&*v)) {
                break;
            }
        }
    }
}

impl Ctxt for ThreadLocalCtxt {
    type CurrentProps = ThreadLocalSpan;
    type LocalFrame = ThreadLocalSpan;

    fn with_current<F: FnOnce(&Self::CurrentProps)>(&self, with: F) {
        ACTIVE.with(|span| with(&*span.borrow()))
    }

    fn open<P: Props>(&self, props: P) -> Self::LocalFrame {
        let mut span = ACTIVE.with(|span| span.borrow().clone());

        props.for_each(|k, v| {
            span.props.insert(k.to_owned(), v.to_owned());
            Continue(())
        });

        span
    }

    fn enter(&self, link: &mut Self::LocalFrame) {
        ACTIVE.with(|span| std::mem::swap(link, &mut *span.borrow_mut()));
    }

    fn exit(&self, link: &mut Self::LocalFrame) {
        ACTIVE.with(|span| std::mem::swap(link, &mut *span.borrow_mut()));
    }

    fn close(&self, _: Self::LocalFrame) {}
}
