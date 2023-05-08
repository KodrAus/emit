use crate::{Key, Value};

use core::{borrow::Borrow, ops::ControlFlow};

pub use crate::adapt::{ByRef, Chain, Empty};

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

impl Props for Empty {
    fn for_each<'a, F: FnMut(Key<'a>, Value<'a>) -> ControlFlow<()>>(&'a self, _: F) {}
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

#[repr(transparent)]
pub struct SortedSlice<'a>([(Key<'a>, Value<'a>)]);

impl SortedSlice<'static> {
    pub fn new(props: &'static [(Key<'static>, Value<'static>)]) -> &'static Self {
        Self::new_ref(props)
    }
}

impl<'a> SortedSlice<'a> {
    pub fn new_ref<'b>(props: &'b [(Key<'a>, Value<'a>)]) -> &'b Self {
        unsafe { &*(props as *const [(Key<'a>, Value<'a>)] as *const SortedSlice<'a>) }
    }

    pub fn by_ref<'b>(&'b self) -> ByRef<'b, Self> {
        ByRef(self)
    }

    pub fn chain<'b, U>(&'b self, other: U) -> Chain<&'b Self, U> {
        Chain {
            first: self,
            second: other,
        }
    }
}

impl<'a> Props for SortedSlice<'a> {
    fn for_each<'v, F: FnMut(Key<'v>, Value<'v>) -> ControlFlow<()>>(&'v self, for_each: F) {
        self.0.for_each(for_each)
    }

    fn get<'v, K: Borrow<str>>(&'v self, key: K) -> Option<Value<'v>> {
        let key = Key::new_ref(key.borrow());

        self.0
            .binary_search_by(|(k, _)| k.cmp(&key))
            .ok()
            .map(|i| self.0[i].1.by_ref())
    }
}

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
