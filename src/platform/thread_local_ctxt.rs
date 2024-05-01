use core::mem;
use std::{cell::RefCell, collections::HashMap, ops::ControlFlow, sync::Mutex};

use alloc::sync::Arc;
use emit_core::{
    ctxt::Ctxt,
    props::Props,
    runtime::InternalCtxt,
    str::Str,
    value::{OwnedValue, Value},
};

static NEXT_CTXT_ID: Mutex<usize> = Mutex::new(0);

fn ctxt_id() -> usize {
    let mut next_id = NEXT_CTXT_ID.lock().unwrap();
    let id = *next_id;
    *next_id = id.wrapping_add(1);

    id
}

thread_local! {
    static ACTIVE: RefCell<HashMap<usize, ThreadLocalSpan>> = RefCell::new(HashMap::new());
}

fn current(id: usize) -> ThreadLocalSpan {
    ACTIVE.with(|active| {
        active
            .borrow_mut()
            .entry(id)
            .or_insert_with(|| ThreadLocalSpan { props: None })
            .clone()
    })
}

fn swap(id: usize, incoming: &mut ThreadLocalSpan) {
    ACTIVE.with(|active| {
        let mut active = active.borrow_mut();

        let current = active
            .entry(id)
            .or_insert_with(|| ThreadLocalSpan { props: None });

        mem::swap(current, incoming);
    })
}

#[derive(Debug, Clone, Copy)]
pub struct ThreadLocalCtxt {
    id: usize,
}

impl Default for ThreadLocalCtxt {
    fn default() -> Self {
        ThreadLocalCtxt { id: ctxt_id() }
    }
}

#[derive(Clone)]
pub struct ThreadLocalSpan {
    props: Option<Arc<HashMap<Str<'static>, OwnedValue>>>,
}

impl Props for ThreadLocalSpan {
    fn for_each<'a, F: FnMut(Str<'a>, Value<'a>) -> ControlFlow<()>>(
        &'a self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        if let Some(ref props) = self.props {
            for (k, v) in &**props {
                for_each(k.by_ref(), v.by_ref())?;
            }
        }

        ControlFlow::Continue(())
    }
}

impl Ctxt for ThreadLocalCtxt {
    type Current = ThreadLocalSpan;
    type Frame = ThreadLocalSpan;

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        let current = current(self.id);
        with(&current)
    }

    fn open_root<P: Props>(&self, props: P) -> Self::Frame {
        let mut span = HashMap::new();

        props.for_each(|k, v| {
            span.insert(k.to_shared(), v.to_shared());
            ControlFlow::Continue(())
        });

        ThreadLocalSpan {
            props: Some(Arc::new(span)),
        }
    }

    fn open_push<P: Props>(&self, props: P) -> Self::Frame {
        let mut span = current(self.id);

        if span.props.is_none() {
            span.props = Some(Arc::new(HashMap::new()));
        }

        let span_props = Arc::make_mut(span.props.as_mut().unwrap());

        props.for_each(|k, v| {
            span_props.insert(k.to_shared(), v.to_shared());
            ControlFlow::Continue(())
        });

        span
    }

    fn enter(&self, link: &mut Self::Frame) {
        swap(self.id, link);
    }

    fn exit(&self, link: &mut Self::Frame) {
        swap(self.id, link);
    }

    fn close(&self, _: Self::Frame) {}
}

impl InternalCtxt for ThreadLocalCtxt {}
