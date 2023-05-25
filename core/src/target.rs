use crate::{empty::Empty, event::Event, props::Props};

pub trait Target {
    fn emit_event<P: Props>(&self, evt: &Event<P>);
}

impl<'a, T: Target + ?Sized> Target for &'a T {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        (**self).emit_event(evt)
    }
}

#[cfg(feature = "std")]
impl<'a, T: Target + ?Sized + 'a> Target for Box<T> {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        (**self).emit_event(evt)
    }
}

impl<T: Target> Target for Option<T> {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        match self {
            Some(target) => target.emit_event(evt),
            None => Empty.emit_event(evt),
        }
    }
}

impl Target for Empty {
    fn emit_event<P: Props>(&self, _: &Event<P>) {}
}

impl Target for fn(&Event) {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        (self)(&evt.erase())
    }
}

pub struct FromFn<F>(F);

impl<F: Fn(&Event)> Target for FromFn<F> {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        (self.0)(&evt.erase())
    }
}

pub fn from_fn<F: Fn(&Event)>(f: F) -> FromFn<F> {
    FromFn(f)
}

mod internal {
    use crate::event::Event;

    pub trait DispatchTarget {
        fn dispatch_emit_to(&self, evt: &Event);
    }

    pub trait SealedTarget {
        fn erase_to(&self) -> crate::internal::Erased<&dyn DispatchTarget>;
    }
}

pub trait ErasedTarget: internal::SealedTarget {}

impl<T: Target> ErasedTarget for T {}

impl<T: Target> internal::SealedTarget for T {
    fn erase_to(&self) -> crate::internal::Erased<&dyn internal::DispatchTarget> {
        crate::internal::Erased(self)
    }
}

impl<T: Target> internal::DispatchTarget for T {
    fn dispatch_emit_to(&self, evt: &Event) {
        self.emit_event(evt)
    }
}

impl<'a> Target for dyn ErasedTarget + 'a {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        self.erase_to().0.dispatch_emit_to(&evt.erase())
    }
}

impl<'a> Target for dyn ErasedTarget + Send + Sync + 'a {
    fn emit_event<P: Props>(&self, evt: &Event<P>) {
        (self as &(dyn ErasedTarget + 'a)).emit_event(evt)
    }
}

macro_rules! tuple {
    ($(
        $len:expr => ( $(self.$i:tt: $ty:ident,)+ ),
    )+) => {
        $(
            impl<$($ty: Target),+> Target for ($($ty,)+) {
                fn emit_event<P: Props>(&self, evt: &Event<P>) {
                    $(
                        self.$i.emit_event(evt);
                    )+
                }
            }
        )+
    }
}

tuple! {
    1 => (
        self.0: T0,
    ),
    2 => (
        self.0: T0,
        self.1: T1,
    ),
    3 => (
        self.0: T0,
        self.1: T1,
        self.2: T2,
    ),
    4 => (
        self.0: T0,
        self.1: T1,
        self.2: T2,
        self.3: T3,
    ),
    5 => (
        self.0: T0,
        self.1: T1,
        self.2: T2,
        self.3: T3,
        self.4: T4,
    ),
    6 => (
        self.0: T0,
        self.1: T1,
        self.2: T2,
        self.3: T3,
        self.4: T4,
        self.5: T5,
    ),
    7 => (
        self.0: T0,
        self.1: T1,
        self.2: T2,
        self.3: T3,
        self.4: T4,
        self.5: T5,
        self.6: T6,
    ),
    8 => (
        self.0: T0,
        self.1: T1,
        self.2: T2,
        self.3: T3,
        self.4: T4,
        self.5: T5,
        self.6: T6,
        self.7: T7,
    ),
    9 => (
        self.0: T0,
        self.1: T1,
        self.2: T2,
        self.3: T3,
        self.4: T4,
        self.5: T5,
        self.6: T6,
        self.7: T7,
        self.8: T8,
    ),
    10 => (
        self.0: T0,
        self.1: T1,
        self.2: T2,
        self.3: T3,
        self.4: T4,
        self.5: T5,
        self.6: T6,
        self.7: T7,
        self.8: T8,
        self.9: T9,
    ),
    11 => (
        self.0: T0,
        self.1: T1,
        self.2: T2,
        self.3: T3,
        self.4: T4,
        self.5: T5,
        self.6: T6,
        self.7: T7,
        self.8: T8,
        self.9: T9,
        self.10: T10,
    ),
    12 => (
        self.0: T0,
        self.1: T1,
        self.2: T2,
        self.3: T3,
        self.4: T4,
        self.5: T5,
        self.6: T6,
        self.7: T7,
        self.8: T8,
        self.9: T9,
        self.10: T10,
        self.11: T11,
    ),
    13 => (
        self.0: T0,
        self.1: T1,
        self.2: T2,
        self.3: T3,
        self.4: T4,
        self.5: T5,
        self.6: T6,
        self.7: T7,
        self.8: T8,
        self.9: T9,
        self.10: T10,
        self.11: T11,
        self.12: T12,
    ),
    14 => (
        self.0: T0,
        self.1: T1,
        self.2: T2,
        self.3: T3,
        self.4: T4,
        self.5: T5,
        self.6: T6,
        self.7: T7,
        self.8: T8,
        self.9: T9,
        self.10: T10,
        self.11: T11,
        self.12: T12,
        self.13: T13,
    ),
    15 => (
        self.0: T0,
        self.1: T1,
        self.2: T2,
        self.3: T3,
        self.4: T4,
        self.5: T5,
        self.6: T6,
        self.7: T7,
        self.8: T8,
        self.9: T9,
        self.10: T10,
        self.11: T11,
        self.12: T12,
        self.13: T13,
        self.14: T14,
    ),
    16 => (
        self.0: T0,
        self.1: T1,
        self.2: T2,
        self.3: T3,
        self.4: T4,
        self.5: T5,
        self.6: T6,
        self.7: T7,
        self.8: T8,
        self.9: T9,
        self.10: T10,
        self.11: T11,
        self.12: T12,
        self.13: T13,
        self.14: T14,
        self.15: T15,
    ),
}
