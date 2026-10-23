//! Root-aware waiting for the composed Monk-save fault/recovery fixture.
use std::time::Duration;

/// Polls until the observed release boundary has no checked-out root holder.
/// A release that ends first is returned as the error. Readiness prevents dropping
/// a pending save at its attempt counter and racing the next durable observation.
pub(super) async fn drive_until<F: std::future::Future>(
    root: &crate::durability::DurabilityRoot,
    release: F,
    reached: impl Fn() -> bool,
) -> Result<(), F::Output> {
    use std::task::Poll;
    let mut release = std::pin::pin!(release);
    let mut probe = std::pin::pin!(async {
        while !reached() || !root.is_ready() {
            tokio::time::sleep(Duration::from_millis(10)).await;
        }
    });
    std::future::poll_fn(move |context| {
        if let Poll::Ready(output) = release.as_mut().poll(context) {
            return Poll::Ready(Err(output));
        }
        if probe.as_mut().poll(context).is_ready() {
            return Poll::Ready(Ok(()));
        }
        Poll::Pending
    })
    .await
}
