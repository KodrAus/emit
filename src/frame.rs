use core::{
    future::Future,
    marker::PhantomData,
    mem,
    pin::Pin,
    task::{Context, Poll},
};
use emit_core::{ctxt::Ctxt, props::Props};

pub struct Frame<C: Ctxt> {
    scope: mem::ManuallyDrop<C::Frame>,
    ctxt: C,
}

impl<C: Ctxt> Frame<C> {
    #[track_caller]
    pub fn current(ctxt: C) -> Self {
        Self::push(ctxt, crate::empty::Empty)
    }

    #[track_caller]
    #[must_use = "call `enter`, `call`, or `in_future` to make the pushed properties active"]
    pub fn push(ctxt: C, props: impl Props) -> Self {
        let scope = mem::ManuallyDrop::new(ctxt.open_push(props));

        Frame { ctxt, scope }
    }

    #[track_caller]
    #[must_use = "call `enter`, `call`, or `in_future` to make the properties active"]
    pub fn root(ctxt: C, props: impl Props) -> Self {
        let scope = mem::ManuallyDrop::new(ctxt.open_root(props));

        Frame { ctxt, scope }
    }

    #[track_caller]
    pub fn with<R>(&mut self, with: impl FnOnce(&C::Current) -> R) -> R {
        self.enter().with(with)
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
    pub fn call<R>(mut self, scope: impl FnOnce() -> R) -> R {
        let __guard = self.enter();
        scope()
    }

    #[track_caller]
    #[must_use = "futures do nothing unless polled"]
    pub fn in_future<F>(self, future: F) -> FrameFuture<C, F> {
        FrameFuture {
            frame: self,
            future,
        }
    }
}

pub struct EnterGuard<'a, C: Ctxt> {
    scope: &'a mut Frame<C>,
    _marker: PhantomData<*mut fn()>,
}

impl<'a, C: Ctxt> EnterGuard<'a, C> {
    #[track_caller]
    pub fn with<R>(&mut self, with: impl FnOnce(&C::Current) -> R) -> R {
        self.scope.ctxt.with_current(with)
    }
}

impl<'a, C: Ctxt> Drop for EnterGuard<'a, C> {
    fn drop(&mut self) {
        self.scope.ctxt.exit(&mut self.scope.scope);
    }
}

impl<C: Ctxt> Drop for Frame<C> {
    fn drop(&mut self) {
        self.ctxt
            .close(unsafe { mem::ManuallyDrop::take(&mut self.scope) })
    }
}

pub struct FrameFuture<C: Ctxt, F> {
    frame: Frame<C>,
    future: F,
}

impl<C: Ctxt, F: Future> Future for FrameFuture<C, F> {
    type Output = F::Output;

    #[track_caller]
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let unpinned = unsafe { Pin::get_unchecked_mut(self) };

        let __guard = unpinned.frame.enter();
        unsafe { Pin::new_unchecked(&mut unpinned.future) }.poll(cx)
    }
}
