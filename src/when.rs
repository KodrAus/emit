use crate::{Event, Props};

pub trait When {
    fn emit_when<P: Props>(&self, evt: &Event<P>) -> bool;

    fn chain<U: When>(self, other: U) -> Chain<Self, U>
    where
        Self: Sized,
    {
        Chain {
            first: self,
            second: other,
        }
    }

    fn by_ref(&self) -> ByRef<Self> {
        ByRef(self)
    }
}

impl<F: Fn(&Event) -> bool> When for F {
    fn emit_when<P: Props>(&self, evt: &Event<P>) -> bool {
        (self)(&Event {
            head: evt.head.by_ref(),
            props: &evt.props,
        })
    }
}

impl<T: When, U: When> When for Chain<T, U> {
    fn emit_when<P: Props>(&self, evt: &Event<P>) -> bool {
        self.first.emit_when(evt) && self.second.emit_when(evt)
    }
}

impl<'a, C: When + ?Sized> When for ByRef<'a, C> {
    fn emit_when<P: Props>(&self, evt: &Event<P>) -> bool {
        self.0.emit_when(evt)
    }
}

pub fn default() -> impl When {
    struct Always;

    impl When for Always {
        fn emit_when<P: Props>(&self, _: &Event<P>) -> bool {
            true
        }
    }

    Always
}

pub struct Chain<T, U> {
    pub(crate) first: T,
    pub(crate) second: U,
}

pub struct ByRef<'a, T: ?Sized>(pub(crate) &'a T);

mod internal {
    use crate::Event;

    pub trait DispatchWhen {
        fn dispatch_emit_when(&self, evt: &Event) -> bool;
    }

    pub trait SealedWhen {
        fn erase_when(&self) -> crate::internal::Erased<&dyn DispatchWhen>;
    }
}

pub trait ErasedWhen: internal::SealedWhen {}

impl<T: When> ErasedWhen for T {}

impl<T: When> internal::SealedWhen for T {
    fn erase_when(&self) -> crate::internal::Erased<&dyn internal::DispatchWhen> {
        crate::internal::Erased(self)
    }
}

impl<T: When> internal::DispatchWhen for T {
    fn dispatch_emit_when(&self, evt: &Event) -> bool {
        self.emit_when(evt)
    }
}

impl<'a> When for dyn ErasedWhen + 'a {
    fn emit_when<P: Props>(&self, evt: &Event<P>) -> bool {
        self.erase_when().0.dispatch_emit_when(&evt.erase())
    }
}
