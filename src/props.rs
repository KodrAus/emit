use crate::{Key, Value};

use core::{borrow::Borrow, ops::ControlFlow};

pub trait Props {
    fn for_each<'a, F: FnMut(Key<'a>, Value<'a>) -> ControlFlow<()>>(&'a self, for_each: F);

    fn get<'v, K: Borrow<str>>(&'v self, key: K) -> Option<Value<'v>> {
        let key = key.borrow();
        let mut value = None;

        self.for_each(|k, v| {
            if k == key {
                value = Some(v);

                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });

        value
    }

    fn chain<U: Props>(self, other: U) -> Chain<Self, U>
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

impl<'a, P: Props + ?Sized> Props for &'a P {
    fn for_each<'v, F: FnMut(Key<'v>, Value<'v>) -> ControlFlow<()>>(&'v self, for_each: F) {
        (**self).for_each(for_each)
    }

    fn get<'v, K: Borrow<str>>(&'v self, key: K) -> Option<Value<'v>> {
        (**self).get(key)
    }
}

impl<P: Props> Props for Option<P> {
    fn for_each<'a, F: FnMut(Key<'a>, Value<'a>) -> ControlFlow<()>>(&'a self, for_each: F) {
        match self {
            Some(props) => props.for_each(for_each),
            None => (),
        }
    }
}

#[cfg(feature = "std")]
impl<'a, P: Props + ?Sized + 'a> Props for Box<P> {
    fn for_each<'v, F: FnMut(Key<'v>, Value<'v>) -> ControlFlow<()>>(&'v self, for_each: F) {
        (**self).for_each(for_each)
    }
}

impl<'k, 'v> Props for [(Key<'k>, Value<'v>)] {
    fn for_each<'a, F: FnMut(Key<'a>, Value<'a>) -> ControlFlow<()>>(&'a self, mut for_each: F) {
        for (k, v) in self {
            match for_each(k.by_ref(), v.by_ref()) {
                ControlFlow::Continue(()) => continue,
                ControlFlow::Break(()) => return,
            }
        }
    }
}

impl<'k, 'v, const N: usize> Props for [(Key<'k>, Value<'v>); N] {
    fn for_each<'a, F: FnMut(Key<'a>, Value<'a>) -> ControlFlow<()>>(&'a self, for_each: F) {
        (self as &[_]).for_each(for_each)
    }
}

impl<A: Props, B: Props> Props for Chain<A, B> {
    fn for_each<'a, F: FnMut(Key<'a>, Value<'a>) -> ControlFlow<()>>(&'a self, mut for_each: F) {
        let mut cf = ControlFlow::Continue(());

        self.first.for_each(|k, v| match for_each(k, v) {
            ControlFlow::Continue(()) => ControlFlow::Continue(()),
            ControlFlow::Break(r) => {
                cf = ControlFlow::Break(());
                ControlFlow::Break(r)
            }
        });

        if let ControlFlow::Break(()) = cf {
            return;
        }

        self.second.for_each(for_each)
    }

    fn get<'v, K: Borrow<str>>(&'v self, key: K) -> Option<Value<'v>> {
        let key = key.borrow();

        self.first.get(key).or_else(|| self.second.get(key))
    }
}

impl<'a, P: Props + ?Sized> Props for ByRef<'a, P> {
    fn for_each<'v, F: FnMut(Key<'v>, Value<'v>) -> ControlFlow<()>>(&'v self, for_each: F) {
        self.0.for_each(for_each)
    }
}

pub(crate) struct Empty;

impl Props for Empty {
    fn for_each<'a, F: FnMut(Key<'a>, Value<'a>) -> ControlFlow<()>>(&'a self, _: F) {}
}

pub struct Chain<T, U> {
    pub(crate) first: T,
    pub(crate) second: U,
}

pub struct ByRef<'a, T: ?Sized>(pub(crate) &'a T);

mod internal {
    use core::ops::ControlFlow;

    use crate::{Key, Value};

    pub trait DispatchProps {
        fn dispatch_for_each<'a, 'b>(
            &'a self,
            for_each: &'b mut dyn FnMut(Key<'a>, Value<'a>) -> ControlFlow<()>,
        );
        fn dispatch_get<'a>(&'a self, key: &str) -> Option<Value<'a>>;
    }

    pub trait SealedProps {
        fn erase_props(&self) -> crate::internal::Erased<&dyn DispatchProps>;
    }
}

pub trait ErasedProps: internal::SealedProps {}

impl<P: Props> ErasedProps for P {}

impl<P: Props> internal::SealedProps for P {
    fn erase_props(&self) -> crate::internal::Erased<&dyn internal::DispatchProps> {
        crate::internal::Erased(self)
    }
}

impl<P: Props> internal::DispatchProps for P {
    fn dispatch_for_each<'a, 'b>(
        &'a self,
        for_each: &'b mut dyn FnMut(Key<'a>, Value<'a>) -> ControlFlow<()>,
    ) {
        self.for_each(for_each)
    }

    fn dispatch_get<'a>(&'a self, key: &str) -> Option<Value<'a>> {
        self.get(key)
    }
}

impl<'a> Props for dyn ErasedProps + 'a {
    fn for_each<'v, F: FnMut(Key<'v>, Value<'v>) -> ControlFlow<()>>(&'v self, mut for_each: F) {
        self.erase_props().0.dispatch_for_each(&mut for_each)
    }

    fn get<'v, K: Borrow<str>>(&'v self, key: K) -> Option<Value<'v>> {
        self.erase_props().0.dispatch_get(key.borrow())
    }
}
