use std::{
    cmp,
    future::Future,
    time::{Duration, Instant},
};

use crate::{BatchError, Channel, Receiver, Sender};

pub fn spawn<
    T: Channel + Send + 'static,
    F: Future<Output = Result<(), BatchError<T>>> + Send + 'static,
>(
    receiver: Receiver<T>,
    on_batch: impl FnMut(T) -> F + Send + 'static,
) where
    T::Item: Send + 'static,
{
    let receive = async move {
        receiver
            .exec(|delay| tokio::time::sleep(delay), on_batch)
            .await
    };

    match tokio::runtime::Handle::try_current() {
        // If we're on a `tokio` thread then spawn on it
        Ok(handle) => {
            handle.spawn(receive);
        }
        // If we're not on a `tokio` thread then spawn a
        // background thread and run the work there
        Err(_) => {
            std::thread::spawn(move || {
                tokio::runtime::Builder::new_current_thread()
                    .enable_all()
                    .build()
                    .unwrap()
                    .block_on(receive)
                    .unwrap();
            });
        }
    }
}

pub fn blocking_flush<T: Channel>(sender: &Sender<T>, timeout: Duration) {
    tokio::task::block_in_place(|| {
        let (notifier, mut notified) = tokio::sync::oneshot::channel();

        sender.on_next_flush(move || {
            let _ = notifier.send(());
        });

        // If there's nothing to flush then return immediately
        if notified.try_recv().is_ok() {
            return;
        }

        match tokio::runtime::Handle::try_current() {
            // If we're on a `tokio` thread then await the receiver
            Ok(handle) => handle.block_on(async {
                let _ = tokio::time::timeout(timeout, notified).await;
            }),
            // If we're not on a `tokio` thread then wait for
            // a notification
            Err(_) => {
                let now = Instant::now();
                let mut wait = Duration::from_micros(1);
                let max_wait_step = cmp::max(timeout / 3, Duration::from_micros(1));

                while now.elapsed() < timeout {
                    if notified.try_recv().is_ok() {
                        return;
                    }

                    // Apply some exponential backoff to avoid spinning
                    // Chances are if data isn't flushed immediately that
                    // it'll be waiting on some network or file IO and could
                    // be a while
                    std::thread::sleep(wait);
                    wait += cmp::min(wait * 2, max_wait_step);
                }
            }
        }
    });
}
