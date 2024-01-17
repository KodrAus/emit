use crate::{empty::Empty, props::Props};

pub trait Ctxt {
    type Props: Props + ?Sized;
    type Frame;

    fn open<P: Props>(&self, props: P) -> Self::Frame;

    fn enter(&self, local: &mut Self::Frame);

    fn with_current<F: FnOnce(&Self::Props)>(&self, with: F);

    fn exit(&self, local: &mut Self::Frame);

    fn close(&self, frame: Self::Frame);

    fn by_ref(&self) -> ByRef<Self> {
        ByRef(self)
    }
}

impl<'a, C: Ctxt + ?Sized> Ctxt for &'a C {
    type Props = C::Props;
    type Frame = C::Frame;

    fn open<P: Props>(&self, props: P) -> Self::Frame {
        (**self).open(props)
    }

    fn enter(&self, frame: &mut Self::Frame) {
        (**self).enter(frame)
    }

    fn with_current<F: FnOnce(&Self::Props)>(&self, with: F) {
        (**self).with_current(with)
    }

    fn exit(&self, frame: &mut Self::Frame) {
        (**self).exit(frame)
    }

    fn close(&self, frame: Self::Frame) {
        (**self).close(frame)
    }
}

impl<C: Ctxt> Ctxt for Option<C> {
    type Props = Option<internal::Slot<C::Props>>;
    type Frame = Option<C::Frame>;

    fn with_current<F: FnOnce(&Self::Props)>(&self, with: F) {
        match self {
            Some(ctxt) => {
                ctxt.with_current(|props| unsafe { with(&Some(internal::Slot::new(props))) })
            }
            None => with(&None),
        }
    }

    fn open<P: Props>(&self, props: P) -> Self::Frame {
        self.as_ref().map(|ctxt| ctxt.open(props))
    }

    fn enter(&self, frame: &mut Self::Frame) {
        if let (Some(ctxt), Some(span)) = (self, frame) {
            ctxt.enter(span)
        }
    }

    fn exit(&self, frame: &mut Self::Frame) {
        if let (Some(ctxt), Some(span)) = (self, frame) {
            ctxt.exit(span)
        }
    }

    fn close(&self, frame: Self::Frame) {
        if let (Some(ctxt), Some(span)) = (self, frame) {
            ctxt.close(span)
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a, C: Ctxt + ?Sized + 'a> Ctxt for alloc::boxed::Box<C> {
    type Props = C::Props;
    type Frame = C::Frame;

    fn with_current<F: FnOnce(&Self::Props)>(&self, with: F) {
        (**self).with_current(with)
    }

    fn open<P: Props>(&self, props: P) -> Self::Frame {
        (**self).open(props)
    }

    fn enter(&self, frame: &mut Self::Frame) {
        (**self).enter(frame)
    }

    fn exit(&self, frame: &mut Self::Frame) {
        (**self).exit(frame)
    }

    fn close(&self, frame: Self::Frame) {
        (**self).close(frame)
    }
}

pub struct ByRef<'a, T: ?Sized>(&'a T);

impl<'a, T: Ctxt + ?Sized> Ctxt for ByRef<'a, T> {
    type Props = T::Props;

    type Frame = T::Frame;

    fn open<P: Props>(&self, props: P) -> Self::Frame {
        self.0.open(props)
    }

    fn enter(&self, frame: &mut Self::Frame) {
        self.0.enter(frame)
    }

    fn with_current<F: FnOnce(&Self::Props)>(&self, with: F) {
        self.0.with_current(with)
    }

    fn exit(&self, frame: &mut Self::Frame) {
        self.0.exit(frame)
    }

    fn close(&self, frame: Self::Frame) {
        self.0.close(frame)
    }
}

impl Ctxt for Empty {
    type Props = Empty;
    type Frame = Empty;

    fn with_current<F: FnOnce(&Self::Props)>(&self, with: F) {
        with(&Empty)
    }

    fn open<P: Props>(&self, _: P) -> Self::Frame {
        Empty
    }

    fn enter(&self, _: &mut Self::Frame) {}

    fn exit(&self, _: &mut Self::Frame) {}

    fn close(&self, _: Self::Frame) {}
}

mod internal {
    use core::{marker::PhantomData, ops::ControlFlow};

    use crate::{props::Props, str::Str, value::Value};

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
        fn for_each<'a, F: FnMut(Str<'a>, Value<'a>) -> ControlFlow<()>>(
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
            props::{ErasedProps, Props},
            str::Str,
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
            fn for_each<'a, F: FnMut(Str<'a>, Value<'a>) -> ControlFlow<()>>(
                &'a self,
                for_each: F,
            ) -> ControlFlow<()> {
                self.get().for_each(for_each)
            }
        }
    }

    pub struct ErasedLocalFrame(Box<dyn Any + Send>);

    pub trait ErasedCtxt: internal::SealedCtxt {}

    impl<C: Ctxt> ErasedCtxt for C where C::Frame: Send + 'static {}

    impl<C: Ctxt> internal::SealedCtxt for C
    where
        C::Frame: Send + 'static,
    {
        fn erase_ctxt(&self) -> crate::internal::Erased<&dyn internal::DispatchCtxt> {
            crate::internal::Erased(self)
        }
    }

    impl<C: Ctxt> internal::DispatchCtxt for C
    where
        C::Frame: Send + 'static,
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
        type Props = internal::ErasedCurrentProps;
        type Frame = ErasedLocalFrame;

        fn with_current<F: FnOnce(&Self::Props)>(&self, with: F) {
            let mut f = Some(with);

            self.erase_ctxt().0.dispatch_with_current(&mut |props| {
                f.take().expect("called multiple times")(&props)
            });
        }

        fn open<P: Props>(&self, props: P) -> Self::Frame {
            self.erase_ctxt().0.dispatch_open(&props)
        }

        fn enter(&self, span: &mut Self::Frame) {
            self.erase_ctxt().0.dispatch_enter(span)
        }

        fn exit(&self, span: &mut Self::Frame) {
            self.erase_ctxt().0.dispatch_exit(span)
        }

        fn close(&self, span: Self::Frame) {
            self.erase_ctxt().0.dispatch_close(span)
        }
    }

    impl<'a> Ctxt for dyn ErasedCtxt + Send + Sync + 'a {
        type Props = <dyn ErasedCtxt + 'a as Ctxt>::Props;
        type Frame = <dyn ErasedCtxt + 'a as Ctxt>::Frame;

        fn with_current<F: FnOnce(&Self::Props)>(&self, with: F) {
            (self as &(dyn ErasedCtxt + 'a)).with_current(with)
        }

        fn open<P: Props>(&self, props: P) -> Self::Frame {
            (self as &(dyn ErasedCtxt + 'a)).open(props)
        }

        fn enter(&self, span: &mut Self::Frame) {
            (self as &(dyn ErasedCtxt + 'a)).enter(span)
        }

        fn exit(&self, span: &mut Self::Frame) {
            (self as &(dyn ErasedCtxt + 'a)).exit(span)
        }

        fn close(&self, span: Self::Frame) {
            (self as &(dyn ErasedCtxt + 'a)).close(span)
        }
    }
}

#[cfg(feature = "alloc")]
pub use alloc_support::*;
