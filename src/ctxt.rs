use crate::{
    props::{self, Props},
    Val,
};

use self::internal::{ErasedSlot, Slot};

pub trait Ctxt {
    type Props: Props + ?Sized;

    fn with_ctxt<F: FnOnce(&Self::Props)>(&self, with: F);

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

impl<P: Props + ?Sized> Ctxt for P {
    type Props = Self;

    fn with_ctxt<F: FnOnce(&Self::Props)>(&self, with: F) {
        with(self)
    }
}

impl<T: Ctxt, U: Ctxt> Ctxt for Chain<T, U> {
    type Props = props::Chain<Slot<T::Props>, Slot<U::Props>>;

    fn with_ctxt<F: FnOnce(&Self::Props)>(&self, with: F) {
        self.first.with_ctxt(|first| {
            self.second.with_ctxt(|second| unsafe {
                with(&Props::chain(Slot::new(first), Slot::new(second)))
            })
        })
    }
}

impl<'a, T: Ctxt + 'a> Ctxt for ByRef<'a, T> {
    type Props = T::Props;

    fn with_ctxt<F: FnOnce(&Self::Props)>(&self, with: F) {
        self.0.with_ctxt(with)
    }
}

pub fn default() -> impl Ctxt {
    struct Empty;

    impl Ctxt for Empty {
        type Props = [(&'static str, Val<'static>); 0];

        fn with_ctxt<F: FnOnce(&Self::Props)>(&self, _: F) {}
    }

    Empty
}

pub struct Chain<T, U> {
    pub(crate) first: T,
    pub(crate) second: U,
}

pub struct ByRef<'a, T: ?Sized>(pub(crate) &'a T);

mod internal {
    use core::{marker::PhantomData, mem};

    use crate::{
        props::{ErasedProps, Visit},
        Props,
    };

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
        fn visit<'a, V: Visit<'a>>(&'a self, visitor: V) {
            self.get().visit(visitor)
        }
    }

    impl Props for ErasedSlot {
        fn visit<'a, V: Visit<'a>>(&'a self, visitor: V) {
            self.get().visit(visitor)
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
        self.with_ctxt(move |props| with(unsafe { ErasedSlot::new(&props) }))
    }
}

impl<'a> Ctxt for dyn ErasedCtxt + 'a {
    type Props = ErasedSlot;

    fn with_ctxt<F: FnOnce(&Self::Props)>(&self, with: F) {
        let mut f = Some(with);

        self.erase_ctxt()
            .0
            .dispatch_with_ctxt(&mut |props| f.take().expect("called multiple times")(&props));
    }
}
