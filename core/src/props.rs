use core::{borrow::Borrow, ops::ControlFlow};

use crate::{
    empty::Empty,
    key::{Key, ToKey},
    value::{ToValue, Value},
};

pub trait Props {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(&'kv self, for_each: F);

    fn get<'v, K: ToKey>(&'v self, key: K) -> Option<Value<'v>> {
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

    fn filter<F: Fn(Key, Value) -> bool>(self, filter: F) -> Filter<Self, F>
    where
        Self: Sized,
    {
        Filter { src: self, filter }
    }

    fn by_ref(&self) -> ByRef<Self> {
        ByRef(self)
    }
}

impl<'a, P: Props + ?Sized> Props for &'a P {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(&'kv self, for_each: F) {
        (**self).for_each(for_each)
    }

    fn get<'v, K: ToKey>(&'v self, key: K) -> Option<Value<'v>> {
        (**self).get(key)
    }
}

impl<P: Props> Props for Option<P> {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(&'kv self, for_each: F) {
        match self {
            Some(props) => props.for_each(for_each),
            None => (),
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a, P: Props + ?Sized + 'a> Props for alloc::boxed::Box<P> {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(&'kv self, for_each: F) {
        (**self).for_each(for_each)
    }
}

impl<K: ToKey, V: ToValue> Props for (K, V) {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) {
        for_each(self.0.to_key(), self.1.to_value());
    }
}

impl<K: ToKey, V: ToValue> Props for [(K, V)] {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) {
        for (k, v) in self {
            match for_each(k.to_key(), v.to_value()) {
                ControlFlow::Continue(()) => continue,
                ControlFlow::Break(()) => return,
            }
        }
    }
}

impl<K: ToKey, V: ToValue> Props for [Option<(K, V)>] {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) {
        for (k, v) in self.iter().filter_map(|kv| kv.as_ref()) {
            match for_each(k.to_key(), v.to_value()) {
                ControlFlow::Continue(()) => continue,
                ControlFlow::Break(()) => return,
            }
        }
    }
}

impl<T, const N: usize> Props for [T; N]
where
    [T]: Props,
{
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(&'kv self, for_each: F) {
        (self as &[_]).for_each(for_each)
    }
}

impl Props for Empty {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(&'kv self, _: F) {}
}

pub struct Chain<T, U> {
    first: T,
    second: U,
}

impl<A: Props, B: Props> Props for Chain<A, B> {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) {
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

    fn get<'v, K: ToKey>(&'v self, key: K) -> Option<Value<'v>> {
        let key = key.borrow();

        self.first.get(key).or_else(|| self.second.get(key))
    }
}

pub struct ByRef<'a, T: ?Sized>(&'a T);

impl<'a, P: Props + ?Sized> Props for ByRef<'a, P> {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(&'kv self, for_each: F) {
        self.0.for_each(for_each)
    }
}

pub struct Filter<T, F> {
    src: T,
    filter: F,
}

impl<T: Props, F: Fn(Key, Value) -> bool> Props for Filter<T, F> {
    fn for_each<'kv, FE: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: FE,
    ) {
        self.src.for_each(|k, v| {
            if (self.filter)(k.by_ref(), v.by_ref()) {
                for_each(k, v)
            } else {
                ControlFlow::Continue(())
            }
        })
    }

    fn get<'v, K: ToKey>(&'v self, key: K) -> Option<Value<'v>> {
        let key = key.to_key();

        match self.src.get(key.by_ref()) {
            Some(value) if (self.filter)(key, value.by_ref()) => Some(value),
            _ => None,
        }
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
}

impl<'a> Props for SortedSlice<'a> {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(&'kv self, for_each: F) {
        self.0.for_each(for_each)
    }

    fn get<'v, K: ToKey>(&'v self, key: K) -> Option<Value<'v>> {
        let key = key.to_key();

        self.0
            .binary_search_by(|(k, _)| k.cmp(&key))
            .ok()
            .map(|i| self.0[i].1.by_ref())
    }
}

mod internal {
    use core::ops::ControlFlow;

    use crate::{key::Key, value::Value};

    pub trait DispatchProps {
        fn dispatch_for_each<'kv, 'f>(
            &'kv self,
            for_each: &'f mut dyn FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>,
        );

        fn dispatch_get(&self, key: Key) -> Option<Value>;
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
    fn dispatch_for_each<'kv, 'f>(
        &'kv self,
        for_each: &'f mut dyn FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>,
    ) {
        self.for_each(for_each)
    }

    fn dispatch_get<'v>(&'v self, key: Key) -> Option<Value<'v>> {
        self.get(key)
    }
}

impl<'a> Props for dyn ErasedProps + 'a {
    fn for_each<'kv, F: FnMut(Key<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) {
        self.erase_props().0.dispatch_for_each(&mut for_each)
    }

    fn get<'v, K: ToKey>(&'v self, key: K) -> Option<Value<'v>> {
        self.erase_props().0.dispatch_get(key.to_key())
    }
}
