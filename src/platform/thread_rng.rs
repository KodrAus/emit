use emit_core::{rng::Rng, runtime::InternalRng};
use rand::{Rng as _, RngCore};

#[derive(Default, Debug, Clone, Copy)]
pub struct ThreadRng;

impl Rng for ThreadRng {
    fn fill<A: AsMut<[u8]>>(&self, mut arr: A) -> Option<A> {
        rand::thread_rng().fill_bytes(arr.as_mut());

        Some(arr)
    }

    fn gen_u64(&self) -> Option<u64> {
        Some(rand::thread_rng().gen())
    }

    fn gen_u128(&self) -> Option<u128> {
        Some(rand::thread_rng().gen())
    }
}

impl InternalRng for ThreadRng {}
