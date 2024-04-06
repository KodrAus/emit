use crate::internal_metrics::InternalMetrics;
use std::{
    any::Any,
    cmp,
    future::{self, Future},
    mem,
    panic::{self, AssertUnwindSafe, UnwindSafe},
    pin::{pin, Pin},
    sync::{Arc, Mutex, OnceLock},
    task,
    task::{Context, Poll},
    thread,
    time::Duration,
};

mod internal_metrics;

type Error = Box<dyn std::error::Error + Send + Sync + 'static>;

pub trait Channel {
    type Item;

    fn new() -> Self;

    fn with_capacity(capacity: usize) -> Self
    where
        Self: Sized,
    {
        let _ = capacity;

        Self::new()
    }

    fn push<'a>(&mut self, item: Self::Item);

    fn remaining(&self) -> usize;

    fn is_empty(&self) -> bool {
        self.remaining() == 0
    }

    fn clear(&mut self);
}

impl<T> Channel for Vec<T> {
    type Item = T;

    fn new() -> Self {
        Vec::new()
    }

    fn with_capacity(capacity: usize) -> Self {
        Vec::with_capacity(capacity)
    }

    fn push<'a>(&mut self, item: Self::Item) {
        self.push(item);
    }

    fn remaining(&self) -> usize {
        self.len()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn clear(&mut self) {
        self.clear()
    }
}

pub fn bounded<T: Channel>(max_capacity: usize) -> (Sender<T>, Receiver<T>) {
    let shared = Arc::new(Shared {
        metrics: Default::default(),
        state: Mutex::new(State {
            next_batch: Batch::new(),
            is_open: true,
            is_in_batch: false,
        }),
    });

    (
        Sender {
            max_capacity,
            shared: shared.clone(),
        },
        Receiver {
            idle_delay: Delay::new(Duration::from_millis(1), Duration::from_millis(500)),
            retry: Retry::new(10),
            retry_delay: Delay::new(Duration::from_millis(50), Duration::from_secs(1)),
            capacity: Capacity::new(),
            shared,
        },
    )
}

pub struct Sender<T> {
    max_capacity: usize,
    shared: Arc<Shared<T>>,
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        self.shared.state.lock().unwrap().is_open = false;
    }
}

impl<T: Channel> Sender<T> {
    pub fn send<'a>(&self, msg: T::Item) {
        let mut state = self.shared.state.lock().unwrap();

        // If the channel is full then drop it; this prevents OOMing
        // when the destination is unavailable. We don't notify the batch
        // in this case because the clearing is opaque to outside observers
        if state.next_batch.channel.remaining() >= self.max_capacity {
            state.next_batch.channel.clear();
            self.shared.metrics.queue_overflow.increment();
        }

        // If the channel is closed then return without adding the message
        if !state.is_open {
            return;
        }

        state.next_batch.channel.push(msg);
    }

    pub fn on_next_flush(&self, watcher: impl FnOnce() + Send + 'static) {
        let watcher = Box::new(watcher);

        let mut state = self.shared.state.lock().unwrap();

        // If:
        // - We're not in a batch and
        //   - the next batch is empty (there's no data) or
        //   - the state is closed
        // Then:
        // - Call the watcher without scheduling it; there's nothing to wait for
        if !state.is_in_batch && (state.next_batch.channel.is_empty() || !state.is_open) {
            // Drop the lock before signalling the watcher
            drop(state);

            watcher();
        }
        // If there's active data to flush then schedule the watcher
        else {
            state.next_batch.watchers.push(watcher);
        }
    }

    pub fn sample_metrics(
        &self,
    ) -> impl Iterator<Item = emit::metric::Metric<'static, emit::empty::Empty>> + 'static {
        self.shared.sample_metrics()
    }
}

pub struct Receiver<T> {
    idle_delay: Delay,
    retry: Retry,
    retry_delay: Delay,
    capacity: Capacity,
    shared: Arc<Shared<T>>,
}

impl<T> Drop for Receiver<T> {
    fn drop(&mut self) {
        self.shared.state.lock().unwrap().is_open = false;
    }
}

pub struct BatchError<T> {
    retryable: T,
}

impl<T: Channel> BatchError<T> {
    pub fn retry(_: impl std::error::Error + Send + Sync + 'static, retryable: T) -> Self {
        BatchError { retryable }
    }

    pub fn no_retry(_: impl std::error::Error + Send + Sync + 'static) -> Self {
        BatchError {
            retryable: T::new(),
        }
    }

    pub fn into_retryable(self) -> T {
        self.retryable
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> BatchError<U> {
        BatchError {
            retryable: f(self.retryable),
        }
    }
}

impl<T: Channel> Receiver<T> {
    pub fn blocking_exec(
        self,
        mut on_batch: impl FnMut(T) -> Result<(), BatchError<T>>,
    ) -> Result<(), Error> {
        static WAKER: OnceLock<Arc<NeverWake>> = OnceLock::new();

        // A waker that does nothing; the tasks it runs are fully
        // synchronous so there's never any notifications to issue
        struct NeverWake;

        impl task::Wake for NeverWake {
            fn wake(self: Arc<Self>) {}
        }

        // The future is polled to completion here, so we can pin
        // it directly on the stack
        let mut fut = pin!(self.exec(
            |delay| future::ready(thread::sleep(delay)),
            move |batch| future::ready(on_batch(batch)),
        ));

        // Get a context for our synchronous task
        let waker = WAKER.get_or_init(|| Arc::new(NeverWake)).clone().into();
        let mut cx = task::Context::from_waker(&waker);

        // Drive the task to completion; it should complete in one go,
        // but may eagerly return as soon as it hits an await point, so
        // just to be sure we continuously poll it
        loop {
            match fut.as_mut().poll(&mut cx) {
                task::Poll::Ready(r) => return r,
                task::Poll::Pending => continue,
            }
        }
    }

    pub async fn exec<
        FBatch: Future<Output = Result<(), BatchError<T>>>,
        FWait: Future<Output = ()>,
    >(
        mut self,
        mut wait: impl FnMut(Duration) -> FWait,
        mut on_batch: impl FnMut(T) -> FBatch,
    ) -> Result<(), Error> {
        // This variable holds the "next" batch
        // Under the lock all we do is push onto a pre-allocated vec
        // and replace it with another pre-allocated vec
        let mut next_batch = Batch::new();

        loop {
            // Run inside the lock
            let (mut current_batch, is_open) = {
                let mut state = self.shared.state.lock().unwrap();

                // NOTE: We don't check the `is_open` value here because we want a chance to emit
                // any last batch

                // If there are events then mark that we're in a batch and replace it with an empty one
                // The sender will start filling this new batch
                if state.next_batch.channel.remaining() > 0 {
                    state.is_in_batch = true;

                    (
                        mem::replace(&mut state.next_batch, mem::take(&mut next_batch)),
                        state.is_open,
                    )
                }
                // If there are no events to emit then mark that we're outside of a batch and take its watchers
                else {
                    state.is_in_batch = false;

                    let watchers = mem::take(&mut state.next_batch.watchers);
                    let open = state.is_open;

                    (
                        Batch {
                            channel: T::new(),
                            watchers,
                        },
                        open,
                    )
                }
            };

            // Run outside of the lock
            if current_batch.channel.remaining() > 0 {
                self.retry.reset();
                self.retry_delay.reset();
                self.idle_delay.reset();

                // Re-allocate our next buffer outside of the lock
                next_batch = Batch {
                    channel: T::with_capacity(
                        self.capacity.next(current_batch.channel.remaining()),
                    ),
                    watchers: Watchers::new(),
                };

                // Emit the batch, taking care not to panic
                loop {
                    match panic::catch_unwind(AssertUnwindSafe(|| on_batch(current_batch.channel)))
                    {
                        Ok(on_batch) => match CatchUnwind(AssertUnwindSafe(on_batch)).await {
                            Ok(Ok(())) => {
                                self.shared.metrics.queue_batch_processed.increment();
                            }
                            Ok(Err(BatchError { retryable })) => {
                                self.shared.metrics.queue_batch_failed.increment();

                                if retryable.remaining() > 0 && self.retry.next() {
                                    // Delay a bit before trying again; this gives the external service
                                    // a chance to get itself together
                                    wait(self.retry_delay.next()).await;

                                    current_batch = Batch {
                                        channel: retryable,
                                        watchers: current_batch.watchers,
                                    };

                                    self.shared.metrics.queue_batch_retry.increment();
                                    continue;
                                }
                            }
                            Err(_) => {
                                self.shared.metrics.queue_batch_panicked.increment();
                            }
                        },
                        Err(_) => {
                            self.shared.metrics.queue_batch_panicked.increment();
                        }
                    }

                    break;
                }

                // After the batch has been emitted, notify any watchers
                current_batch.watchers.notify();
            }
            // If the batch was empty then notify any watchers (there was nothing to flush)
            // and wait before checking again
            else {
                current_batch.watchers.notify();

                // If the channel is closed then exit the loop and return; this will
                // drop the receiver
                if !is_open {
                    return Ok(());
                }

                // If we didn't see any events, then sleep for a bit
                wait(self.idle_delay.next()).await;
            }
        }
    }

    pub fn sample_metrics(
        &self,
    ) -> impl Iterator<Item = emit::metric::Metric<'static, emit::empty::Empty>> + 'static {
        self.shared.sample_metrics()
    }
}

struct CatchUnwind<F>(F);

impl<F: Future + UnwindSafe> Future for CatchUnwind<F> {
    type Output = Result<F::Output, Box<dyn Any + Send>>;

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        // SAFETY: `CatchUnwind` uses structural pinning
        let f = unsafe { Pin::map_unchecked_mut(self, |x| &mut x.0) };

        panic::catch_unwind(AssertUnwindSafe(|| f.poll(cx)))?.map(Ok)
    }
}

struct Delay {
    current: Duration,
    step: Duration,
    max: Duration,
}

impl Delay {
    fn new(step: Duration, max: Duration) -> Delay {
        Delay {
            current: Duration::ZERO,
            step,
            max,
        }
    }

    fn reset(&mut self) {
        self.current = Duration::ZERO
    }

    fn next(&mut self) -> Duration {
        self.current = cmp::min(self.current * 2 + self.step, self.max);
        self.current
    }
}

const CAPACITY_WINDOW: usize = 16;

struct Capacity([usize; CAPACITY_WINDOW], usize);

impl Capacity {
    fn new() -> Self {
        Capacity([1; CAPACITY_WINDOW], 0)
    }

    fn next(&mut self, last_len: usize) -> usize {
        self.0[self.1 % CAPACITY_WINDOW] = last_len;
        self.0.iter().copied().max().unwrap()
    }
}

struct Retry {
    current: u32,
    max: u32,
}

impl Retry {
    fn new(max: u32) -> Self {
        Retry { current: 0, max }
    }

    fn reset(&mut self) {
        self.current = 0;
    }

    fn next(&mut self) -> bool {
        self.current += 1;
        self.current <= self.max
    }
}

struct Shared<T> {
    metrics: InternalMetrics,
    state: Mutex<State<T>>,
}

impl<T: Channel> Shared<T> {
    fn sample_metrics(
        &self,
    ) -> impl Iterator<Item = emit::metric::Metric<'static, emit::empty::Empty>> + 'static {
        let queue_length = { self.state.lock().unwrap().next_batch.channel.remaining() };

        self.metrics.sample().chain(Some(emit::metric::Metric::new(
            env!("CARGO_PKG_NAME"),
            emit::empty::Empty,
            "queue_length",
            emit::well_known::METRIC_AGG_LAST,
            queue_length,
            emit::empty::Empty,
        )))
    }
}

struct State<T> {
    next_batch: Batch<T>,
    is_open: bool,
    is_in_batch: bool,
}

struct Batch<T> {
    channel: T,
    watchers: Watchers,
}

impl<T: Channel> Batch<T> {
    fn new() -> Self {
        Batch {
            channel: T::new(),
            watchers: Watchers::new(),
        }
    }
}

impl<T: Channel> Default for Batch<T> {
    fn default() -> Self {
        Batch::new()
    }
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

pub mod sync;

#[cfg(feature = "tokio")]
pub mod tokio;
