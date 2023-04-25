use crate::props::{self, Props};

use self::internal::{ErasedSlot, Slot};

pub trait Ctxt {
    type Props: Props + ?Sized;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F);

    fn by_ref(&self) -> ByRef<Self> {
        ByRef(self)
    }

    fn chain<U: Ctxt>(self, other: U) -> Chain<Self, U>
    where
        Self: Sized,
    {
        Chain {
            first: self,
            second: other,
        }
    }
}

impl<'a, C: Ctxt + ?Sized> Ctxt for &'a C {
    type Props = C::Props;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        (**self).with_props(with)
    }
}

#[cfg(feature = "std")]
impl<'a, C: Ctxt + ?Sized + 'a> Ctxt for Box<C> {
    type Props = C::Props;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        (**self).with_props(with)
    }
}

impl<C: Ctxt> Ctxt for Option<C> {
    type Props = Option<Slot<C::Props>>;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        match self {
            Some(ctxt) => ctxt.with_props(|props| unsafe { with(&Some(Slot::new(props))) }),
            None => with(&None),
        }
    }
}

pub fn default() -> impl Ctxt {
    Empty
}

pub(crate) struct Empty;

impl Ctxt for Empty {
    type Props = props::Empty;

    fn with_props<F: FnOnce(&Self::Props)>(&self, _: F) {}
}

pub struct Chain<T, U> {
    pub(crate) first: T,
    pub(crate) second: U,
}

impl<T: Ctxt, U: Ctxt> Ctxt for Chain<T, U> {
    type Props = props::Chain<Slot<T::Props>, Slot<U::Props>>;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        self.first.with_props(|first| {
            self.second.with_props(|second| unsafe {
                with(&Props::chain(Slot::new(first), Slot::new(second)))
            })
        })
    }
}

pub struct ByRef<'a, T: ?Sized>(pub(crate) &'a T);

impl<'a, T: Ctxt + 'a> Ctxt for ByRef<'a, T> {
    type Props = T::Props;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        self.0.with_props(with)
    }
}

pub struct FromProps<P>(P);

impl<P: Props> Ctxt for FromProps<P> {
    type Props = P;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        with(&self.0)
    }
}

pub fn from_props<P: Props>(props: P) -> FromProps<P> {
    FromProps(props)
}

mod internal {
    use core::{marker::PhantomData, mem, ops::ControlFlow};

    use crate::{props::ErasedProps, Key, Props, Value};

    pub trait DispatchCtxt {
        fn dispatch_with_ctxt(&self, with: &mut dyn FnMut(ErasedSlot));
    }

    pub trait SealedCtxt {
        fn erase_ctxt(&self) -> crate::internal::Erased<&dyn DispatchCtxt>;
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

pub trait ErasedCtxt: internal::SealedCtxt {}

impl<C: Ctxt> ErasedCtxt for C {}

impl<C: Ctxt> internal::SealedCtxt for C {
    fn erase_ctxt(&self) -> crate::internal::Erased<&dyn internal::DispatchCtxt> {
        crate::internal::Erased(self)
    }
}

impl<C: Ctxt> internal::DispatchCtxt for C {
    fn dispatch_with_ctxt(&self, with: &mut dyn FnMut(ErasedSlot)) {
        self.with_props(move |props| with(unsafe { ErasedSlot::new(&props) }))
    }
}

impl<'a> Ctxt for dyn ErasedCtxt + 'a {
    type Props = ErasedSlot;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        let mut f = Some(with);

        self.erase_ctxt()
            .0
            .dispatch_with_ctxt(&mut |props| f.take().expect("called multiple times")(&props));
    }
}

impl<'a> Ctxt for dyn ErasedCtxt + Send + Sync + 'a {
    type Props = ErasedSlot;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        (self as &(dyn ErasedCtxt + 'a)).with_props(with)
    }
}
