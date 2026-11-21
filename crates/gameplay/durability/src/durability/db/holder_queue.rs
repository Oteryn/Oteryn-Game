//! First-come hand-off of the single ready durability holder to refused owners.
//!
//! Every checkout of the holder is non-blocking. Without an order, the owners that a busy
//! holder refused all wake at its return and the fastest retry takes it: a slower owner, such
//! as a fresh admission, can lose every return until its recovery window runs out. An owner
//! that retries a refused checkout takes a ticket ([`HolderWaiter`]). While a ticket older than
//! a checkout's own is waiting, the checkout is refused, so a returned holder goes to the
//! owner refused first. A served owner keeps its place until its retried step ends or a whole
//! liveness passes without a refusal, so a step of several passes, with other work such as an
//! evidence exchange between them, is not lost to later owners between its passes and repeated
//! whole. Tickets are keyed by the owning task, or by an explicit scope for independent
//! futures polled in one task. Root tasks spawned on an owner's behalf inherit its key, and
//! a future driven outside any task by its thread's `block_on` is keyed by that thread.
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::Notify;

/// An independent future scope, its owning task, or the thread driving it outside any task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OwnerKey {
    Background(u64),
    Scoped(u64),
    Task(tokio::task::Id),
    Thread(std::thread::ThreadId),
}

pub(super) fn is_background() -> bool {
    matches!(current_owner(), OwnerKey::Background(_))
}

tokio::task_local! {
    /// An explicit future owner, also inherited by root tasks spawned on its behalf.
    static SPAWNED_FOR: OwnerKey;
}

/// The current owner: an explicit future scope or inherited owner, else the current task,
/// else the current thread.
pub(super) fn current_owner() -> OwnerKey {
    SPAWNED_FOR.try_with(|owner| *owner).unwrap_or_else(|_| {
        tokio::task::try_id().map_or_else(
            || OwnerKey::Thread(std::thread::current().id()),
            OwnerKey::Task,
        )
    })
}

/// Runs `future` as work of `owner`, so its checkouts keep the owner's place.
pub(super) fn spawned_for<F: Future>(
    owner: OwnerKey,
    future: F,
) -> tokio::task::futures::TaskLocalFuture<OwnerKey, F> {
    SPAWNED_FOR.scope(owner, future)
}

/// A refusal older than this no longer holds the queue: its owner stopped retrying without
/// leaving it. Every retry waits less than this between attempts.
const WAITING_LIVENESS: Duration = Duration::from_secs(1);

struct Waiter {
    owner: OwnerKey,
    ticket: u64,
    /// Set by a refused checkout while the owner waits for the holder; cleared by a served one.
    refused_at: Option<Instant>,
    /// The owner's last refusal, kept across its served passes: its place in the queue.
    last_refused: Option<Instant>,
}

impl Waiter {
    fn refused(&self, now: Instant) -> bool {
        live(self.refused_at, now)
    }

    /// Whether the owner holds later owners back: refused within the liveness, also when a
    /// pass has served it since.
    fn waiting(&self, now: Instant) -> bool {
        live(self.last_refused, now)
    }
}

fn live(refused_at: Option<Instant>, now: Instant) -> bool {
    refused_at.is_some_and(|at| now.saturating_duration_since(at) < WAITING_LIVENESS)
}

/// The current owner's position for an idle holder.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum Turn {
    /// The owner was refused the holder and no owner ahead of it still waits.
    Mine,
    /// An older owner still waits: the holder is kept for it.
    Kept,
    /// Nobody waits ahead, and the owner was not refused the holder: a failed step of its own
    /// failed for another reason.
    Free,
}

/// Maximum retry tickets retained by one root; excess owners keep using plain backoff.
const MAX_HOLDER_WAITERS: usize = 1024;

pub(super) const HOLDER_QUEUE_ALLOCATION_BYTES: usize =
    MAX_HOLDER_WAITERS * std::mem::size_of::<Waiter>();

pub(super) struct HolderQueue {
    next_ticket: u64,
    waiters: Vec<Waiter>,
}

impl Default for HolderQueue {
    fn default() -> Self {
        Self {
            next_ticket: 0,
            waiters: Vec::with_capacity(MAX_HOLDER_WAITERS),
        }
    }
}

impl HolderQueue {
    fn find(&self, owner: OwnerKey) -> Option<&Waiter> {
        self.waiters.iter().find(|waiter| waiter.owner == owner)
    }

    /// The turn of `owner` for an idle holder. An owner without a ticket comes after every
    /// ticket.
    pub(super) fn turn(&self, owner: OwnerKey, now: Instant) -> Turn {
        let mine = self.find(owner);
        let kept = self.waiters.iter().any(|waiter| {
            mine.is_none_or(|mine| waiter.ticket < mine.ticket) && waiter.waiting(now)
        });
        if kept {
            Turn::Kept
        } else if mine.is_some_and(|mine| mine.refused(now)) {
            Turn::Mine
        } else {
            Turn::Free
        }
    }

    /// A refused checkout keeps the owner's ticket waiting.
    pub(super) fn refused(&mut self, owner: OwnerKey, now: Instant) {
        if let Some(waiter) = self.waiter_mut(owner) {
            waiter.refused_at = Some(now);
            waiter.last_refused = Some(now);
        }
    }

    /// A served checkout: the owner keeps its place for the rest of its step, but a later
    /// failure of that step is no refusal of the holder.
    pub(super) fn served(&mut self, owner: OwnerKey) {
        if let Some(waiter) = self.waiter_mut(owner) {
            waiter.refused_at = None;
        }
    }

    /// The owner holds no later owner back, as when it gave up its retries.
    fn stop_waiting(&mut self, owner: OwnerKey) {
        if let Some(waiter) = self.waiter_mut(owner) {
            waiter.refused_at = None;
            waiter.last_refused = None;
        }
    }

    fn waiter_mut(&mut self, owner: OwnerKey) -> Option<&mut Waiter> {
        self.waiters.iter_mut().find(|waiter| waiter.owner == owner)
    }

    /// Enqueues `owner` as refused now. `false` when full or it already holds a ticket.
    fn enqueue(&mut self, owner: OwnerKey, now: Instant) -> bool {
        if self.waiters.len() >= MAX_HOLDER_WAITERS
            || self.waiters.iter().any(|waiter| waiter.owner == owner)
        {
            return false;
        }
        let ticket = self.next_ticket;
        self.next_ticket = self.next_ticket.wrapping_add(1);
        self.waiters.push(Waiter {
            owner,
            ticket,
            refused_at: Some(now),
            last_refused: Some(now),
        });
        true
    }

    fn remove(&mut self, owner: OwnerKey) {
        self.waiters.retain(|waiter| waiter.owner != owner);
    }
}

/// An owner's place in the holder queue, from its first retried refusal until it finishes.
/// Dropping it, also by cancellation, leaves the queue and wakes the owners behind it.
#[must_use = "the queue place ends when the waiter is dropped"]
pub struct HolderWaiter {
    queue: Arc<Mutex<HolderQueue>>,
    returned: Arc<Notify>,
    owner: OwnerKey,
}

impl HolderWaiter {
    /// Takes a place for the current owner, refused now. `None` when an outer retry of the
    /// same owner already holds its place, or the queue is full.
    pub(super) fn enqueue(queue: &Arc<Mutex<HolderQueue>>, returned: &Arc<Notify>) -> Option<Self> {
        let owner = current_owner();
        let mut locked = queue.lock().ok()?;
        locked.enqueue(owner, Instant::now()).then(|| Self {
            queue: Arc::clone(queue),
            returned: Arc::clone(returned),
            owner,
        })
    }

    /// The owner gave up its retries: its place stops holding later owners.
    pub fn stop_waiting(&self) {
        if let Ok(mut queue) = self.queue.lock() {
            queue.stop_waiting(self.owner);
        }
        self.returned.notify_waiters();
    }
}

impl Drop for HolderWaiter {
    fn drop(&mut self) {
        if let Ok(mut queue) = self.queue.lock() {
            queue.remove(self.owner);
        }
        self.returned.notify_waiters();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    fn run(test: impl AsyncFnOnce() -> TestResult) -> TestResult {
        tokio::runtime::Builder::new_current_thread()
            .build()?
            .block_on(test())
    }

    async fn owners() -> Result<(OwnerKey, OwnerKey, OwnerKey), tokio::task::JoinError> {
        let first = tokio::spawn(async { current_owner() }).await?;
        let second = tokio::spawn(async { current_owner() }).await?;
        // Outside any task: the thread driving `block_on`.
        Ok((first, second, current_owner()))
    }

    #[test]
    fn a_full_holder_queue_refuses_new_tickets_without_growing() -> TestResult {
        let mut queue = HolderQueue::default();
        let allocated = queue.waiters.capacity();
        let now = Instant::now();
        for index in 0..MAX_HOLDER_WAITERS {
            assert!(queue.enqueue(OwnerKey::Scoped(u64::try_from(index)?), now));
        }
        let overflow = OwnerKey::Scoped(u64::MAX);
        assert!(
            !queue.enqueue(overflow, now),
            "a full queue must refuse a new ticket"
        );
        assert_eq!(queue.waiters.len(), MAX_HOLDER_WAITERS);
        assert_eq!(queue.waiters.capacity(), allocated);
        assert_eq!(queue.turn(overflow, now), Turn::Kept);
        queue.remove(OwnerKey::Scoped(0));
        assert!(queue.enqueue(overflow, now));
        assert_eq!(queue.turn(overflow, now), Turn::Kept);
        assert_eq!(queue.waiters.capacity(), allocated);
        Ok(())
    }

    #[test]
    fn scoped_futures_in_one_task_keep_independent_fifo_tickets() -> TestResult {
        run(async || {
            tokio::spawn(async {
                let queue = Arc::new(Mutex::new(HolderQueue::default()));
                let returned = Arc::new(Notify::new());
                let take_ticket = || async {
                    let owner = current_owner();
                    let ticket = HolderWaiter::enqueue(&queue, &returned);
                    tokio::task::yield_now().await;
                    assert_eq!(current_owner(), owner, "scope survives a pending poll");
                    (owner, ticket)
                };
                let mut first = std::pin::pin!(
                    crate::durability::DurabilityRoot::with_holder_owner(take_ticket())
                );
                let mut second = std::pin::pin!(
                    crate::durability::DurabilityRoot::with_holder_owner(take_ticket())
                );
                let (mut first_result, mut second_result) = (None, None);
                let ((first_owner, first_ticket), (second_owner, second_ticket)) =
                    std::future::poll_fn(|cx| {
                        if first_result.is_none()
                            && let std::task::Poll::Ready(result) = first.as_mut().poll(cx)
                        {
                            first_result = Some(result);
                        }
                        if second_result.is_none()
                            && let std::task::Poll::Ready(result) = second.as_mut().poll(cx)
                        {
                            second_result = Some(result);
                        }
                        match (first_result.take(), second_result.take()) {
                            (Some(first), Some(second)) => std::task::Poll::Ready((first, second)),
                            (first, second) => {
                                first_result = first;
                                second_result = second;
                                std::task::Poll::Pending
                            }
                        }
                    })
                    .await;
                assert_ne!(first_owner, second_owner);
                assert!(first_ticket.is_some() && second_ticket.is_some());
                {
                    let mut locked = queue.lock().map_err(|_| "queue poisoned")?;
                    let now = Instant::now();
                    assert_eq!(locked.turn(first_owner, now), Turn::Mine);
                    assert_eq!(locked.turn(second_owner, now), Turn::Kept);
                    locked.served(first_owner);
                    assert!(
                        locked
                            .find(second_owner)
                            .is_some_and(|waiter| waiter.refused(now))
                    );
                }
                drop(first_ticket);
                assert_eq!(
                    queue
                        .lock()
                        .map_err(|_| "queue poisoned")?
                        .turn(second_owner, Instant::now()),
                    Turn::Mine
                );
                drop(second_ticket);
                assert!(
                    queue
                        .lock()
                        .map_err(|_| "queue poisoned")?
                        .waiters
                        .is_empty()
                );
                Ok::<(), &'static str>(())
            })
            .await??;
            Ok(())
        })
    }

    #[test]
    fn the_oldest_waiting_owner_takes_the_idle_holder_before_later_ones() -> TestResult {
        run(async || {
            let (first, second, later) = owners().await?;
            let mut queue = HolderQueue::default();
            let now = Instant::now();
            assert!(queue.enqueue(first, now));
            assert!(queue.enqueue(second, now));
            assert_eq!(queue.turn(first, now), Turn::Mine);
            assert_eq!(queue.turn(second, now), Turn::Kept);
            // An owner without a ticket, such as a background pass, comes after every ticket.
            assert_eq!(queue.turn(later, now), Turn::Kept);

            // Refused again, the first owner keeps its old place.
            queue.refused(first, now);
            assert_eq!(queue.turn(first, now), Turn::Mine);
            assert_eq!(queue.turn(second, now), Turn::Kept);

            // A re-enqueue of the same owner keeps its ticket.
            assert!(!queue.enqueue(first, now));
            queue.remove(first);
            assert_eq!(queue.turn(second, now), Turn::Mine);
            queue.remove(second);
            assert_eq!(queue.turn(later, now), Turn::Free);
            Ok(())
        })
    }

    #[test]
    fn a_served_owner_keeps_its_place_until_its_step_ends() -> TestResult {
        run(async || {
            let (first, second, _) = owners().await?;
            let mut queue = HolderQueue::default();
            let now = Instant::now();
            assert!(queue.enqueue(first, now));
            assert!(queue.enqueue(second, now));
            queue.served(first);
            // Between its passes, however far apart, later owners wait for its next one.
            let later = now + WAITING_LIVENESS / 2;
            assert_eq!(queue.turn(second, later), Turn::Kept);
            // Its own retry keeps the backoff of a step that failed for another reason.
            assert_eq!(queue.turn(first, later), Turn::Free);
            // A whole liveness without a refusal ends its place, as does its step's end.
            queue.refused(second, later);
            assert_eq!(queue.turn(second, later), Turn::Kept);
            assert_eq!(queue.turn(second, now + WAITING_LIVENESS), Turn::Mine);
            queue.remove(first);
            assert_eq!(queue.turn(second, later), Turn::Mine);
            Ok(())
        })
    }

    #[test]
    fn a_stale_refusal_no_longer_keeps_the_holder() -> TestResult {
        run(async || {
            let (first, second, later) = owners().await?;
            let mut queue = HolderQueue::default();
            let now = Instant::now();
            assert!(queue.enqueue(first, now));
            assert!(queue.enqueue(second, now + WAITING_LIVENESS / 2));
            let stale = now + WAITING_LIVENESS;
            assert_eq!(queue.turn(first, stale), Turn::Free);
            assert_eq!(queue.turn(second, stale), Turn::Mine);
            assert_eq!(queue.turn(later, stale), Turn::Kept);
            assert_eq!(queue.turn(later, now + WAITING_LIVENESS * 2), Turn::Free);
            Ok(())
        })
    }

    #[test]
    fn dropping_a_waiter_leaves_the_queue_and_spawned_work_keeps_its_owner() -> TestResult {
        run(async || {
            let queue = Arc::new(Mutex::new(HolderQueue::default()));
            let returned = Arc::new(Notify::new());
            let (queue_in, returned_in) = (Arc::clone(&queue), Arc::clone(&returned));
            let checked = tokio::spawn(async move {
                let waiter = HolderWaiter::enqueue(&queue_in, &returned_in);
                let nested = HolderWaiter::enqueue(&queue_in, &returned_in);
                let owner = OwnerKey::Task(tokio::task::id());
                let spawned = tokio::spawn(spawned_for(owner, async { current_owner() })).await;
                (
                    waiter.is_some(),
                    nested.is_none(),
                    spawned.is_ok_and(|spawned| spawned == owner),
                )
            })
            .await?;
            assert_eq!(checked, (true, true, true));
            let empty = queue.lock().is_ok_and(|queue| queue.waiters.is_empty());
            assert!(empty);
            Ok(())
        })
    }
}
