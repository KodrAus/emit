use crate::empty::Empty;

pub trait Rng {
    fn gen_u64(&self) -> Option<u64>;
}

impl<'a, T: Rng + ?Sized> Rng for &'a T {
    fn gen_u64(&self) -> Option<u64> {
        (**self).gen_u64()
    }
}

impl<'a, T: Rng> Rng for Option<T> {
    fn gen_u64(&self) -> Option<u64> {
        self.as_ref().and_then(|id| id.gen_u64())
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: Rng + ?Sized + 'a> Rng for alloc::boxed::Box<T> {
    fn gen_u64(&self) -> Option<u64> {
        (**self).gen_u64()
    }
}

impl Rng for Empty {
    fn gen_u64(&self) -> Option<u64> {
        None
    }
}

mod internal {
    pub trait DispatchRng {
        fn dispatch_gen_u64(&self) -> Option<u64>;
    }

    pub trait SealedRng {
        fn erase_rng(&self) -> crate::internal::Erased<&dyn DispatchRng>;
    }
}

pub trait ErasedRng: internal::SealedRng {}

impl<T: Rng> ErasedRng for T {}

impl<T: Rng> internal::SealedRng for T {
    fn erase_rng(&self) -> crate::internal::Erased<&dyn internal::DispatchRng> {
        crate::internal::Erased(self)
    }
}

impl<T: Rng> internal::DispatchRng for T {
    fn dispatch_gen_u64(&self) -> Option<u64> {
        self.gen_u64()
    }
}

impl<'a> Rng for dyn ErasedRng + 'a {
    fn gen_u64(&self) -> Option<u64> {
        self.erase_rng().0.dispatch_gen_u64()
    }
}

impl<'a> Rng for dyn ErasedRng + Send + Sync + 'a {
    fn gen_u64(&self) -> Option<u64> {
        (self as &(dyn ErasedRng + 'a)).gen_u64()
    }
}
