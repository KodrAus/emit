use crate::empty::Empty;

pub trait Rng {
    fn fill<A: AsMut<[u8]>>(&self, arr: A) -> Option<A>;

    fn gen_u64(&self) -> Option<u64> {
        self.fill([0; 8]).map(u64::from_le_bytes)
    }

    fn gen_u128(&self) -> Option<u128> {
        self.fill([0; 16]).map(u128::from_le_bytes)
    }
}

impl<'a, T: Rng + ?Sized> Rng for &'a T {
    fn fill<A: AsMut<[u8]>>(&self, arr: A) -> Option<A> {
        (**self).fill(arr)
    }

    fn gen_u64(&self) -> Option<u64> {
        (**self).gen_u64()
    }

    fn gen_u128(&self) -> Option<u128> {
        (**self).gen_u128()
    }
}

impl<'a, T: Rng> Rng for Option<T> {
    fn fill<A: AsMut<[u8]>>(&self, arr: A) -> Option<A> {
        self.as_ref().and_then(|id| id.fill(arr))
    }

    fn gen_u64(&self) -> Option<u64> {
        self.as_ref().and_then(|id| id.gen_u64())
    }

    fn gen_u128(&self) -> Option<u128> {
        self.as_ref().and_then(|id| id.gen_u128())
    }
}

#[cfg(feature = "alloc")]
impl<'a, T: Rng + ?Sized + 'a> Rng for alloc::boxed::Box<T> {
    fn fill<A: AsMut<[u8]>>(&self, arr: A) -> Option<A> {
        (**self).fill(arr)
    }

    fn gen_u64(&self) -> Option<u64> {
        (**self).gen_u64()
    }

    fn gen_u128(&self) -> Option<u128> {
        (**self).gen_u128()
    }
}

impl Rng for Empty {
    fn fill<A: AsMut<[u8]>>(&self, _: A) -> Option<A> {
        None
    }
}

mod internal {
    pub trait DispatchRng {
        fn dispatch_gen(&self, arr: &mut [u8]) -> bool;
        fn dispatch_gen_u64(&self) -> Option<u64>;
        fn dispatch_gen_u128(&self) -> Option<u128>;
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
    fn dispatch_gen(&self, arr: &mut [u8]) -> bool {
        self.fill(arr).is_some()
    }

    fn dispatch_gen_u64(&self) -> Option<u64> {
        self.gen_u64()
    }

    fn dispatch_gen_u128(&self) -> Option<u128> {
        self.gen_u128()
    }
}

impl<'a> Rng for dyn ErasedRng + 'a {
    fn fill<A: AsMut<[u8]>>(&self, mut arr: A) -> Option<A> {
        if self.erase_rng().0.dispatch_gen(arr.as_mut()) {
            Some(arr)
        } else {
            None
        }
    }

    fn gen_u64(&self) -> Option<u64> {
        self.erase_rng().0.dispatch_gen_u64()
    }

    fn gen_u128(&self) -> Option<u128> {
        self.erase_rng().0.dispatch_gen_u128()
    }
}

impl<'a> Rng for dyn ErasedRng + Send + Sync + 'a {
    fn fill<A: AsMut<[u8]>>(&self, arr: A) -> Option<A> {
        (self as &(dyn ErasedRng + 'a)).fill(arr)
    }

    fn gen_u64(&self) -> Option<u64> {
        (self as &(dyn ErasedRng + 'a)).gen_u64()
    }

    fn gen_u128(&self) -> Option<u128> {
        (self as &(dyn ErasedRng + 'a)).gen_u128()
    }
}
