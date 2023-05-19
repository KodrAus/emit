use std::{
    cell::RefCell,
    ops::ControlFlow::{self, *},
};

use crate::{ctxt::Ctxt, Key, OwnedKey, OwnedValue, Props, Value};

thread_local! {
    static ACTIVE: RefCell<ThreadLocalProps> = RefCell::new(ThreadLocalProps(Vec::new()));
}

pub struct ThreadLocalCtxt;

pub struct ThreadLocalProps(Vec<(OwnedKey, OwnedValue)>);

impl Props for ThreadLocalProps {
    fn for_each<'a, F: FnMut(Key<'a>, Value<'a>) -> ControlFlow<()>>(&'a self, mut for_each: F) {
        for (k, v) in &self.0 {
            if let Break(()) = for_each(Key::from(&*k), Value::from(&*v)) {
                break;
            }
        }
    }
}

impl Ctxt for ThreadLocalCtxt {
    type Props = ThreadLocalProps;
    type Span = ThreadLocalProps;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        ACTIVE.with(|props| with(&*props.borrow()))
    }

    fn open<P: Props>(&self, props: P) -> Self::Span {
        let mut owned = ACTIVE.with(|props| props.borrow().0.clone());

        props.for_each(|k, v| {
            owned.push((k.to_owned(), v.to_owned()));
            Continue(())
        });

        ThreadLocalProps(owned)
    }

    fn enter(&self, link: &mut Self::Span) {
        ACTIVE.with(|props| std::mem::swap(&mut link.0, &mut props.borrow_mut().0));
    }

    fn exit(&self, link: &mut Self::Span) {
        ACTIVE.with(|props| std::mem::swap(&mut link.0, &mut props.borrow_mut().0));
    }

    fn close(&self, _: Self::Span) {}
}
