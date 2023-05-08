use crate::{
    target,
    filter,
    ctxt,
    time,
};

#[cfg(feature = "std")]
use std::sync::{Arc, OnceLock};

#[cfg(not(feature = "std"))]
struct StaticCell<T>(T);

#[cfg(not(feature = "std"))]
impl<T> StaticCell<T> {
    fn get(&self) -> &T {
        &self.0
    }
}

struct Ambient<Target = target::Discard, Filter = filter::Always, Ctxt = ctxt::Discard, Time = time::Unsupported> {
    target: Target,
    filter: Filter,
    ctxt: Ctxt,
    time: Option<Time>,
}

impl<Target: target::Target, Filter, Ctxt, Time> target::Target for Ambient<Target, Filter, Ctxt, Time> {
    fn emit_event<P: crate::Props>(&self, evt: &crate::Event<P>) {
        todo!()
    }
}

impl<Target, Filter: filter::Filter, Ctxt, Time> filter::Filter for Ambient<Target, Filter, Ctxt, Time> {
    fn matches_event<P: crate::Props>(&self, evt: &crate::Event<P>) -> bool {
        todo!()
    }
}

impl<Target, Filter, Ctxt: ctxt::PropsCtxt, Time> ctxt::PropsCtxt for Ambient<Target, Filter, Ctxt, Time> {
    type Props = Ctxt::Props;

    fn with_props<F: FnOnce(&Self::Props)>(&self, with: F) {
        todo!()
    }
}
