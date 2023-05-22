use std::{
    future::Future,
    mem,
    panic::{self, AssertUnwindSafe},
    sync::{Arc, Mutex},
    time::Duration,
};

use crate::Error;

pub struct Batcher<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Batcher<T> {
    pub fn new(max_capacity: usize) -> Self {
        Batcher {
            shared: Arc::new(Shared {
                max_capacity,
                state: Mutex::new(State {
                    batch: Batch::new(),
                    open: true,
                }),
            }),
        }
    }

    pub fn watch_next_flush(&self, watcher: impl FnOnce() + Send + 'static) {
        let watcher = Box::new(watcher);

        let mut state = self.shared.state.lock().unwrap();

        state.batch.watchers.push(watcher);
    }
}

impl<T> Clone for Batcher<T> {
    fn clone(&self) -> Self {
        Batcher {
            shared: self.shared.clone(),
        }
    }
}

struct Shared<T> {
    max_capacity: usize,
    state: Mutex<State<T>>,
}

struct State<T> {
    batch: Batch<T>,
    open: bool,
}

struct Batch<T> {
    contents: Vec<T>,
    watchers: Watchers,
}

struct Watchers(Vec<Watcher>);

type Watcher = Box<dyn FnOnce() + Send>;

impl Default for Watchers {
    fn default() -> Self {
        Watchers::new()
    }
}

impl Watchers {
    fn new() -> Self {
        Watchers(Vec::new())
    }

    fn push(&mut self, watcher: Watcher) {
        self.0.push(watcher);
    }

    fn notify(self) {
        for watcher in self.0 {
            let _ = panic::catch_unwind(AssertUnwindSafe(watcher));
        }
    }
}

impl<T> Batch<T> {
    fn new() -> Self {
        Batch {
            contents: Vec::new(),
            watchers: Watchers::new(),
        }
    }
}

impl<T> Default for Batch<T> {
    fn default() -> Self {
        Batch::new()
    }
}

pub struct Receiver<T> {
    inner: Batcher<T>,
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.inner.shared.state.lock().unwrap().open = false;
    }
}

impl<T> Batcher<T> {
    pub fn send(&self, msg: T) {
        let mut state = self.shared.state.lock().unwrap();

        // If the channel is full then drop it; this prevents OOMing
        // when the destination is unavailable
        if state.batch.contents.len() >= self.shared.max_capacity {
            state.batch.contents.clear();
        }

        // If the channel is closed then return without adding
        if !state.open {
            return;
        }

        state.batch.contents.push(msg);
    }

    pub fn receiver(&self) -> Receiver<T> {
        Receiver {
            inner: self.clone(),
        }
    }
}

impl<T> Receiver<T> {
    pub async fn exec<F: Future<Output = Result<(), Error>>>(
        self,
        mut on_batch: impl FnMut(Vec<T>) -> F,
    ) -> Result<(), Error> {
        // This variable holds the "next" buffer
        // Under the lock all we do is push onto a pre-allocated vec
        // and replace it with another pre-allocated vec
        let mut next = Batch::new();

        loop {
            // NOTE: Written weirdly because async doesn't like non-Send locals
            // in the same block as awaits, even if they're dropped before the await
            let (batch, open) = {
                // Held for as little as possible
                let mut state = self.inner.shared.state.lock().unwrap();

                if state.batch.contents.len() > 0 {
                    (
                        mem::replace(&mut state.batch, mem::take(&mut next)),
                        state.open,
                    )
                }
                // If there are no events to emit then notify any watchers and sleep a bit
                else {
                    let watchers = mem::take(&mut state.batch.watchers);
                    let open = state.open;

                    (
                        Batch {
                            contents: Vec::new(),
                            watchers,
                        },
                        open,
                    )
                }
            };

            if batch.contents.len() > 0 {
                // Re-allocate our next buffer outside of the lock
                // TODO: rolling_max
                next = Batch {
                    contents: Vec::with_capacity(batch.contents.len()),
                    watchers: Watchers::new(),
                };

                let _ = on_batch(batch.contents).await;
                batch.watchers.notify();
            } else {
                batch.watchers.notify();

                // If the channel is closed then drop our state and return
                if !open {
                    return Ok(());
                }

                // If we didn't see any events, then sleep for a bit
                // TODO: Exponential backoff?
                tokio::time::sleep(Duration::from_millis(500)).await;
            }
        }
    }
}
