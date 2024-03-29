use emit_core::{rng::Rng, runtime::InternalRng};
use rand::Rng as _;

#[derive(Default, Debug, Clone, Copy)]
pub struct ThreadRng;

impl Rng for ThreadRng {
    fn gen_u64(&self) -> Option<u64> {
        Some(rand::thread_rng().gen())
    }
}

impl InternalRng for ThreadRng {}
