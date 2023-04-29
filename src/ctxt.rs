use core::{
    future::Future,
    marker::PhantomData,
    pin::Pin,
    task::{Context, Poll},
};

use crate::props::{self, Props};

use self::internal::{ErasedSlot, Slot};

pub use crate::adapt::{ByRef, Chain, Discard, Empty};

pub trait GetCtxt {
    type Props: Props + ?Sized;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F);

    fn by_ref(&self) -> ByRef<Self> {
        ByRef(self)
    }

    fn chain<U: GetCtxt>(self, other: U) -> Chain<Self, U>
    where
        Self: Sized,
    {
        Chain {
            first: self,
            second: other,
        }
    }
}

impl<'a, C: GetCtxt + ?Sized> GetCtxt for &'a C {
    type Props = C::Props;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        (**self).with_props(with)
    }
}

#[cfg(feature = "std")]
impl<'a, C: GetCtxt + ?Sized + 'a> GetCtxt for Box<C> {
    type Props = C::Props;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        (**self).with_props(with)
    }
}

#[cfg(feature = "std")]
impl<'a, C: GetCtxt + ?Sized + 'a> GetCtxt for std::sync::Arc<C> {
    type Props = C::Props;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        (**self).with_props(with)
    }
}

impl<C: GetCtxt> GetCtxt for Option<C> {
    type Props = Option<Slot<C::Props>>;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        match self {
            Some(ctxt) => ctxt.with_props(|props| unsafe { with(&Some(Slot::new(props))) }),
            None => with(&None),
        }
    }
}

impl GetCtxt for Empty {
    type Props = Self;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        with(self)
    }
}

impl<'a> GetCtxt for props::SortedSlice<'a> {
    type Props = Self;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        with(self)
    }
}

impl<T: GetCtxt, U: GetCtxt> GetCtxt for Chain<T, U> {
    type Props = Chain<Slot<T::Props>, Slot<U::Props>>;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        self.first.with_props(|first| {
            self.second.with_props(|second| unsafe {
                with(&Props::chain(Slot::new(first), Slot::new(second)))
            })
        })
    }
}

impl<'a, T: GetCtxt + 'a> GetCtxt for ByRef<'a, T> {
    type Props = T::Props;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        self.0.with_props(with)
    }
}

pub struct FromProps<P>(P);

impl<P: Props> GetCtxt for FromProps<P> {
    type Props = P;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        with(&self.0)
    }
}

pub fn from_props<P: Props>(props: P) -> FromProps<P> {
    FromProps(props)
}

pub trait LinkCtxt: GetCtxt {
    type Link;

    fn prepare<P: Props>(&self, props: P) -> Self::Link;

    fn link(&self, link: &mut Self::Link);
    fn unlink(&self, link: &mut Self::Link);
}

impl<'a, C: LinkCtxt + ?Sized> LinkCtxt for &'a C {
    type Link = C::Link;

    fn prepare<P: Props>(&self, props: P) -> Self::Link {
        (**self).prepare(props)
    }

    fn link(&self, link: &mut Self::Link) {
        (**self).link(link)
    }

    fn unlink(&self, link: &mut Self::Link) {
        (**self).unlink(link)
    }
}

impl<C: LinkCtxt> LinkCtxt for Option<C> {
    type Link = Option<C::Link>;

    fn prepare<P: Props>(&self, props: P) -> Self::Link {
        self.as_ref().map(|ctxt| ctxt.prepare(props))
    }

    fn link(&self, link: &mut Self::Link) {
        if let (Some(ctxt), Some(link)) = (self, link) {
            ctxt.link(link)
        }
    }

    fn unlink(&self, link: &mut Self::Link) {
        if let (Some(ctxt), Some(link)) = (self, link) {
            ctxt.unlink(link)
        }
    }
}

#[cfg(feature = "std")]
impl<'a, C: LinkCtxt + ?Sized + 'a> LinkCtxt for Box<C> {
    type Link = C::Link;

    fn prepare<P: Props>(&self, props: P) -> Self::Link {
        (**self).prepare(props)
    }

    fn link(&self, link: &mut Self::Link) {
        (**self).link(link)
    }

    fn unlink(&self, link: &mut Self::Link) {
        (**self).unlink(link)
    }
}

#[cfg(feature = "std")]
impl<'a, C: LinkCtxt + ?Sized + 'a> LinkCtxt for std::sync::Arc<C> {
    type Link = C::Link;

    fn prepare<P: Props>(&self, props: P) -> Self::Link {
        (**self).prepare(props)
    }

    fn link(&self, link: &mut Self::Link) {
        (**self).link(link)
    }

    fn unlink(&self, link: &mut Self::Link) {
        (**self).unlink(link)
    }
}

pub struct Link<C: LinkCtxt> {
    link: C::Link,
    ctxt: C,
}

impl<C: LinkCtxt> Link<C> {
    pub fn new(ctxt: C, props: impl Props) -> Self {
        let link = ctxt.prepare(props);

        Link { ctxt, link }
    }

    pub fn link(&mut self) -> LinkGuard<C> {
        self.ctxt.link(&mut self.link);

        LinkGuard {
            link: self,
            _marker: PhantomData,
        }
    }
}

pub struct LinkGuard<'a, C: LinkCtxt> {
    link: &'a mut Link<C>,
    _marker: PhantomData<*mut fn()>,
}

impl<'a, C: LinkCtxt> Drop for LinkGuard<'a, C> {
    fn drop(&mut self) {
        self.link.ctxt.unlink(&mut self.link.link);
    }
}

pub struct LinkFuture<C: LinkCtxt, F> {
    link: Link<C>,
    future: F,
}

impl<C: LinkCtxt, F> LinkFuture<C, F> {
    pub fn new(ctxt: C, props: impl Props, future: F) -> Self {
        LinkFuture {
            link: Link::new(ctxt, props),
            future,
        }
    }
}

impl<C: LinkCtxt, F: Future> Future for LinkFuture<C, F> {
    type Output = F::Output;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let unpinned = unsafe { Pin::get_unchecked_mut(self) };

        let __guard = unpinned.link.link();
        unsafe { Pin::new_unchecked(&mut unpinned.future) }.poll(cx)
    }
}

impl GetCtxt for Discard {
    type Props = Empty;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        with(&Empty)
    }
}

impl LinkCtxt for Discard {
    type Link = ();

    fn prepare<P: Props>(&self, _: P) -> Self::Link {}

    fn link(&self, _: &mut Self::Link) {}

    fn unlink(&self, _: &mut Self::Link) {}
}

mod internal {
    use core::{marker::PhantomData, mem, ops::ControlFlow};

    use crate::{props::ErasedProps, Key, Props, Value};

    pub trait DispatchGetCtxt {
        fn dispatch_with_ctxt(&self, with: &mut dyn FnMut(ErasedSlot));
    }

    pub trait SealedGetCtxt {
        fn erase_get_ctxt(&self) -> crate::internal::Erased<&dyn DispatchGetCtxt>;
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

pub trait ErasedGetCtxt: internal::SealedGetCtxt {}

impl<C: GetCtxt> ErasedGetCtxt for C {}

impl<C: GetCtxt> internal::SealedGetCtxt for C {
    fn erase_get_ctxt(&self) -> crate::internal::Erased<&dyn internal::DispatchGetCtxt> {
        crate::internal::Erased(self)
    }
}

impl<C: GetCtxt> internal::DispatchGetCtxt for C {
    fn dispatch_with_ctxt(&self, with: &mut dyn FnMut(ErasedSlot)) {
        self.with_props(move |props| with(unsafe { ErasedSlot::new(&props) }))
    }
}

impl<'a> GetCtxt for dyn ErasedGetCtxt + 'a {
    type Props = ErasedSlot;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        let mut f = Some(with);

        self.erase_get_ctxt()
            .0
            .dispatch_with_ctxt(&mut |props| f.take().expect("called multiple times")(&props));
    }
}

impl<'a> GetCtxt for dyn ErasedGetCtxt + Send + Sync + 'a {
    type Props = <dyn ErasedGetCtxt + 'a as GetCtxt>::Props;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        (self as &(dyn ErasedGetCtxt + 'a)).with_props(with)
    }
}

#[cfg(feature = "std")]
mod std_support {
    use core::any::Any;

    use crate::props::ErasedProps;

    use super::*;

    mod internal {
        use crate::props::ErasedProps;

        use super::ErasedLink;

        pub trait DispatchLinkCtxt {
            fn dispatch_prepare(&self, props: &dyn ErasedProps) -> ErasedLink;

            fn dispatch_link(&self, link: &mut ErasedLink);
            fn dispatch_unlink(&self, link: &mut ErasedLink);
        }

        pub trait SealedLinkCtxt {
            fn erase_set_ctxt(&self) -> crate::internal::Erased<&dyn DispatchLinkCtxt>;
        }
    }

    pub struct ErasedLink(Box<dyn Any + Send>);

    pub trait ErasedLinkCtxt: internal::SealedLinkCtxt + ErasedGetCtxt {}

    impl<C: LinkCtxt> ErasedLinkCtxt for C where C::Link: Send + 'static {}

    impl<C: LinkCtxt> internal::SealedLinkCtxt for C
    where
        C::Link: Send + 'static,
    {
        fn erase_set_ctxt(&self) -> crate::internal::Erased<&dyn internal::DispatchLinkCtxt> {
            crate::internal::Erased(self)
        }
    }

    impl<C: LinkCtxt> internal::DispatchLinkCtxt for C
    where
        C::Link: Send + 'static,
    {
        fn dispatch_prepare(&self, props: &dyn ErasedProps) -> ErasedLink {
            ErasedLink(Box::new(self.prepare(props)))
        }

        fn dispatch_link(&self, link: &mut ErasedLink) {
            if let Some(link) = link.0.downcast_mut() {
                self.link(link)
            }
        }

        fn dispatch_unlink(&self, link: &mut ErasedLink) {
            if let Some(link) = link.0.downcast_mut() {
                self.unlink(link)
            }
        }
    }

    impl<'a> GetCtxt for dyn ErasedLinkCtxt + 'a {
        type Props = ErasedSlot;

        fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
            let mut f = Some(with);

            self.erase_get_ctxt()
                .0
                .dispatch_with_ctxt(&mut |props| f.take().expect("called multiple times")(&props));
        }
    }

    impl<'a> LinkCtxt for dyn ErasedLinkCtxt + 'a {
        type Link = ErasedLink;

        fn prepare<P: Props>(&self, props: P) -> Self::Link {
            self.erase_set_ctxt().0.dispatch_prepare(&props)
        }

        fn link(&self, link: &mut Self::Link) {
            self.erase_set_ctxt().0.dispatch_link(link)
        }

        fn unlink(&self, link: &mut Self::Link) {
            self.erase_set_ctxt().0.dispatch_unlink(link)
        }
    }

    impl<'a> GetCtxt for dyn ErasedLinkCtxt + Send + Sync + 'a {
        type Props = <dyn ErasedLinkCtxt + 'a as GetCtxt>::Props;

        fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
            (self as &(dyn ErasedLinkCtxt + 'a)).with_props(with)
        }
    }

    impl<'a> LinkCtxt for dyn ErasedLinkCtxt + Send + Sync + 'a {
        type Link = <dyn ErasedLinkCtxt + 'a as LinkCtxt>::Link;

        fn prepare<P: Props>(&self, props: P) -> Self::Link {
            (self as &(dyn ErasedLinkCtxt + 'a)).prepare(props)
        }

        fn link(&self, link: &mut Self::Link) {
            (self as &(dyn ErasedLinkCtxt + 'a)).link(link)
        }

        fn unlink(&self, link: &mut Self::Link) {
            (self as &(dyn ErasedLinkCtxt + 'a)).unlink(link)
        }
    }
}

#[cfg(feature = "std")]
pub use std_support::*;
