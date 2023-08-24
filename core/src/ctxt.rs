use crate::{empty::Empty, props::Props};

pub trait Ctxt {
    type CurrentProps: Props + ?Sized;
    type LocalFrame;

    fn open<P: Props>(&self, props: P) -> Self::LocalFrame;

    fn enter(&self, local: &mut Self::LocalFrame);

    fn with_current<F: FnOnce(&Self::CurrentProps)>(&self, with: F);

    fn exit(&self, local: &mut Self::LocalFrame);

    fn close(&self, frame: Self::LocalFrame);

    fn by_ref(&self) -> ByRef<Self> {
        ByRef(self)
    }
}

impl<'a, C: Ctxt + ?Sized> Ctxt for &'a C {
    type CurrentProps = C::CurrentProps;
    type LocalFrame = C::LocalFrame;

    fn open<P: Props>(&self, props: P) -> Self::LocalFrame {
        (**self).open(props)
    }

    fn enter(&self, frame: &mut Self::LocalFrame) {
        (**self).enter(frame)
    }

    fn with_current<F: FnOnce(&Self::CurrentProps)>(&self, with: F) {
        (**self).with_current(with)
    }

    fn exit(&self, frame: &mut Self::LocalFrame) {
        (**self).exit(frame)
    }

    fn close(&self, frame: Self::LocalFrame) {
        (**self).close(frame)
    }
}

impl<C: Ctxt> Ctxt for Option<C> {
    type CurrentProps = Option<internal::Slot<C::CurrentProps>>;
    type LocalFrame = Option<C::LocalFrame>;

    fn with_current<F: FnOnce(&Self::CurrentProps)>(&self, with: F) {
        match self {
            Some(ctxt) => {
                ctxt.with_current(|props| unsafe { with(&Some(internal::Slot::new(props))) })
            }
            None => with(&None),
        }
    }

    fn open<P: Props>(&self, props: P) -> Self::LocalFrame {
        self.as_ref().map(|ctxt| ctxt.open(props))
    }

    fn enter(&self, frame: &mut Self::LocalFrame) {
        if let (Some(ctxt), Some(span)) = (self, frame) {
            ctxt.enter(span)
        }
    }

    fn exit(&self, frame: &mut Self::LocalFrame) {
        if let (Some(ctxt), Some(span)) = (self, frame) {
            ctxt.exit(span)
        }
    }

    fn close(&self, frame: Self::LocalFrame) {
        if let (Some(ctxt), Some(span)) = (self, frame) {
            ctxt.close(span)
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a, C: Ctxt + ?Sized + 'a> Ctxt for alloc::boxed::Box<C> {
    type CurrentProps = C::CurrentProps;
    type LocalFrame = C::LocalFrame;

    fn with_current<F: FnOnce(&Self::CurrentProps)>(&self, with: F) {
        (**self).with_current(with)
    }

    fn open<P: Props>(&self, props: P) -> Self::LocalFrame {
        (**self).open(props)
    }

    fn enter(&self, frame: &mut Self::LocalFrame) {
        (**self).enter(frame)
    }

    fn exit(&self, frame: &mut Self::LocalFrame) {
        (**self).exit(frame)
    }

    fn close(&self, frame: Self::LocalFrame) {
        (**self).close(frame)
    }
}

pub struct ByRef<'a, T: ?Sized>(&'a T);

impl<'a, T: Ctxt + ?Sized> Ctxt for ByRef<'a, T> {
    type CurrentProps = T::CurrentProps;

    type LocalFrame = T::LocalFrame;

    fn open<P: Props>(&self, props: P) -> Self::LocalFrame {
        self.0.open(props)
    }

    fn enter(&self, frame: &mut Self::LocalFrame) {
        self.0.enter(frame)
    }

    fn with_current<F: FnOnce(&Self::CurrentProps)>(&self, with: F) {
        self.0.with_current(with)
    }

    fn exit(&self, frame: &mut Self::LocalFrame) {
        self.0.exit(frame)
    }

    fn close(&self, frame: Self::LocalFrame) {
        self.0.close(frame)
    }
}

impl Ctxt for Empty {
    type CurrentProps = Empty;
    type LocalFrame = Empty;

    fn with_current<F: FnOnce(&Self::CurrentProps)>(&self, with: F) {
        with(&Empty)
    }

    fn open<P: Props>(&self, _: P) -> Self::LocalFrame {
        Empty
    }

    fn enter(&self, _: &mut Self::LocalFrame) {}

    fn exit(&self, _: &mut Self::LocalFrame) {}

    fn close(&self, _: Self::LocalFrame) {}
}

mod internal {
    use core::{marker::PhantomData, ops::ControlFlow};

    use crate::{key::Key, props::Props, value::Value};

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
        fn for_each<'a, F: FnMut(Key<'a>, Value<'a>) -> ControlFlow<()>>(
            &'a self,
            for_each: F,
        ) -> ControlFlow<()> {
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

        use crate::{
            key::Key,
            props::{ErasedProps, Props},
            value::Value,
        };

        use super::ErasedLocalFrame;

        pub trait DispatchCtxt {
            fn dispatch_with_current(&self, with: &mut dyn FnMut(ErasedCurrentProps));

            fn dispatch_open(&self, props: &dyn ErasedProps) -> ErasedLocalFrame;
            fn dispatch_enter(&self, frame: &mut ErasedLocalFrame);
            fn dispatch_exit(&self, frame: &mut ErasedLocalFrame);
            fn dispatch_close(&self, frame: ErasedLocalFrame);
        }

        pub trait SealedCtxt {
            fn erase_ctxt(&self) -> crate::internal::Erased<&dyn DispatchCtxt>;
        }

        pub struct ErasedCurrentProps(
            *const dyn ErasedProps,
            PhantomData<fn(&mut dyn ErasedProps)>,
        );

        impl ErasedCurrentProps {
            pub(super) unsafe fn new<'a>(v: &'a impl Props) -> Self {
                let v: &'a dyn ErasedProps = v;
                let v: &'a (dyn ErasedProps + 'static) =
                    mem::transmute::<&'a dyn ErasedProps, &'a (dyn ErasedProps + 'static)>(v);

                ErasedCurrentProps(v as *const dyn ErasedProps, PhantomData)
            }

            pub(super) fn get<'a>(&'a self) -> &'a (dyn ErasedProps + 'a) {
                unsafe { &*self.0 }
            }
        }

        impl Props for ErasedCurrentProps {
            fn for_each<'a, F: FnMut(Key<'a>, Value<'a>) -> ControlFlow<()>>(
                &'a self,
                for_each: F,
            ) -> ControlFlow<()> {
                self.get().for_each(for_each)
            }
        }
    }

    pub struct ErasedLocalFrame(Box<dyn Any + Send>);

    pub trait ErasedCtxt: internal::SealedCtxt {}

    impl<C: Ctxt> ErasedCtxt for C where C::LocalFrame: Send + 'static {}

    impl<C: Ctxt> internal::SealedCtxt for C
    where
        C::LocalFrame: Send + 'static,
    {
        fn erase_ctxt(&self) -> crate::internal::Erased<&dyn internal::DispatchCtxt> {
            crate::internal::Erased(self)
        }
    }

    impl<C: Ctxt> internal::DispatchCtxt for C
    where
        C::LocalFrame: Send + 'static,
    {
        fn dispatch_with_current(&self, with: &mut dyn FnMut(internal::ErasedCurrentProps)) {
            self.with_current(move |props| {
                with(unsafe { internal::ErasedCurrentProps::new(&props) })
            })
        }

        fn dispatch_open(&self, props: &dyn ErasedProps) -> ErasedLocalFrame {
            ErasedLocalFrame(Box::new(self.open(props)))
        }

        fn dispatch_enter(&self, span: &mut ErasedLocalFrame) {
            if let Some(span) = span.0.downcast_mut() {
                self.enter(span)
            }
        }

        fn dispatch_exit(&self, span: &mut ErasedLocalFrame) {
            if let Some(span) = span.0.downcast_mut() {
                self.exit(span)
            }
        }

        fn dispatch_close(&self, span: ErasedLocalFrame) {
            if let Ok(span) = span.0.downcast() {
                self.close(*span)
            }
        }
    }

    impl<'a> Ctxt for dyn ErasedCtxt + 'a {
        type CurrentProps = internal::ErasedCurrentProps;
        type LocalFrame = ErasedLocalFrame;

        fn with_current<F: FnOnce(&Self::CurrentProps)>(&self, with: F) {
            let mut f = Some(with);

            self.erase_ctxt().0.dispatch_with_current(&mut |props| {
                f.take().expect("called multiple times")(&props)
            });
        }

        fn open<P: Props>(&self, props: P) -> Self::LocalFrame {
            self.erase_ctxt().0.dispatch_open(&props)
        }

        fn enter(&self, span: &mut Self::LocalFrame) {
            self.erase_ctxt().0.dispatch_enter(span)
        }

        fn exit(&self, span: &mut Self::LocalFrame) {
            self.erase_ctxt().0.dispatch_exit(span)
        }

        fn close(&self, span: Self::LocalFrame) {
            self.erase_ctxt().0.dispatch_close(span)
        }
    }

    impl<'a> Ctxt for dyn ErasedCtxt + Send + Sync + 'a {
        type CurrentProps = <dyn ErasedCtxt + 'a as Ctxt>::CurrentProps;
        type LocalFrame = <dyn ErasedCtxt + 'a as Ctxt>::LocalFrame;

        fn with_current<F: FnOnce(&Self::CurrentProps)>(&self, with: F) {
            (self as &(dyn ErasedCtxt + 'a)).with_current(with)
        }

        fn open<P: Props>(&self, props: P) -> Self::LocalFrame {
            (self as &(dyn ErasedCtxt + 'a)).open(props)
        }

        fn enter(&self, span: &mut Self::LocalFrame) {
            (self as &(dyn ErasedCtxt + 'a)).enter(span)
        }

        fn exit(&self, span: &mut Self::LocalFrame) {
            (self as &(dyn ErasedCtxt + 'a)).exit(span)
        }

        fn close(&self, span: Self::LocalFrame) {
            (self as &(dyn ErasedCtxt + 'a)).close(span)
        }
    }
}

#[cfg(feature = "alloc")]
pub use alloc_support::*;
