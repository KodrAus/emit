use core::{
    future::Future,
    marker::PhantomData,
    mem,
    pin::Pin,
    task::{Context, Poll},
};
use emit_core::{ctxt::Ctxt, props::Props};

pub struct LocalFrame<C: Ctxt> {
    scope: mem::ManuallyDrop<C::LocalFrame>,
    ctxt: C,
}

impl<C: Ctxt> LocalFrame<C> {
    #[track_caller]
    pub fn new(ctxt: C, props: impl Props) -> Self {
        let scope = mem::ManuallyDrop::new(ctxt.open(props));

        LocalFrame { ctxt, scope }
    }

    #[track_caller]
    pub fn enter(&mut self) -> EnterGuard<C> {
        self.ctxt.enter(&mut self.scope);

        EnterGuard {
            scope: self,
            _marker: PhantomData,
        }
    }

    #[track_caller]
    pub fn into_future<F>(self, future: F) -> LocalFrameFuture<C, F> {
        LocalFrameFuture {
            frame: self,
            future,
        }
    }
}

pub struct EnterGuard<'a, C: Ctxt> {
    scope: &'a mut LocalFrame<C>,
    _marker: PhantomData<*mut fn()>,
}

impl<'a, C: Ctxt> Drop for EnterGuard<'a, C> {
    fn drop(&mut self) {
        self.scope.ctxt.exit(&mut self.scope.scope);
    }
}

impl<C: Ctxt> Drop for LocalFrame<C> {
    fn drop(&mut self) {
        self.ctxt
            .close(unsafe { mem::ManuallyDrop::take(&mut self.scope) })
    }
}

pub struct LocalFrameFuture<C: Ctxt, F> {
    frame: LocalFrame<C>,
    future: F,
}

impl<C: Ctxt, F: Future> Future for LocalFrameFuture<C, F> {
    type Output = F::Output;

    #[track_caller]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let unpinned = unsafe { Pin::get_unchecked_mut(self) };

        let __guard = unpinned.frame.enter();
        unsafe { Pin::new_unchecked(&mut unpinned.future) }.poll(cx)
    }
}
