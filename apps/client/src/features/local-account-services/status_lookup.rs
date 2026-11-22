//! Poll the bounded history page as one batch, preserving server order and partial results.
use std::{
    future::{Future, poll_fn},
    task::Poll,
    time::Duration,
};

pub(crate) async fn collect<F: Future>(lookups: impl Iterator<Item = F>) -> Vec<Option<F::Output>> {
    let mut pending: Vec<_> = lookups
        .enumerate()
        .map(|(index, future)| (index, Box::pin(future)))
        .collect();
    let mut results: Vec<_> = (0..pending.len()).map(|_| None).collect();
    let batch = poll_fn(|cx| {
        pending.retain_mut(|(index, future)| match future.as_mut().poll(cx) {
            Poll::Ready(result) => {
                results[*index] = Some(result);
                false
            }
            Poll::Pending => true,
        });
        if pending.is_empty() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    });
    let _ = tokio::time::timeout(Duration::from_secs(15), batch).await;
    results
}
