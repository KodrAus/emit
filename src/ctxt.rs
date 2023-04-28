use crate::{
    props::{self, Props},
    ByRef, Chain,
};

use self::internal::{ErasedSlot, Slot};

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

impl GetCtxt for props::Empty {
    type Props = Self;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        with(self)
    }
}

impl<C: SetCtxt> SetCtxt for Option<C> {
    type Link = Option<C::Link>;

    fn link<P: Props>(&self, props: P) -> Self::Link {
        self.as_ref().map(|ctxt| ctxt.link(props))
    }

    fn unlink(&self, link: Self::Link) {
        if let (Some(ctxt), Some(link)) = (self, link) {
            ctxt.unlink(link)
        }
    }

    fn activate(&self, link: &mut Self::Link) {
        if let (Some(ctxt), Some(link)) = (self, link) {
            ctxt.activate(link)
        }
    }

    fn deactivate(&self, link: &mut Self::Link) {
        if let (Some(ctxt), Some(link)) = (self, link) {
            ctxt.deactivate(link)
        }
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

pub trait SetCtxt {
    type Link;

    fn link<P: Props>(&self, props: P) -> Self::Link;
    fn unlink(&self, link: Self::Link);

    fn activate(&self, link: &mut Self::Link);
    fn deactivate(&self, link: &mut Self::Link);
}

impl<'a, C: SetCtxt + ?Sized> SetCtxt for &'a C {
    type Link = C::Link;

    fn link<P: Props>(&self, props: P) -> Self::Link {
        (**self).link(props)
    }

    fn unlink(&self, link: Self::Link) {
        (**self).unlink(link)
    }

    fn activate(&self, link: &mut Self::Link) {
        (**self).activate(link)
    }

    fn deactivate(&self, link: &mut Self::Link) {
        (**self).deactivate(link)
    }
}

#[cfg(feature = "std")]
impl<'a, C: SetCtxt + ?Sized + 'a> SetCtxt for Box<C> {
    type Link = C::Link;

    fn link<P: Props>(&self, props: P) -> Self::Link {
        (**self).link(props)
    }

    fn unlink(&self, link: Self::Link) {
        (**self).unlink(link)
    }

    fn activate(&self, link: &mut Self::Link) {
        (**self).activate(link)
    }

    fn deactivate(&self, link: &mut Self::Link) {
        (**self).deactivate(link)
    }
}

#[cfg(feature = "std")]
impl<'a, C: SetCtxt + ?Sized + 'a> SetCtxt for std::sync::Arc<C> {
    type Link = C::Link;

    fn link<P: Props>(&self, props: P) -> Self::Link {
        (**self).link(props)
    }

    fn unlink(&self, link: Self::Link) {
        (**self).unlink(link)
    }

    fn activate(&self, link: &mut Self::Link) {
        (**self).activate(link)
    }

    fn deactivate(&self, link: &mut Self::Link) {
        (**self).deactivate(link)
    }
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

    pub struct Slot<T: ?Sized>(*const T, PhantomData<fn(&mut T)>);

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

        pub trait DispatchSetCtxt {
            fn dispatch_link(&self, props: &dyn ErasedProps) -> ErasedLink;
            fn dispatch_unlink(&self, link: ErasedLink);

            fn dispatch_activate(&self, link: &mut ErasedLink);
            fn dispatch_deactivate(&self, link: &mut ErasedLink);
        }

        pub trait SealedSetCtxt {
            fn erase_set_ctxt(&self) -> crate::internal::Erased<&dyn DispatchSetCtxt>;
        }
    }

    pub struct ErasedLink(Box<dyn Any>);

    pub trait ErasedSetCtxt: internal::SealedSetCtxt {}

    impl<C: SetCtxt> ErasedSetCtxt for C where C::Link: 'static {}

    impl<C: SetCtxt> internal::SealedSetCtxt for C
    where
        C::Link: 'static,
    {
        fn erase_set_ctxt(&self) -> crate::internal::Erased<&dyn internal::DispatchSetCtxt> {
            crate::internal::Erased(self)
        }
    }

    impl<C: SetCtxt> internal::DispatchSetCtxt for C
    where
        C::Link: 'static,
    {
        fn dispatch_link(&self, props: &dyn ErasedProps) -> ErasedLink {
            ErasedLink(Box::new(self.link(props)))
        }

        fn dispatch_unlink(&self, link: ErasedLink) {
            if let Ok(link) = link.0.downcast() {
                self.unlink(*link)
            }
        }

        fn dispatch_activate(&self, link: &mut ErasedLink) {
            if let Some(link) = link.0.downcast_mut() {
                self.activate(link)
            }
        }

        fn dispatch_deactivate(&self, link: &mut ErasedLink) {
            if let Some(link) = link.0.downcast_mut() {
                self.deactivate(link)
            }
        }
    }

    impl<'a> SetCtxt for dyn ErasedSetCtxt + 'a {
        type Link = ErasedLink;

        fn link<P: Props>(&self, props: P) -> Self::Link {
            self.erase_set_ctxt().0.dispatch_link(&props)
        }

        fn unlink(&self, link: Self::Link) {
            self.erase_set_ctxt().0.dispatch_unlink(link)
        }

        fn activate(&self, link: &mut Self::Link) {
            self.erase_set_ctxt().0.dispatch_activate(link)
        }

        fn deactivate(&self, link: &mut Self::Link) {
            self.erase_set_ctxt().0.dispatch_deactivate(link)
        }
    }

    impl<'a> SetCtxt for dyn ErasedSetCtxt + Send + Sync + 'a {
        type Link = <dyn ErasedSetCtxt + 'a as SetCtxt>::Link;

        fn link<P: Props>(&self, props: P) -> Self::Link {
            (self as &(dyn ErasedSetCtxt + 'a)).link(props)
        }

        fn unlink(&self, link: Self::Link) {
            (self as &(dyn ErasedSetCtxt + 'a)).unlink(link)
        }

        fn activate(&self, link: &mut Self::Link) {
            (self as &(dyn ErasedSetCtxt + 'a)).activate(link)
        }

        fn deactivate(&self, link: &mut Self::Link) {
            (self as &(dyn ErasedSetCtxt + 'a)).deactivate(link)
        }
    }
}

#[cfg(feature = "std")]
pub use std_support::*;
