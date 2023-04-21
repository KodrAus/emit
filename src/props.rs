use crate::Key;
use crate::ToKey;
use crate::Val;

use core::ops::ControlFlow;

pub trait Props {
    fn for_each<'a, F: FnMut(Key<'a>, Val<'a>) -> ControlFlow<()>>(&'a self, for_each: F);

    fn get<'v, K: ToKey>(&'v self, key: K) -> Option<Val<'v>> {
        let key = key.to_key();
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
    fn for_each<'v, F: FnMut(Key<'v>, Val<'v>) -> ControlFlow<()>>(&'v self, for_each: F) {
        (**self).for_each(for_each)
    }

    fn get<'v, K: ToKey>(&'v self, key: K) -> Option<Val<'v>> {
        (**self).get(key)
    }
}

impl<A: Props, B: Props> Props for Chain<A, B> {
    fn for_each<'a, F: FnMut(Key<'a>, Val<'a>) -> ControlFlow<()>>(&'a self, mut for_each: F) {
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

    fn get<'v, K: ToKey>(&'v self, key: K) -> Option<Val<'v>> {
        self.first.get(&key).or_else(|| self.second.get(&key))
    }
}

impl<'a, P: Props + ?Sized> Props for ByRef<'a, P> {
    fn for_each<'v, F: FnMut(Key<'v>, Val<'v>) -> ControlFlow<()>>(&'v self, for_each: F) {
        self.0.for_each(for_each)
    }
}

pub(crate) struct Empty;

impl Props for Empty {
    fn for_each<'a, F: FnMut(Key<'a>, Val<'a>) -> ControlFlow<()>>(&'a self, for_each: F) {}
}

pub struct Chain<T, U> {
    pub(crate) first: T,
    pub(crate) second: U,
}

pub struct ByRef<'a, T: ?Sized>(pub(crate) &'a T);

mod internal {
    use core::ops::ControlFlow;

    use crate::{Key, Val};

    pub trait DispatchProps {
        fn dispatch_for_each<'a, 'b>(
            &'a self,
            for_each: &'b mut dyn FnMut(Key<'a>, Val<'a>) -> ControlFlow<()>,
        );
        fn dispatch_get<'a>(&'a self, key: Key) -> Option<Val<'a>>;
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
        for_each: &'b mut dyn FnMut(Key<'a>, Val<'a>) -> ControlFlow<()>,
    ) {
        self.for_each(for_each)
    }

    fn dispatch_get<'a>(&'a self, key: Key) -> Option<Val<'a>> {
        self.get(key)
    }
}

impl<'a> Props for dyn ErasedProps + 'a {
    fn for_each<'v, F: FnMut(Key<'v>, Val<'v>) -> ControlFlow<()>>(&'v self, mut for_each: F) {
        self.erase_props().0.dispatch_for_each(&mut for_each)
    }

    fn get<'v, K: ToKey>(&'v self, key: K) -> Option<Val<'v>> {
        self.erase_props().0.dispatch_get(key.to_key())
    }
}
