use crate::value::Val;
use crate::well_known;

use std::borrow::Borrow;
use std::ops::ControlFlow;

pub trait Props {
    fn visit<'a, V: Visit<'a>>(&'a self, visitor: V);

    fn get<'v, K: Borrow<str>>(&'v self, key: K) -> Option<Val<'v>> {
        let key = key.borrow();
        let mut value = None;

        self.visit(|prop: Prop<'v>| {
            if prop.key == key {
                value = Some(prop.val);

                ControlFlow::Break(())
            } else {
                ControlFlow::Continue(())
            }
        });

        value
    }

    fn err<'v>(&'v self) -> Option<well_known::Err<'v>> {
        self.get(well_known::Err::KEY).map(well_known::Err::new)
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
    fn visit<'b, V: Visit<'b>>(&'b self, visitor: V) {
        (**self).visit(visitor)
    }

    fn get<'v, K: Borrow<str>>(&'v self, key: K) -> Option<Val<'v>> {
        (**self).get(key)
    }

    fn err<'v>(&'v self) -> Option<well_known::Err<'v>> {
        (**self).err()
    }
}

impl<A: Props, B: Props> Props for Chain<A, B> {
    fn visit<'a, V: Visit<'a>>(&'a self, mut visitor: V) {
        let mut cf = ControlFlow::Continue(());

        self.first
            .visit(|prop: Prop<'a>| match visitor.visit_property(prop) {
                ControlFlow::Continue(()) => ControlFlow::Continue(()),
                ControlFlow::Break(r) => {
                    cf = ControlFlow::Break(());
                    ControlFlow::Break(r)
                }
            });

        if let ControlFlow::Break(()) = cf {
            return;
        }

        self.second
            .visit(|prop: Prop<'a>| visitor.visit_property(prop))
    }

    fn get<'v, K: Borrow<str>>(&'v self, key: K) -> Option<Val<'v>> {
        let key = key.borrow();

        self.first.get(key).or_else(|| self.second.get(key))
    }
}

impl<'a, P: Props + ?Sized> Props for ByRef<'a, P> {
    fn visit<'b, V: Visit<'b>>(&'b self, visitor: V) {
        self.0.visit(visitor)
    }
}

pub struct Prop<'kv> {
    pub key: &'kv str,
    pub val: Val<'kv>,
}

impl<'kv> Prop<'kv> {
    pub fn new(key: &'kv str, val: Val<'kv>) -> Self {
        Prop { key, val }
    }

    pub fn err(&self) -> Option<&well_known::Err<'kv>> {
        if self.key == well_known::Err::KEY {
            Some(well_known::Err::new_ref(&self.val))
        } else {
            None
        }
    }
}

impl<'a> Props for [(&'a str, Val<'a>)] {
    fn visit<'b, V: Visit<'b>>(&'b self, mut visitor: V) {
        for (k, v) in self {
            match visitor.visit_property(Prop::new(k, v.by_ref())) {
                ControlFlow::Continue(()) => continue,
                ControlFlow::Break(()) => return,
            }
        }
    }
}

impl<'a, const N: usize> Props for [(&'a str, Val<'a>); N] {
    fn visit<'b, V: Visit<'b>>(&'b self, visitor: V) {
        (&*self as &[_]).visit(visitor)
    }
}

impl<P: Props> Props for Option<P> {
    fn visit<'a, V: Visit<'a>>(&'a self, visitor: V) {
        if let Some(props) = self {
            props.visit(visitor)
        }
    }
}

pub trait Visit<'a> {
    fn visit_property(&mut self, property: Prop<'a>) -> ControlFlow<()>;

    fn by_mut(&mut self) -> ByMut<Self> {
        ByMut(self)
    }
}

impl<'a, F: FnMut(Prop<'a>) -> ControlFlow<()>> Visit<'a> for F {
    fn visit_property(&mut self, property: Prop<'a>) -> ControlFlow<()> {
        (self)(property)
    }
}

impl<'a, V: Visit<'a> + ?Sized> Visit<'a> for ByMut<'a, V> {
    fn visit_property(&mut self, property: Prop<'a>) -> ControlFlow<()> {
        self.0.visit_property(property)
    }
}

pub struct ByMut<'a, T: ?Sized>(pub(crate) &'a mut T);

pub struct Chain<T, U> {
    pub(crate) first: T,
    pub(crate) second: U,
}

pub struct ByRef<'a, T: ?Sized>(pub(crate) &'a T);

mod internal {
    use std::ops::ControlFlow;

    use crate::{well_known, Prop, Val};

    use super::ErasedVisit;

    pub trait DispatchProps {
        fn dispatch_visit<'a, 'b>(&'a self, visitor: &'b mut dyn ErasedVisit<'a>);
        fn dispatch_get<'a>(&'a self, key: &str) -> Option<Val<'a>>;
        fn dispatch_err<'a>(&'a self) -> Option<well_known::Err<'a>>;
    }

    pub trait DispatchVisit<'a> {
        fn dispatch_visit_property(&mut self, property: Prop<'a>) -> ControlFlow<()>;
    }

    pub trait SealedProps {
        fn erase_props(&self) -> crate::internal::Erased<&dyn DispatchProps>;
    }

    pub trait SealedVisit<'a> {
        fn erase_visit(&mut self) -> crate::internal::Erased<&mut dyn DispatchVisit<'a>>;
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
    fn dispatch_visit<'a, 'b>(&'a self, visitor: &'b mut dyn ErasedVisit<'a>) {
        self.visit(|prop: Prop<'a>| visitor.visit_property(prop))
    }

    fn dispatch_get<'a>(&'a self, key: &str) -> Option<Val<'a>> {
        self.get(key)
    }

    fn dispatch_err<'a>(&'a self) -> Option<well_known::Err<'a>> {
        self.err()
    }
}

impl<'a> Props for dyn ErasedProps + 'a {
    fn visit<'v, V: Visit<'v>>(&'v self, mut visitor: V) {
        self.erase_props().0.dispatch_visit(&mut visitor)
    }

    fn get<'v, K: Borrow<str>>(&'v self, key: K) -> Option<Val<'v>> {
        self.erase_props().0.dispatch_get(key.borrow())
    }

    fn err<'v>(&'v self) -> Option<well_known::Err<'v>> {
        self.erase_props().0.dispatch_err()
    }
}

pub trait ErasedVisit<'a>: internal::SealedVisit<'a> {}

impl<'a, V: Visit<'a>> ErasedVisit<'a> for V {}

impl<'a, V: Visit<'a>> internal::SealedVisit<'a> for V {
    fn erase_visit(&mut self) -> crate::internal::Erased<&mut dyn internal::DispatchVisit<'a>> {
        crate::internal::Erased(self)
    }
}

impl<'a, V: Visit<'a>> internal::DispatchVisit<'a> for V {
    fn dispatch_visit_property(&mut self, property: Prop<'a>) -> ControlFlow<()> {
        self.visit_property(property)
    }
}

impl<'a, 'b> Visit<'a> for dyn ErasedVisit<'a> + 'b {
    fn visit_property(&mut self, property: Prop<'a>) -> ControlFlow<()> {
        self.erase_visit().0.dispatch_visit_property(property)
    }
}
