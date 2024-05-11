use crate::{empty::Empty, props::Props};

pub trait Ctxt {
    type Current: Props + ?Sized;
    type Frame;

    fn open_root<P: Props>(&self, props: P) -> Self::Frame;

    fn open_push<P: Props>(&self, props: P) -> Self::Frame {
        self.with_current(|current| self.open_root(props.and_props(current)))
    }

    fn enter(&self, local: &mut Self::Frame);

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R;

    fn exit(&self, local: &mut Self::Frame);

    fn close(&self, frame: Self::Frame);
}

impl<'a, C: Ctxt + ?Sized> Ctxt for &'a C {
    type Current = C::Current;
    type Frame = C::Frame;

    fn open_root<P: Props>(&self, props: P) -> Self::Frame {
        (**self).open_root(props)
    }

    fn open_push<P: Props>(&self, props: P) -> Self::Frame {
        (**self).open_push(props)
    }

    fn enter(&self, frame: &mut Self::Frame) {
        (**self).enter(frame)
    }

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
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
    type Current = Option<internal::Slot<C::Current>>;
    type Frame = Option<C::Frame>;

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        match self {
            Some(ctxt) => {
                ctxt.with_current(|props| unsafe { with(&Some(internal::Slot::new(props))) })
            }
            None => with(&None),
        }
    }

    fn open_root<P: Props>(&self, props: P) -> Self::Frame {
        self.as_ref().map(|ctxt| ctxt.open_root(props))
    }

    fn open_push<P: Props>(&self, props: P) -> Self::Frame {
        self.as_ref().map(|ctxt| ctxt.open_push(props))
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
    type Current = C::Current;
    type Frame = C::Frame;

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        (**self).with_current(with)
    }

    fn open_root<P: Props>(&self, props: P) -> Self::Frame {
        (**self).open_root(props)
    }

    fn open_push<P: Props>(&self, props: P) -> Self::Frame {
        (**self).open_push(props)
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

#[cfg(feature = "alloc")]
impl<'a, C: Ctxt + ?Sized + 'a> Ctxt for alloc::sync::Arc<C> {
    type Current = C::Current;
    type Frame = C::Frame;

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        (**self).with_current(with)
    }

    fn open_root<P: Props>(&self, props: P) -> Self::Frame {
        (**self).open_root(props)
    }

    fn open_push<P: Props>(&self, props: P) -> Self::Frame {
        (**self).open_push(props)
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

impl Ctxt for Empty {
    type Current = Empty;
    type Frame = Empty;

    fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
        with(&Empty)
    }

    fn open_root<P: Props>(&self, _: P) -> Self::Frame {
        Empty
    }

    fn open_push<P: Props>(&self, _: P) -> Self::Frame {
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

        use super::ErasedFrame;

        pub trait DispatchCtxt {
            fn dispatch_with_current(&self, with: &mut dyn FnMut(ErasedCurrent));

            fn dispatch_open_root(&self, props: &dyn ErasedProps) -> ErasedFrame;
            fn dispatch_open_push(&self, props: &dyn ErasedProps) -> ErasedFrame;
            fn dispatch_enter(&self, frame: &mut ErasedFrame);
            fn dispatch_exit(&self, frame: &mut ErasedFrame);
            fn dispatch_close(&self, frame: ErasedFrame);
        }

        pub trait SealedCtxt {
            fn erase_ctxt(&self) -> crate::internal::Erased<&dyn DispatchCtxt>;
        }

        pub struct ErasedCurrent(
            *const dyn ErasedProps,
            PhantomData<fn(&mut dyn ErasedProps)>,
        );

        impl ErasedCurrent {
            pub(super) unsafe fn new<'a>(v: &'a impl Props) -> Self {
                let v: &'a dyn ErasedProps = v;
                let v: &'a (dyn ErasedProps + 'static) =
                    mem::transmute::<&'a dyn ErasedProps, &'a (dyn ErasedProps + 'static)>(v);

                ErasedCurrent(v as *const dyn ErasedProps, PhantomData)
            }

            pub(super) fn get<'a>(&'a self) -> &'a (dyn ErasedProps + 'a) {
                unsafe { &*self.0 }
            }
        }

        impl Props for ErasedCurrent {
            fn for_each<'a, F: FnMut(Str<'a>, Value<'a>) -> ControlFlow<()>>(
                &'a self,
                for_each: F,
            ) -> ControlFlow<()> {
                self.get().for_each(for_each)
            }
        }
    }

    pub struct ErasedFrame(Box<dyn Any + Send>);

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
        fn dispatch_with_current(&self, with: &mut dyn FnMut(internal::ErasedCurrent)) {
            self.with_current(move |props| with(unsafe { internal::ErasedCurrent::new(&props) }))
        }

        fn dispatch_open_root(&self, props: &dyn ErasedProps) -> ErasedFrame {
            ErasedFrame(Box::new(self.open_root(props)))
        }

        fn dispatch_open_push(&self, props: &dyn ErasedProps) -> ErasedFrame {
            // TODO: For pointer-sized frames we could consider inlining
            // to avoid boxing
            ErasedFrame(Box::new(self.open_push(props)))
        }

        fn dispatch_enter(&self, span: &mut ErasedFrame) {
            if let Some(span) = span.0.downcast_mut() {
                self.enter(span)
            }
        }

        fn dispatch_exit(&self, span: &mut ErasedFrame) {
            if let Some(span) = span.0.downcast_mut() {
                self.exit(span)
            }
        }

        fn dispatch_close(&self, span: ErasedFrame) {
            if let Ok(span) = span.0.downcast() {
                self.close(*span)
            }
        }
    }

    impl<'a> Ctxt for dyn ErasedCtxt + 'a {
        type Current = internal::ErasedCurrent;
        type Frame = ErasedFrame;

        fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
            let mut f = Some(with);
            let mut r = None;

            self.erase_ctxt().0.dispatch_with_current(&mut |props| {
                r = Some(f.take().expect("called multiple times")(&props));
            });

            r.expect("ctxt didn't call `with`")
        }

        fn open_root<P: Props>(&self, props: P) -> Self::Frame {
            self.erase_ctxt().0.dispatch_open_root(&props)
        }

        fn open_push<P: Props>(&self, props: P) -> Self::Frame {
            self.erase_ctxt().0.dispatch_open_push(&props)
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
        type Current = <dyn ErasedCtxt + 'a as Ctxt>::Current;
        type Frame = <dyn ErasedCtxt + 'a as Ctxt>::Frame;

        fn with_current<R, F: FnOnce(&Self::Current) -> R>(&self, with: F) -> R {
            (self as &(dyn ErasedCtxt + 'a)).with_current(with)
        }

        fn open_root<P: Props>(&self, props: P) -> Self::Frame {
            (self as &(dyn ErasedCtxt + 'a)).open_root(props)
        }

        fn open_push<P: Props>(&self, props: P) -> Self::Frame {
            (self as &(dyn ErasedCtxt + 'a)).open_push(props)
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
