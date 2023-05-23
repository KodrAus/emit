use std::{
    cell::RefCell,
    ops::ControlFlow::{self, *},
};

use crate::{ctxt::Ctxt, Key, OwnedKey, OwnedValue, Props, Value};

use super::{Id, SpanId, TraceId};

thread_local! {
    static ACTIVE: RefCell<ThreadLocalSpan> = RefCell::new(ThreadLocalSpan {
        id: Id::EMPTY,
        props: Vec::new(),
    });
}

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

    fn open<P: Props>(&self, id: Id, props: P) -> Self::Span {
        let mut owned = ACTIVE.with(|span| span.borrow().clone());

        owned.id = Id::new(
            id.trace().or(owned.id.trace()).or_else(|| gen_trace()),
            id.span().or_else(|| gen_span()),
        );

        props.for_each(|k, v| {
            owned.props.push((k.to_owned(), v.to_owned()));
            Continue(())
        });

        owned
    }

    fn enter(&self, link: &mut Self::Span) {
        ACTIVE.with(|span| std::mem::swap(link, &mut *span.borrow_mut()));
    }

    fn exit(&self, link: &mut Self::Span) {
        ACTIVE.with(|span| std::mem::swap(link, &mut *span.borrow_mut()));
    }

    fn close(&self, _: Self::Span) {}
}

fn gen_trace() -> Option<TraceId> {
    None
}

fn gen_span() -> Option<SpanId> {
    None
}
