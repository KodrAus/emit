use emit_core::{ctxt::Ctxt, empty::Empty, id::Id, props::Props, template::Template, time::Clock};
use std::{
    future::Future,
    marker::PhantomData,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

pub struct Span<C: Ctxt, T: Clock = Empty> {
    scope: mem::ManuallyDrop<C::Span>,
    ctxt: C,
    clock: T,
}

impl<C: Ctxt, T: Clock> Span<C, T> {
    pub fn new(ctxt: C, clock: T, id: Id, tpl: Template, props: impl Props) -> Self {
        let scope = mem::ManuallyDrop::new(ctxt.open(clock.now(), id, tpl, props));

        Span { ctxt, scope, clock }
    }

    pub fn enter(&mut self) -> SpanGuard<C, T> {
        self.ctxt.enter(&mut self.scope);

        SpanGuard {
            scope: self,
            _marker: PhantomData,
        }
    }
}

pub struct SpanGuard<'a, C: Ctxt, T: Clock> {
    scope: &'a mut Span<C, T>,
    _marker: PhantomData<*mut fn()>,
}

impl<'a, C: Ctxt, T: Clock> Drop for SpanGuard<'a, C, T> {
    fn drop(&mut self) {
        self.scope.ctxt.exit(&mut self.scope.scope);
    }
}

impl<C: Ctxt, T: Clock> Drop for Span<C, T> {
    fn drop(&mut self) {
        self.ctxt.close(self.clock.now(), unsafe {
            mem::ManuallyDrop::take(&mut self.scope)
        })
    }
}

pub struct SpanFuture<C: Ctxt, F, T: Clock = Empty> {
    scope: Span<C, T>,
    future: F,
}

impl<C: Ctxt, F, T: Clock> SpanFuture<C, F, T> {
    pub fn new(scope: C, clock: T, id: Id, tpl: Template, props: impl Props, future: F) -> Self {
        SpanFuture {
            scope: Span::new(scope, clock, id, tpl, props),
            future,
        }
    }
}

impl<C: Ctxt, F: Future, T: Clock> Future for SpanFuture<C, F, T> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let unpinned = unsafe { Pin::get_unchecked_mut(self) };

        let __guard = unpinned.scope.enter();
        unsafe { Pin::new_unchecked(&mut unpinned.future) }.poll(cx)
    }
}
