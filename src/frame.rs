use core::{
    future::Future,
    marker::PhantomData,
    mem,
    pin::Pin,
    task::{Context, Poll},
};
use emit_core::{ctxt::Ctxt, props::Props};

pub trait FrameCtxt: Ctxt {
    fn push_frame<P: Props>(&self, props: P) -> Frame<&Self>;
    fn root_frame<P: Props>(&self, props: P) -> Frame<&Self>;
    fn current_frame(&self) -> Frame<&Self>;
}

impl<C: Ctxt + ?Sized> FrameCtxt for C {
    fn push_frame<P: Props>(&self, props: P) -> Frame<&Self> {
        Frame::new_push(self, props)
    }

    fn root_frame<P: Props>(&self, props: P) -> Frame<&Self> {
        Frame::new_root(self, props)
    }

    fn current_frame(&self) -> Frame<&Self> {
        Frame::new_push(self, crate::empty::Empty)
    }
}

pub struct Frame<C: Ctxt> {
    scope: mem::ManuallyDrop<C::Frame>,
    ctxt: C,
}

impl<C: Ctxt> Frame<C> {
    #[track_caller]
    pub fn new_push(ctxt: C, props: impl Props) -> Self {
        let scope = mem::ManuallyDrop::new(ctxt.open_push(props));

        Frame { ctxt, scope }
    }

    #[track_caller]
    pub fn new_root(ctxt: C, props: impl Props) -> Self {
        let scope = mem::ManuallyDrop::new(ctxt.open_root(props));

        Frame { ctxt, scope }
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
