use std::{
    sync::{Arc, Condvar, Mutex},
    time::Duration,
};

use crate::{Channel, Sender};

#[derive(Clone)]
struct Trigger(Arc<(Mutex<bool>, Condvar)>);

impl Trigger {
    pub fn new() -> Self {
        Trigger(Arc::new((Mutex::new(false), Condvar::new())))
    }

    pub fn trigger(self) {
        *(self.0).0.lock().unwrap() = true;
        (self.0).1.notify_all();
    }

    pub fn wait_timeout(&self, timeout: Duration) {
        let mut flushed = (self.0).0.lock().unwrap();
        while !*flushed {
            match (self.0).1.wait_timeout(flushed, timeout).unwrap() {
                (next_flushed, r) if !r.timed_out() => {
                    flushed = next_flushed;
                    continue;
                }
                _ => return,
            }
        }
    }
}

pub fn blocking_flush<T: Channel>(sender: &Sender<T>, timeout: Duration) {
    let on_flush = Trigger::new();

    sender.on_next_flush({
        let on_flush = on_flush.clone();

        move || {
            on_flush.trigger();
        }
    });

    on_flush.wait_timeout(timeout);
}
