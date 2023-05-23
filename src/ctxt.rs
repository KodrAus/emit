use core::{
    future::Future,
    marker::PhantomData,
    mem,
    pin::Pin,
    task::{Context, Poll},
};

use crate::{props::Props, Id};

pub use crate::empty::Empty;

pub trait Ctxt {
    type Props: Props + ?Sized;
    type Span;

    fn span<P: Props>(self, id: Id, props: P) -> Span<Self>
    where
        Self: Sized,
    {
        Span::new(self, id, props)
    }

    fn span_future<P: Props, F: Future>(self, id: Id, props: P, future: F) -> SpanFuture<Self, F>
    where
        Self: Sized,
    {
        SpanFuture::new(self, id, props, future)
    }

    fn open<P: Props>(&self, id: Id, props: P) -> Self::Span;
    fn enter(&self, span: &mut Self::Span);

    fn with_current<F: FnOnce(Id, &Self::Props)>(&self, with: F);
    fn current_id(&self) -> Id {
        let mut current = Id::EMPTY;

        self.with_current(|id, _| {
            current = id;
        });

        current
    }

    fn exit(&self, span: &mut Self::Span);
    fn close(&self, span: Self::Span);
}

impl<'a, C: Ctxt + ?Sized> Ctxt for &'a C {
    type Props = C::Props;
    type Span = C::Span;

    fn with_current<F: FnOnce(Id, &Self::Props)>(&self, with: F) {
        (**self).with_current(with)
    }

    fn current_id(&self) -> Id {
        (**self).current_id()
    }

    fn open<P: Props>(&self, id: Id, props: P) -> Self::Span {
        (**self).open(id, props)
    }

    fn enter(&self, span: &mut Self::Span) {
        (**self).enter(span)
    }

    fn exit(&self, span: &mut Self::Span) {
        (**self).exit(span)
    }

    fn close(&self, scope: Self::Span) {
        (**self).close(scope)
    }
}

impl<C: Ctxt> Ctxt for Option<C> {
    type Props = Option<internal::Slot<C::Props>>;
    type Span = Option<C::Span>;

    fn with_current<F: FnOnce(Id, &Self::Props)>(&self, with: F) {
        match self {
            Some(ctxt) => ctxt
                .with_current(|id, props| unsafe { with(id, &Some(internal::Slot::new(props))) }),
            None => with(Id::EMPTY, &None),
        }
    }

    fn current_id(&self) -> Id {
        self.as_ref()
            .map(|ctxt| ctxt.current_id())
            .unwrap_or(Id::EMPTY)
    }

    fn open<P: Props>(&self, id: Id, props: P) -> Self::Span {
        self.as_ref().map(|ctxt| ctxt.open(id, props))
    }

    fn enter(&self, span: &mut Self::Span) {
        if let (Some(ctxt), Some(span)) = (self, span) {
            ctxt.enter(span)
        }
    }

    fn exit(&self, span: &mut Self::Span) {
        if let (Some(ctxt), Some(span)) = (self, span) {
            ctxt.exit(span)
        }
    }

    fn close(&self, span: Self::Span) {
        if let (Some(ctxt), Some(span)) = (self, span) {
            ctxt.close(span)
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a, C: Ctxt + ?Sized + 'a> Ctxt for alloc::boxed::Box<C> {
    type Props = C::Props;
    type Span = C::Span;

    fn with_current<F: FnOnce(Id, &Self::Props)>(&self, with: F) {
        (**self).with_current(with)
    }

    fn current_id(&self) -> Id {
        (**self).current_id()
    }

    fn open<P: Props>(&self, id: Id, props: P) -> Self::Span {
        (**self).open(id, props)
    }

    fn enter(&self, span: &mut Self::Span) {
        (**self).enter(span)
    }

    fn exit(&self, span: &mut Self::Span) {
        (**self).exit(span)
    }

    fn close(&self, span: Self::Span) {
        (**self).close(span)
    }
}

#[cfg(feature = "alloc")]
impl<'a, C: Ctxt + ?Sized + 'a> Ctxt for alloc::sync::Arc<C> {
    type Props = C::Props;
    type Span = C::Span;

    fn with_current<F: FnOnce(Id, &Self::Props)>(&self, with: F) {
        (**self).with_current(with)
    }

    fn current_id(&self) -> Id {
        (**self).current_id()
    }

    fn open<P: Props>(&self, id: Id, props: P) -> Self::Span {
        (**self).open(id, props)
    }

    fn enter(&self, span: &mut Self::Span) {
        (**self).enter(span)
    }

    fn exit(&self, span: &mut Self::Span) {
        (**self).exit(span)
    }

    fn close(&self, span: Self::Span) {
        (**self).close(span)
    }
}

pub struct Span<C: Ctxt> {
    scope: mem::ManuallyDrop<C::Span>,
    ctxt: C,
}

impl<C: Ctxt> Drop for Span<C> {
    fn drop(&mut self) {
        self.ctxt
            .close(unsafe { mem::ManuallyDrop::take(&mut self.scope) })
    }
}

impl<C: Ctxt> Span<C> {
    fn new(ctxt: C, id: Id, props: impl Props) -> Self {
        let scope = mem::ManuallyDrop::new(ctxt.open(id, props));

        Span { ctxt, scope }
    }

    pub fn enter(&mut self) -> ScopeGuard<C> {
        self.ctxt.enter(&mut self.scope);

        ScopeGuard {
            scope: self,
            _marker: PhantomData,
        }
    }
}

pub struct ScopeGuard<'a, C: Ctxt> {
    scope: &'a mut Span<C>,
    _marker: PhantomData<*mut fn()>,
}

impl<'a, C: Ctxt> Drop for ScopeGuard<'a, C> {
    fn drop(&mut self) {
        self.scope.ctxt.exit(&mut self.scope.scope);
    }
}

pub struct SpanFuture<C: Ctxt, F> {
    scope: Span<C>,
    future: F,
}

impl<C: Ctxt, F> SpanFuture<C, F> {
    fn new(scope: C, id: Id, props: impl Props, future: F) -> Self {
        SpanFuture {
            scope: Span::new(scope, id, props),
            future,
        }
    }
}

impl<C: Ctxt, F: Future> Future for SpanFuture<C, F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let unpinned = unsafe { Pin::get_unchecked_mut(self) };

        let __guard = unpinned.scope.enter();
        unsafe { Pin::new_unchecked(&mut unpinned.future) }.poll(cx)
    }
}

impl Ctxt for Empty {
    type Props = Empty;
    type Span = Empty;

    fn with_current<F: FnOnce(Id, &Self::Props)>(&self, with: F) {
        with(Id::EMPTY, &Empty)
    }

    fn current_id(&self) -> Id {
        Id::EMPTY
    }

    fn open<P: Props>(&self, _: Id, _: P) -> Self::Span {
        Empty
    }

    fn enter(&self, _: &mut Self::Span) {}

    fn exit(&self, _: &mut Self::Span) {}

    fn close(&self, _: Self::Span) {}
}

mod internal {
    use core::{marker::PhantomData, ops::ControlFlow};

    use crate::{Key, Props, Value};

    pub struct Slot<T: ?Sized>(*const T, PhantomData<*mut fn()>);

    impl<T: ?Sized> Slot<T> {
        pub(super) unsafe fn new(v: &T) -> Slot<T> {
            Slot(v as *const T, PhantomData)
        }

        pub(super) fn get(&self) -> &T {
            unsafe { &*self.0 }
        }
    }

    impl<T: Props + ?Sized> Props for Slot<T> {
        fn for_each<'a, F: FnMut(Key<'a>, Value<'a>) -> ControlFlow<()>>(&'a self, for_each: F) {
            self.get().for_each(for_each)
        }
    }
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use alloc::boxed::Box;
    use core::any::Any;

    use crate::props::ErasedProps;

    use super::*;

    mod internal {
        use core::{marker::PhantomData, mem, ops::ControlFlow};

        use crate::{ctxt::Id, props::ErasedProps, Key, Props, Value};

        use super::ErasedScope;

        pub trait DispatchCtxt {
            fn dispatch_with_current(&self, with: &mut dyn FnMut(Id, ErasedSlot));
            fn dispatch_current_id(&self) -> Id;

            fn dispatch_open(&self, id: Id, props: &dyn ErasedProps) -> ErasedScope;
            fn dispatch_enter(&self, span: &mut ErasedScope);
            fn dispatch_exit(&self, span: &mut ErasedScope);
            fn dispatch_close(&self, span: ErasedScope);
        }

        pub trait SealedCtxt {
            fn erase_ctxt(&self) -> crate::internal::Erased<&dyn DispatchCtxt>;
        }

        pub struct ErasedSlot(
            *const dyn ErasedProps,
            PhantomData<fn(&mut dyn ErasedProps)>,
        );

        impl ErasedSlot {
            pub(super) unsafe fn new<'a>(v: &'a impl Props) -> Self {
                let v: &'a dyn ErasedProps = v;
                let v: &'a (dyn ErasedProps + 'static) =
                    mem::transmute::<&'a dyn ErasedProps, &'a (dyn ErasedProps + 'static)>(v);

                ErasedSlot(v as *const dyn ErasedProps, PhantomData)
            }

            pub(super) fn get<'a>(&'a self) -> &'a (dyn ErasedProps + 'a) {
                unsafe { &*self.0 }
            }
        }

        impl Props for ErasedSlot {
            fn for_each<'a, F: FnMut(Key<'a>, Value<'a>) -> ControlFlow<()>>(
                &'a self,
                for_each: F,
            ) {
                self.get().for_each(for_each)
            }
        }
    }

    pub struct ErasedScope(Box<dyn Any + Send>);

    pub trait ErasedCtxt: internal::SealedCtxt {}

    impl<C: Ctxt> ErasedCtxt for C where C::Span: Send + 'static {}

    impl<C: Ctxt> internal::SealedCtxt for C
    where
        C::Span: Send + 'static,
    {
        fn erase_ctxt(&self) -> crate::internal::Erased<&dyn internal::DispatchCtxt> {
            crate::internal::Erased(self)
        }
    }

    impl<C: Ctxt> internal::DispatchCtxt for C
    where
        C::Span: Send + 'static,
    {
        fn dispatch_with_current(&self, with: &mut dyn FnMut(Id, internal::ErasedSlot)) {
            self.with_current(move |id, props| {
                with(id, unsafe { internal::ErasedSlot::new(&props) })
            })
        }

        fn dispatch_current_id(&self) -> Id {
            self.current_id()
        }

        fn dispatch_open(&self, id: Id, props: &dyn ErasedProps) -> ErasedScope {
            ErasedScope(Box::new(self.open(id, props)))
        }

        fn dispatch_enter(&self, span: &mut ErasedScope) {
            if let Some(span) = span.0.downcast_mut() {
                self.enter(span)
            }
        }

        fn dispatch_exit(&self, span: &mut ErasedScope) {
            if let Some(span) = span.0.downcast_mut() {
                self.exit(span)
            }
        }

        fn dispatch_close(&self, span: ErasedScope) {
            if let Ok(span) = span.0.downcast() {
                self.close(*span)
            }
        }
    }

    impl<'a> Ctxt for dyn ErasedCtxt + 'a {
        type Props = internal::ErasedSlot;
        type Span = ErasedScope;

        fn with_current<F: FnOnce(Id, &Self::Props)>(&self, with: F) {
            let mut f = Some(with);

            self.erase_ctxt().0.dispatch_with_current(&mut |id, props| {
                f.take().expect("called multiple times")(id, &props)
            });
        }

        fn current_id(&self) -> Id {
            self.erase_ctxt().0.dispatch_current_id()
        }

        fn open<P: Props>(&self, id: Id, props: P) -> Self::Span {
            self.erase_ctxt().0.dispatch_open(id, &props)
        }

        fn enter(&self, span: &mut Self::Span) {
            self.erase_ctxt().0.dispatch_enter(span)
        }

        fn exit(&self, span: &mut Self::Span) {
            self.erase_ctxt().0.dispatch_exit(span)
        }

        fn close(&self, span: Self::Span) {
            self.erase_ctxt().0.dispatch_close(span)
        }
    }

    impl<'a> Ctxt for dyn ErasedCtxt + Send + Sync + 'a {
        type Props = <dyn ErasedCtxt + 'a as Ctxt>::Props;
        type Span = <dyn ErasedCtxt + 'a as Ctxt>::Span;

        fn with_current<F: FnOnce(Id, &Self::Props)>(&self, with: F) {
            (self as &(dyn ErasedCtxt + 'a)).with_current(with)
        }

        fn current_id(&self) -> Id {
            (self as &(dyn ErasedCtxt + 'a)).current_id()
        }

        fn open<P: Props>(&self, id: Id, props: P) -> Self::Span {
            (self as &(dyn ErasedCtxt + 'a)).open(id, props)
        }

        fn enter(&self, span: &mut Self::Span) {
            (self as &(dyn ErasedCtxt + 'a)).enter(span)
        }

        fn exit(&self, span: &mut Self::Span) {
            (self as &(dyn ErasedCtxt + 'a)).exit(span)
        }

        fn close(&self, span: Self::Span) {
            (self as &(dyn ErasedCtxt + 'a)).close(span)
        }
    }
}

#[cfg(feature = "alloc")]
pub use alloc_support::*;
