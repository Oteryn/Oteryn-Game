use oteryn_client_ui::login_queue::QueueWait;
use std::time::{Duration, Instant};

#[test]
fn countdown_rounds_up_and_retry_is_consumed_once() {
    let now = Instant::now();
    let mut queue = QueueWait::new(Some(7), now, Duration::from_millis(1500));
    assert_eq!(queue.text(now), "Queue position 7. Retrying in 2s.");
    assert!(!queue.take_retry(now + Duration::from_secs(1)));
    assert!(queue.take_retry(now + Duration::from_secs(2)));
    assert!(!queue.take_retry(now + Duration::from_secs(3)));
}
#[test]
fn cancellation_never_retries_or_invents_a_position() {
    let now = Instant::now();
    let mut queue = QueueWait::new(None, now, Duration::from_secs(1));
    assert!(queue.text(now).starts_with("Waiting for capacity."));
    queue.cancel();
    assert!(!queue.take_retry(now + Duration::from_secs(10)));
}
