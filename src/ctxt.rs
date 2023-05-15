use core::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use crate::props::Props;

#[cfg(feature = "std")]
pub mod thread_local;

pub use crate::adapt::{ByRef, Chain, Empty};

pub trait Ctxt {
    type Props: Props + ?Sized;
    type Scope;

    fn scope<P: Props>(self, props: P) -> Scope<Self>
    where
        Self: Sized,
    {
        Scope::new(self, props)
    }

    fn scope_future<P: Props, F: Future>(self, props: P, future: F) -> ScopeFuture<Self, F>
    where
        Self: Sized,
    {
        ScopeFuture::new(self, props, future)
    }

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F);

    fn prepare<P: Props>(&self, props: P) -> Self::Scope;

    fn enter(&self, scope: &mut Self::Scope);
    fn exit(&self, scope: &mut Self::Scope);
}

impl<'a, C: Ctxt + ?Sized> Ctxt for &'a C {
    type Props = C::Props;
    type Scope = C::Scope;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        (**self).with_props(with)
    }

    fn prepare<P: Props>(&self, props: P) -> Self::Scope {
        (**self).prepare(props)
    }

    fn enter(&self, link: &mut Self::Scope) {
        (**self).enter(link)
    }

    fn exit(&self, link: &mut Self::Scope) {
        (**self).exit(link)
    }
}

impl<C: Ctxt> Ctxt for Option<C> {
    type Props = Option<internal::Slot<C::Props>>;
    type Scope = Option<C::Scope>;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        match self {
            Some(ctxt) => {
                ctxt.with_props(|props| unsafe { with(&Some(internal::Slot::new(props))) })
            }
            None => with(&None),
        }
    }

    fn prepare<P: Props>(&self, props: P) -> Self::Scope {
        self.as_ref().map(|ctxt| ctxt.prepare(props))
    }

    fn enter(&self, link: &mut Self::Scope) {
        if let (Some(ctxt), Some(link)) = (self, link) {
            ctxt.enter(link)
        }
    }

    fn exit(&self, link: &mut Self::Scope) {
        if let (Some(ctxt), Some(link)) = (self, link) {
            ctxt.exit(link)
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a, C: Ctxt + ?Sized + 'a> Ctxt for alloc::boxed::Box<C> {
    type Props = C::Props;
    type Scope = C::Scope;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        (**self).with_props(with)
    }

    fn prepare<P: Props>(&self, props: P) -> Self::Scope {
        (**self).prepare(props)
    }

    fn enter(&self, link: &mut Self::Scope) {
        (**self).enter(link)
    }

    fn exit(&self, link: &mut Self::Scope) {
        (**self).exit(link)
    }
}

#[cfg(feature = "alloc")]
impl<'a, C: Ctxt + ?Sized + 'a> Ctxt for alloc::sync::Arc<C> {
    type Props = C::Props;
    type Scope = C::Scope;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        (**self).with_props(with)
    }

    fn prepare<P: Props>(&self, props: P) -> Self::Scope {
        (**self).prepare(props)
    }

    fn enter(&self, link: &mut Self::Scope) {
        (**self).enter(link)
    }

    fn exit(&self, link: &mut Self::Scope) {
        (**self).exit(link)
    }
}

pub struct Scope<C: Ctxt> {
    scope: C::Scope,
    ctxt: C,
}

impl<C: Ctxt> Scope<C> {
    fn new(ctxt: C, props: impl Props) -> Self {
        let scope = ctxt.prepare(props);

        Scope { ctxt, scope }
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
    scope: &'a mut Scope<C>,
    _marker: PhantomData<*mut fn()>,
}

impl<'a, C: Ctxt> Drop for ScopeGuard<'a, C> {
    fn drop(&mut self) {
        self.scope.ctxt.exit(&mut self.scope.scope);
    }
}

pub struct ScopeFuture<C: Ctxt, F> {
    scope: Scope<C>,
    future: F,
}

impl<C: Ctxt, F> ScopeFuture<C, F> {
    fn new(scope: C, props: impl Props, future: F) -> Self {
        ScopeFuture {
            scope: Scope::new(scope, props),
            future,
        }
    }
}

impl<C: Ctxt, F: Future> Future for ScopeFuture<C, F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let unpinned = unsafe { Pin::get_unchecked_mut(self) };

        let __guard = unpinned.scope.enter();
        unsafe { Pin::new_unchecked(&mut unpinned.future) }.poll(cx)
    }
}

impl Ctxt for Empty {
    type Props = Empty;
    type Scope = Empty;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        with(&Empty)
    }

    fn prepare<P: Props>(&self, _: P) -> Self::Scope {
        Empty
    }

    fn enter(&self, _: &mut Self::Scope) {}

    fn exit(&self, _: &mut Self::Scope) {}
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

        use crate::{props::ErasedProps, Key, Props, Value};

        use super::ErasedScope;

        pub trait DispatchCtxt {
            fn dispatch_with_props(&self, with: &mut dyn FnMut(ErasedSlot));

            fn dispatch_prepare(&self, props: &dyn ErasedProps) -> ErasedScope;

            fn dispatch_enter(&self, link: &mut ErasedScope);
            fn dispatch_exit(&self, link: &mut ErasedScope);
        }

        pub trait SealedCtxt {
            fn erase_scope_ctxt(&self) -> crate::internal::Erased<&dyn DispatchCtxt>;
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

    impl<C: Ctxt> ErasedCtxt for C where C::Scope: Send + 'static {}

    impl<C: Ctxt> internal::SealedCtxt for C
    where
        C::Scope: Send + 'static,
    {
        fn erase_scope_ctxt(&self) -> crate::internal::Erased<&dyn internal::DispatchCtxt> {
            crate::internal::Erased(self)
        }
    }

    impl<C: Ctxt> internal::DispatchCtxt for C
    where
        C::Scope: Send + 'static,
    {
        fn dispatch_with_props(&self, with: &mut dyn FnMut(internal::ErasedSlot)) {
            self.with_props(move |props| with(unsafe { internal::ErasedSlot::new(&props) }))
        }

        fn dispatch_prepare(&self, props: &dyn ErasedProps) -> ErasedScope {
            ErasedScope(Box::new(self.prepare(props)))
        }

        fn dispatch_enter(&self, link: &mut ErasedScope) {
            if let Some(link) = link.0.downcast_mut() {
                self.enter(link)
            }
        }

        fn dispatch_exit(&self, link: &mut ErasedScope) {
            if let Some(link) = link.0.downcast_mut() {
                self.exit(link)
            }
        }
    }

    impl<'a> Ctxt for dyn ErasedCtxt + 'a {
        type Props = internal::ErasedSlot;
        type Scope = ErasedScope;

        fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
            let mut f = Some(with);

            self.erase_scope_ctxt()
                .0
                .dispatch_with_props(&mut |props| f.take().expect("called multiple times")(&props));
        }

        fn prepare<P: Props>(&self, props: P) -> Self::Scope {
            self.erase_scope_ctxt().0.dispatch_prepare(&props)
        }

        fn enter(&self, link: &mut Self::Scope) {
            self.erase_scope_ctxt().0.dispatch_enter(link)
        }

        fn exit(&self, link: &mut Self::Scope) {
            self.erase_scope_ctxt().0.dispatch_exit(link)
        }
    }

    impl<'a> Ctxt for dyn ErasedCtxt + Send + Sync + 'a {
        type Props = <dyn ErasedCtxt + 'a as Ctxt>::Props;
        type Scope = <dyn ErasedCtxt + 'a as Ctxt>::Scope;

        fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
            (self as &(dyn ErasedCtxt + 'a)).with_props(with)
        }

        fn prepare<P: Props>(&self, props: P) -> Self::Scope {
            (self as &(dyn ErasedCtxt + 'a)).prepare(props)
        }

        fn enter(&self, link: &mut Self::Scope) {
            (self as &(dyn ErasedCtxt + 'a)).enter(link)
        }

        fn exit(&self, link: &mut Self::Scope) {
            (self as &(dyn ErasedCtxt + 'a)).exit(link)
        }
    }
}

#[cfg(feature = "alloc")]
pub use alloc_support::*;
