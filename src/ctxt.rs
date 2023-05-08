use core::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use crate::props::{self, Props};

use self::internal::{ErasedSlot, Slot};

pub use crate::adapt::{ByRef, Chain, Discard, Empty};

pub trait PropsCtxt {
    type Props: Props + ?Sized;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F);

    fn by_ref(&self) -> ByRef<Self> {
        ByRef(self)
    }

    fn chain<U: PropsCtxt>(self, other: U) -> Chain<Self, U>
    where
        Self: Sized,
    {
        Chain {
            first: self,
            second: other,
        }
    }
}

impl<'a, C: PropsCtxt + ?Sized> PropsCtxt for &'a C {
    type Props = C::Props;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        (**self).with_props(with)
    }
}

#[cfg(feature = "std")]
impl<'a, C: PropsCtxt + ?Sized + 'a> PropsCtxt for Box<C> {
    type Props = C::Props;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        (**self).with_props(with)
    }
}

#[cfg(feature = "std")]
impl<'a, C: PropsCtxt + ?Sized + 'a> PropsCtxt for std::sync::Arc<C> {
    type Props = C::Props;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        (**self).with_props(with)
    }
}

impl<C: PropsCtxt> PropsCtxt for Option<C> {
    type Props = Option<Slot<C::Props>>;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        match self {
            Some(ctxt) => ctxt.with_props(|props| unsafe { with(&Some(Slot::new(props))) }),
            None => with(&None),
        }
    }
}

impl PropsCtxt for Empty {
    type Props = Self;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        with(self)
    }
}

impl<'a> PropsCtxt for props::SortedSlice<'a> {
    type Props = Self;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        with(self)
    }
}

impl<T: PropsCtxt, U: PropsCtxt> PropsCtxt for Chain<T, U> {
    type Props = Chain<Slot<T::Props>, Slot<U::Props>>;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        self.first.with_props(|first| {
            self.second.with_props(|second| unsafe {
                with(&Props::chain(Slot::new(first), Slot::new(second)))
            })
        })
    }
}

impl<'a, T: PropsCtxt + 'a> PropsCtxt for ByRef<'a, T> {
    type Props = T::Props;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        self.0.with_props(with)
    }
}

pub struct FromProps<P>(P);

impl<P: Props> PropsCtxt for FromProps<P> {
    type Props = P;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        with(&self.0)
    }
}

pub fn from_props<P: Props>(props: P) -> FromProps<P> {
    FromProps(props)
}

pub struct ReadOnly<C>(C);

impl<C: PropsCtxt> PropsCtxt for ReadOnly<C> {
    type Props = C::Props;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        self.0.with_props(with)
    }
}

impl<C: PropsCtxt> ScopeCtxt for ReadOnly<C> {
    type Scope = ();

    fn prepare<P: Props>(&self, _: P) -> Self::Scope {}

    fn enter(&self, _: &mut Self::Scope) {}

    fn exit(&self, _: &mut Self::Scope) {}
}

pub fn read_only<C: PropsCtxt>(ctxt: C) -> ReadOnly<C> {
    ReadOnly(ctxt)
}

pub trait ScopeCtxt: PropsCtxt {
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

    fn prepare<P: Props>(&self, props: P) -> Self::Scope;

    fn enter(&self, scope: &mut Self::Scope);
    fn exit(&self, scope: &mut Self::Scope);
}

impl<'a, C: ScopeCtxt + ?Sized> ScopeCtxt for &'a C {
    type Scope = C::Scope;

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

impl<C: ScopeCtxt> ScopeCtxt for Option<C> {
    type Scope = Option<C::Scope>;

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

#[cfg(feature = "std")]
impl<'a, C: ScopeCtxt + ?Sized + 'a> ScopeCtxt for Box<C> {
    type Scope = C::Scope;

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

#[cfg(feature = "std")]
impl<'a, C: ScopeCtxt + ?Sized + 'a> ScopeCtxt for std::sync::Arc<C> {
    type Scope = C::Scope;

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

pub struct Scope<C: ScopeCtxt> {
    scope: C::Scope,
    ctxt: C,
}

impl<C: ScopeCtxt> Scope<C> {
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

pub struct ScopeGuard<'a, C: ScopeCtxt> {
    scope: &'a mut Scope<C>,
    _marker: PhantomData<*mut fn()>,
}

impl<'a, C: ScopeCtxt> Drop for ScopeGuard<'a, C> {
    fn drop(&mut self) {
        self.scope.ctxt.exit(&mut self.scope.scope);
    }
}

pub struct ScopeFuture<C: ScopeCtxt, F> {
    scope: Scope<C>,
    future: F,
}

impl<C: ScopeCtxt, F> ScopeFuture<C, F> {
    fn new(scope: C, props: impl Props, future: F) -> Self {
        ScopeFuture {
            scope: Scope::new(scope, props),
            future,
        }
    }
}

impl<C: ScopeCtxt, F: Future> Future for ScopeFuture<C, F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let unpinned = unsafe { Pin::get_unchecked_mut(self) };

        let __guard = unpinned.scope.enter();
        unsafe { Pin::new_unchecked(&mut unpinned.future) }.poll(cx)
    }
}

impl PropsCtxt for Discard {
    type Props = Empty;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        with(&Empty)
    }
}

impl ScopeCtxt for Discard {
    type Scope = ();

    fn prepare<P: Props>(&self, _: P) -> Self::Scope {}

    fn enter(&self, _: &mut Self::Scope) {}

    fn exit(&self, _: &mut Self::Scope) {}
}

mod internal {
    use core::{marker::PhantomData, mem, ops::ControlFlow};

    use crate::{props::ErasedProps, Key, Props, Value};

    pub trait DispatchPropsCtxt {
        fn dispatch_with_props(&self, with: &mut dyn FnMut(ErasedSlot));
    }

    pub trait SealedPropsCtxt {
        fn erase_props_ctxt(&self) -> crate::internal::Erased<&dyn DispatchPropsCtxt>;
    }

    pub struct Slot<T: ?Sized>(*const T, PhantomData<*mut fn()>);

    impl<T: ?Sized> Slot<T> {
        pub(super) unsafe fn new(v: &T) -> Slot<T> {
            Slot(v as *const T, PhantomData)
        }

        pub(super) fn get(&self) -> &T {
            unsafe { &*self.0 }
        }
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

    impl<T: Props + ?Sized> Props for Slot<T> {
        fn for_each<'a, F: FnMut(Key<'a>, Value<'a>) -> ControlFlow<()>>(&'a self, for_each: F) {
            self.get().for_each(for_each)
        }
    }

    impl Props for ErasedSlot {
        fn for_each<'a, F: FnMut(Key<'a>, Value<'a>) -> ControlFlow<()>>(&'a self, for_each: F) {
            self.get().for_each(for_each)
        }
    }
}

pub trait ErasedPropsCtxt: internal::SealedPropsCtxt {}

impl<C: PropsCtxt> ErasedPropsCtxt for C {}

impl<C: PropsCtxt> internal::SealedPropsCtxt for C {
    fn erase_props_ctxt(&self) -> crate::internal::Erased<&dyn internal::DispatchPropsCtxt> {
        crate::internal::Erased(self)
    }
}

impl<C: PropsCtxt> internal::DispatchPropsCtxt for C {
    fn dispatch_with_props(&self, with: &mut dyn FnMut(ErasedSlot)) {
        self.with_props(move |props| with(unsafe { ErasedSlot::new(&props) }))
    }
}

impl<'a> PropsCtxt for dyn ErasedPropsCtxt + 'a {
    type Props = ErasedSlot;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        let mut f = Some(with);

        self.erase_props_ctxt()
            .0
            .dispatch_with_props(&mut |props| f.take().expect("called multiple times")(&props));
    }
}

impl<'a> PropsCtxt for dyn ErasedPropsCtxt + Send + Sync + 'a {
    type Props = <dyn ErasedPropsCtxt + 'a as PropsCtxt>::Props;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        (self as &(dyn ErasedPropsCtxt + 'a)).with_props(with)
    }
}

#[cfg(feature = "std")]
mod std_support {
    use core::any::Any;

    use crate::props::ErasedProps;

    use super::*;

    mod internal {
        use crate::props::ErasedProps;

        use super::ErasedScope;

        pub trait DispatchScopeCtxt {
            fn dispatch_prepare(&self, props: &dyn ErasedProps) -> ErasedScope;

            fn dispatch_enter(&self, link: &mut ErasedScope);
            fn dispatch_exit(&self, link: &mut ErasedScope);
        }

        pub trait SealedScopeCtxt {
            fn erase_scope_ctxt(&self) -> crate::internal::Erased<&dyn DispatchScopeCtxt>;
        }
    }

    pub struct ErasedScope(Box<dyn Any + Send>);

    pub trait ErasedScopeCtxt: internal::SealedScopeCtxt + ErasedPropsCtxt {}

    impl<C: ScopeCtxt> ErasedScopeCtxt for C where C::Scope: Send + 'static {}

    impl<C: ScopeCtxt> internal::SealedScopeCtxt for C
    where
        C::Scope: Send + 'static,
    {
        fn erase_scope_ctxt(&self) -> crate::internal::Erased<&dyn internal::DispatchScopeCtxt> {
            crate::internal::Erased(self)
        }
    }

    impl<C: ScopeCtxt> internal::DispatchScopeCtxt for C
    where
        C::Scope: Send + 'static,
    {
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

    impl<'a> PropsCtxt for dyn ErasedScopeCtxt + 'a {
        type Props = ErasedSlot;

        fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
            let mut f = Some(with);

            self.erase_props_ctxt()
                .0
                .dispatch_with_props(&mut |props| f.take().expect("called multiple times")(&props));
        }
    }

    impl<'a> ScopeCtxt for dyn ErasedScopeCtxt + 'a {
        type Scope = ErasedScope;

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

    impl<'a> PropsCtxt for dyn ErasedScopeCtxt + Send + Sync + 'a {
        type Props = <dyn ErasedScopeCtxt + 'a as PropsCtxt>::Props;

        fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
            (self as &(dyn ErasedScopeCtxt + 'a)).with_props(with)
        }
    }

    impl<'a> ScopeCtxt for dyn ErasedScopeCtxt + Send + Sync + 'a {
        type Scope = <dyn ErasedScopeCtxt + 'a as ScopeCtxt>::Scope;

        fn prepare<P: Props>(&self, props: P) -> Self::Scope {
            (self as &(dyn ErasedScopeCtxt + 'a)).prepare(props)
        }

        fn enter(&self, link: &mut Self::Scope) {
            (self as &(dyn ErasedScopeCtxt + 'a)).enter(link)
        }

        fn exit(&self, link: &mut Self::Scope) {
            (self as &(dyn ErasedScopeCtxt + 'a)).exit(link)
        }
    }
}

#[cfg(feature = "std")]
pub use std_support::*;
