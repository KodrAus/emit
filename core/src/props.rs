use core::{borrow::Borrow, ops::ControlFlow};

use crate::{
    and::And,
    empty::Empty,
    str::{Str, ToStr},
    value::{FromValue, ToValue, Value},
};

pub trait Props {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        for_each: F,
    ) -> ControlFlow<()>;

    fn get<'v, K: ToStr>(&'v self, key: K) -> Option<Value<'v>> {
        let key = key.to_str();
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

    fn pull<'kv, V: FromValue<'kv>, K: ToStr>(&'kv self, key: K) -> Option<V> {
        self.get(key).and_then(|v| v.cast())
    }

    fn count(&self) -> usize {
        let mut count = 0;

        self.for_each(|_, _| {
            count += 1;

            ControlFlow::Continue(())
        });

        count
    }

    fn is_unique(&self) -> bool {
        false
    }

    fn is_sorted(&self) -> bool {
        false
    }

    fn and_props<U: Props>(self, other: U) -> And<Self, U>
    where
        Self: Sized,
    {
        And::new(self, other)
    }

    fn filter<F: Fn(Str, Value) -> bool>(self, filter: F) -> Filter<Self, F>
    where
        Self: Sized,
    {
        Filter { src: self, filter }
    }

    #[cfg(feature = "alloc")]
    fn dedup(self) -> Dedup<Self>
    where
        Self: Sized,
    {
        Dedup { src: self }
    }
}

impl<'a, P: Props + ?Sized> Props for &'a P {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        for_each: F,
    ) -> ControlFlow<()> {
        (**self).for_each(for_each)
    }

    fn get<'v, K: ToStr>(&'v self, key: K) -> Option<Value<'v>> {
        (**self).get(key)
    }

    fn count(&self) -> usize {
        (**self).count()
    }

    fn pull<'kv, V: FromValue<'kv>, K: ToStr>(&'kv self, key: K) -> Option<V> {
        (**self).pull(key)
    }
}

impl<P: Props> Props for Option<P> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        for_each: F,
    ) -> ControlFlow<()> {
        match self {
            Some(props) => props.for_each(for_each),
            None => ControlFlow::Continue(()),
        }
    }
}

#[cfg(feature = "alloc")]
impl<'a, P: Props + ?Sized + 'a> Props for alloc::boxed::Box<P> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        for_each: F,
    ) -> ControlFlow<()> {
        (**self).for_each(for_each)
    }
}

#[cfg(feature = "alloc")]
impl<'a, P: Props + ?Sized + 'a> Props for alloc::sync::Arc<P> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        for_each: F,
    ) -> ControlFlow<()> {
        (**self).for_each(for_each)
    }
}

impl<K: ToStr, V: ToValue> Props for (K, V) {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for_each(self.0.to_str(), self.1.to_value())
    }

    fn count(&self) -> usize {
        1
    }

    fn is_sorted(&self) -> bool {
        true
    }

    fn is_unique(&self) -> bool {
        true
    }
}

impl<P: Props> Props for [P] {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for p in self {
            p.for_each(&mut for_each)?;
        }

        ControlFlow::Continue(())
    }

    fn count(&self) -> usize {
        self.iter().map(|props| props.count()).sum()
    }
}

impl<T, const N: usize> Props for [T; N]
where
    [T]: Props,
{
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        for_each: F,
    ) -> ControlFlow<()> {
        (self as &[_]).for_each(for_each)
    }

    fn count(&self) -> usize {
        (self as &[_]).count()
    }
}

#[cfg(feature = "alloc")]
impl<K, V> Props for alloc::collections::BTreeMap<K, V>
where
    K: Ord + ToStr + Borrow<str>,
    V: ToValue,
{
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for (k, v) in self {
            for_each(k.to_str(), v.to_value())?;
        }

        ControlFlow::Continue(())
    }

    fn get<'v, Q: ToStr>(&'v self, key: Q) -> Option<Value<'v>> {
        self.get(key.to_str().as_ref()).map(|v| v.to_value())
    }

    fn count(&self) -> usize {
        self.len()
    }

    fn is_unique(&self) -> bool {
        true
    }

    fn is_sorted(&self) -> bool {
        true
    }
}

#[cfg(feature = "std")]
impl<K, V> Props for std::collections::HashMap<K, V>
where
    K: Eq + std::hash::Hash + ToStr + Borrow<str>,
    V: ToValue,
{
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for (k, v) in self {
            for_each(k.to_str(), v.to_value())?;
        }

        ControlFlow::Continue(())
    }

    fn get<'v, Q: ToStr>(&'v self, key: Q) -> Option<Value<'v>> {
        self.get(key.to_str().as_ref()).map(|v| v.to_value())
    }

    fn count(&self) -> usize {
        self.len()
    }

    fn is_unique(&self) -> bool {
        true
    }
}

impl Props for Empty {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        _: F,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }

    fn count(&self) -> usize {
        0
    }
}

impl<A: Props, B: Props> Props for And<A, B> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        self.left().for_each(&mut for_each)?;
        self.right().for_each(for_each)
    }

    fn get<'v, K: ToStr>(&'v self, key: K) -> Option<Value<'v>> {
        let key = key.borrow();

        self.left().get(key).or_else(|| self.right().get(key))
    }

    fn count(&self) -> usize {
        self.left().count() + self.right().count()
    }

    fn is_sorted(&self) -> bool {
        self.left().is_sorted() && self.right().is_sorted()
    }
}

pub struct Filter<T, F> {
    src: T,
    filter: F,
}

impl<T: Props, F: Fn(Str, Value) -> bool> Props for Filter<T, F> {
    fn for_each<'kv, FE: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: FE,
    ) -> ControlFlow<()> {
        self.src.for_each(|k, v| {
            if (self.filter)(k.by_ref(), v.by_ref()) {
                for_each(k, v)
            } else {
                ControlFlow::Continue(())
            }
        })
    }

    fn get<'v, K: ToStr>(&'v self, key: K) -> Option<Value<'v>> {
        let key = key.to_str();

        match self.src.get(key.by_ref()) {
            Some(value) if (self.filter)(key, value.by_ref()) => Some(value),
            _ => None,
        }
    }
}

#[cfg(feature = "alloc")]
mod alloc_support {
    use super::*;

    pub struct Dedup<P> {
        pub(super) src: P,
    }

    impl<P: Props> Props for Dedup<P> {
        fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
            &'kv self,
            mut for_each: F,
        ) -> ControlFlow<()> {
            if self.src.is_unique() {
                return self.src.for_each(for_each);
            }

            let mut seen = alloc::collections::BTreeMap::new();

            self.src.for_each(|key, value| {
                seen.entry(key).or_insert(value);

                ControlFlow::Continue(())
            });

            for (key, value) in seen {
                for_each(key, value)?;
            }

            ControlFlow::Continue(())
        }

        fn get<'v, K: ToStr>(&'v self, key: K) -> Option<Value<'v>> {
            self.src.get(key)
        }

        fn is_unique(&self) -> bool {
            true
        }

        fn is_sorted(&self) -> bool {
            true
        }
    }
}

#[cfg(feature = "alloc")]
pub use alloc_support::*;

mod internal {
    use core::ops::ControlFlow;

    use crate::{str::Str, value::Value};

    pub trait DispatchProps {
        fn dispatch_for_each<'kv, 'f>(
            &'kv self,
            for_each: &'f mut dyn FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>,
        ) -> ControlFlow<()>;

        fn dispatch_get(&self, key: Str) -> Option<Value>;

        fn dispatch_count(&self) -> usize;

        fn dispatch_is_unique(&self) -> bool;

        fn dispatch_is_sorted(&self) -> bool;
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
        for_each: &'f mut dyn FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>,
    ) -> ControlFlow<()> {
        self.for_each(for_each)
    }

    fn dispatch_get<'v>(&'v self, key: Str) -> Option<Value<'v>> {
        self.get(key)
    }

    fn dispatch_count(&self) -> usize {
        self.count()
    }

    fn dispatch_is_sorted(&self) -> bool {
        self.is_sorted()
    }

    fn dispatch_is_unique(&self) -> bool {
        self.is_unique()
    }
}

impl<'a> Props for dyn ErasedProps + 'a {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        self.erase_props().0.dispatch_for_each(&mut for_each)
    }

    fn get<'v, K: ToStr>(&'v self, key: K) -> Option<Value<'v>> {
        self.erase_props().0.dispatch_get(key.to_str())
    }

    fn count(&self) -> usize {
        self.erase_props().0.dispatch_count()
    }

    fn is_sorted(&self) -> bool {
        self.erase_props().0.dispatch_is_sorted()
    }

    fn is_unique(&self) -> bool {
        self.erase_props().0.dispatch_is_unique()
    }
}
