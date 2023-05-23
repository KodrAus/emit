use std::{
    cell::RefCell,
    ops::ControlFlow::{self, *},
};

use crate::{ctxt::Ctxt, key::OwnedKey, value::OwnedValue, Id, Key, Props, Value};

thread_local! {
    static ACTIVE: RefCell<ThreadLocalSpan> = RefCell::new(ThreadLocalSpan {
        id: Id::EMPTY,
        props: Vec::new(),
    });
}

#[derive(Default, Debug, Clone, Copy)]
pub struct ThreadLocalCtxt;

#[derive(Clone)]
pub struct ThreadLocalSpan {
    id: Id,
    props: Vec<(OwnedKey, OwnedValue)>,
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
    type Props = ThreadLocalSpan;
    type Span = ThreadLocalSpan;

    fn with_current<F: FnOnce(Id, &Self::Props)>(&self, with: F) {
        ACTIVE.with(|span| {
            let span = &*span.borrow();

            with(span.id, &span)
        })
    }

    fn current_id(&self) -> Id {
        ACTIVE.with(|span| span.borrow().id)
    }

    fn open<P: Props>(&self, id: Id, props: P) -> Self::Span {
        let mut span = ACTIVE.with(|span| span.borrow().clone());

        span.id = id;
        props.for_each(|k, v| {
            span.props.push((k.to_owned(), v.to_owned()));
            Continue(())
        });

        span
    }

    fn enter(&self, link: &mut Self::Span) {
        ACTIVE.with(|span| std::mem::swap(link, &mut *span.borrow_mut()));
    }

    fn exit(&self, link: &mut Self::Span) {
        ACTIVE.with(|span| std::mem::swap(link, &mut *span.borrow_mut()));
    }

    fn close(&self, _: Self::Span) {}
}
