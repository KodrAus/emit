use crate::{empty::Empty, id::Id, props::Props, template::Template, time::Timestamp};

pub trait Ctxt {
    type Props: Props + ?Sized;
    type Span;

    fn open<P: Props>(&self, ts: Option<Timestamp>, id: Id, tpl: Template, props: P) -> Self::Span;

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

    fn close(&self, ts: Option<Timestamp>, span: Self::Span);

    fn by_ref(&self) -> ByRef<Self> {
        ByRef(self)
    }
}

impl<'a, C: Ctxt + ?Sized> Ctxt for &'a C {
    type Props = C::Props;
    type Span = C::Span;

    fn open<P: Props>(&self, ts: Option<Timestamp>, id: Id, tpl: Template, props: P) -> Self::Span {
        (**self).open(ts, id, tpl, props)
    }

    fn enter(&self, span: &mut Self::Span) {
        (**self).enter(span)
    }

    fn with_current<F: FnOnce(Id, &Self::Props)>(&self, with: F) {
        (**self).with_current(with)
    }

    fn current_id(&self) -> Id {
        (**self).current_id()
    }

    fn exit(&self, span: &mut Self::Span) {
        (**self).exit(span)
    }

    fn close(&self, ts: Option<Timestamp>, scope: Self::Span) {
        (**self).close(ts, scope)
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
            .unwrap_or_default()
    }

    fn open<P: Props>(&self, ts: Option<Timestamp>, id: Id, tpl: Template, props: P) -> Self::Span {
        self.as_ref().map(|ctxt| ctxt.open(ts, id, tpl, props))
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

    fn close(&self, ts: Option<Timestamp>, span: Self::Span) {
        if let (Some(ctxt), Some(span)) = (self, span) {
            ctxt.close(ts, span)
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

    fn open<P: Props>(&self, ts: Option<Timestamp>, id: Id, tpl: Template, props: P) -> Self::Span {
        (**self).open(ts, id, tpl, props)
    }

    fn enter(&self, span: &mut Self::Span) {
        (**self).enter(span)
    }

    fn exit(&self, span: &mut Self::Span) {
        (**self).exit(span)
    }

    fn close(&self, ts: Option<Timestamp>, span: Self::Span) {
        (**self).close(ts, span)
    }
}

pub struct ByRef<'a, T: ?Sized>(&'a T);

impl<'a, T: Ctxt + ?Sized> Ctxt for ByRef<'a, T> {
    type Props = T::Props;

    type Span = T::Span;

    fn open<P: Props>(&self, ts: Option<Timestamp>, id: Id, tpl: Template, props: P) -> Self::Span {
        self.0.open(ts, id, tpl, props)
    }

    fn enter(&self, span: &mut Self::Span) {
        self.0.enter(span)
    }

    fn with_current<F: FnOnce(Id, &Self::Props)>(&self, with: F) {
        self.0.with_current(with)
    }

    fn exit(&self, span: &mut Self::Span) {
        self.0.exit(span)
    }

    fn close(&self, ts: Option<Timestamp>, span: Self::Span) {
        self.0.close(ts, span)
    }

    fn current_id(&self) -> Id {
        self.0.current_id()
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

    fn open<P: Props>(&self, _: Option<Timestamp>, _: Id, _: Template, _: P) -> Self::Span {
        Empty
    }

    fn enter(&self, _: &mut Self::Span) {}

    fn exit(&self, _: &mut Self::Span) {}

    fn close(&self, _: Option<Timestamp>, _: Self::Span) {}
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

        use crate::{
            id::Id,
            key::Key,
            props::{ErasedProps, Props},
            template::Template,
            time::Timestamp,
            value::Value,
        };

        use super::ErasedScope;

        pub trait DispatchCtxt {
            fn dispatch_with_current(&self, with: &mut dyn FnMut(Id, ErasedSlot));
            fn dispatch_current_id(&self) -> Id;

            fn dispatch_open(
                &self,
                ts: Option<Timestamp>,
                id: Id,
                tpl: Template,
                props: &dyn ErasedProps,
            ) -> ErasedScope;
            fn dispatch_enter(&self, span: &mut ErasedScope);
            fn dispatch_exit(&self, span: &mut ErasedScope);
            fn dispatch_close(&self, ts: Option<Timestamp>, span: ErasedScope);
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

        fn dispatch_open(
            &self,
            ts: Option<Timestamp>,
            id: Id,
            tpl: Template,
            props: &dyn ErasedProps,
        ) -> ErasedScope {
            ErasedScope(Box::new(self.open(ts, id, tpl, props)))
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

        fn dispatch_close(&self, ts: Option<Timestamp>, span: ErasedScope) {
            if let Ok(span) = span.0.downcast() {
                self.close(ts, *span)
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

        fn open<P: Props>(
            &self,
            ts: Option<Timestamp>,
            id: Id,
            tpl: Template,
            props: P,
        ) -> Self::Span {
            self.erase_ctxt().0.dispatch_open(ts, id, tpl, &props)
        }

        fn enter(&self, span: &mut Self::Span) {
            self.erase_ctxt().0.dispatch_enter(span)
        }

        fn exit(&self, span: &mut Self::Span) {
            self.erase_ctxt().0.dispatch_exit(span)
        }

        fn close(&self, ts: Option<Timestamp>, span: Self::Span) {
            self.erase_ctxt().0.dispatch_close(ts, span)
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

        fn open<P: Props>(
            &self,
            ts: Option<Timestamp>,
            id: Id,
            tpl: Template,
            props: P,
        ) -> Self::Span {
            (self as &(dyn ErasedCtxt + 'a)).open(ts, id, tpl, props)
        }

        fn enter(&self, span: &mut Self::Span) {
            (self as &(dyn ErasedCtxt + 'a)).enter(span)
        }

        fn exit(&self, span: &mut Self::Span) {
            (self as &(dyn ErasedCtxt + 'a)).exit(span)
        }

        fn close(&self, ts: Option<Timestamp>, span: Self::Span) {
            (self as &(dyn ErasedCtxt + 'a)).close(ts, span)
        }
    }
}

#[cfg(feature = "alloc")]
pub use alloc_support::*;
