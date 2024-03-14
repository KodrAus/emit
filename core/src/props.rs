use core::{borrow::Borrow, ops::ControlFlow};

use crate::{
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

    fn chain<U: Props>(self, other: U) -> Chain<Self, U>
    where
        Self: Sized,
    {
        Chain {
            first: self,
            second: other,
        }
    }

    fn filter<F: Fn(Str, Value) -> bool>(self, filter: F) -> Filter<Self, F>
    where
        Self: Sized,
    {
        Filter { src: self, filter }
    }

    fn by_ref(&self) -> ByRef<Self> {
        ByRef(self)
    }

    fn pull<'kv, V: FromValue<'kv>, K: ToStr>(&'kv self, key: K) -> Option<V> {
        self.get(key).and_then(|v| v.cast())
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

impl<K: ToStr, V: ToValue> Props for (K, V) {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        for_each(self.0.to_str(), self.1.to_value())
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
}

impl Props for Empty {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        _: F,
    ) -> ControlFlow<()> {
        ControlFlow::Continue(())
    }
}

pub struct Chain<T, U> {
    first: T,
    second: U,
}

impl<T, U> Chain<T, U> {
    pub fn first(&self) -> &T {
        &self.first
    }

    pub fn second(&self) -> &U {
        &self.second
    }
}

impl<A: Props, B: Props> Props for Chain<A, B> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        mut for_each: F,
    ) -> ControlFlow<()> {
        self.first.for_each(&mut for_each)?;
        self.second.for_each(for_each)
    }

    fn get<'v, K: ToStr>(&'v self, key: K) -> Option<Value<'v>> {
        let key = key.borrow();

        self.first.get(key).or_else(|| self.second.get(key))
    }
}

pub struct ByRef<'a, T: ?Sized>(&'a T);

impl<'a, P: Props + ?Sized> Props for ByRef<'a, P> {
    fn for_each<'kv, F: FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>>(
        &'kv self,
        for_each: F,
    ) -> ControlFlow<()> {
        self.0.for_each(for_each)
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

mod internal {
    use core::ops::ControlFlow;

    use crate::{str::Str, value::Value};

    pub trait DispatchProps {
        fn dispatch_for_each<'kv, 'f>(
            &'kv self,
            for_each: &'f mut dyn FnMut(Str<'kv>, Value<'kv>) -> ControlFlow<()>,
        ) -> ControlFlow<()>;

        fn dispatch_get(&self, key: Str) -> Option<Value>;
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
}
