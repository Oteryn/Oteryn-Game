use std::future::{Future, poll_fn};
use std::pin::pin;
use std::task::Poll;

/// Output of `primary`, or `None` once `stop` completes first.
pub(super) async fn first<T>(
    primary: impl Future<Output = T>,
    stop: impl Future<Output = ()>,
) -> Option<T> {
    let mut primary = pin!(primary);
    let mut stop = pin!(stop);
    poll_fn(|context| {
        if let Poll::Ready(value) = primary.as_mut().poll(context) {
            return Poll::Ready(Some(value));
        }
        stop.as_mut().poll(context).map(|()| None)
    })
    .await
}
