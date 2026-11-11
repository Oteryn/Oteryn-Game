//! First-come hand-off of the single ready durability holder to refused owners.
//!
//! Every checkout of the holder is non-blocking. Without an order, the owners that a busy
//! holder refused all wake at its return and the fastest retry takes it: a slower owner, such
//! as a fresh admission, can lose every return until its recovery window runs out. An owner
//! that retries a refused checkout takes a ticket ([`HolderWaiter`]). While a ticket older than
//! a checkout's own is waiting, the checkout is refused, so a returned holder goes to the
//! owner refused first. An owner whose pass just returned the holder keeps its place for a
//! short grace, so a step of several passes is not lost to later owners between its passes and
//! repeated whole. Tickets are keyed by the owning task; root tasks spawned on its behalf
//! inherit its key, and a future driven outside any task by its thread's `block_on` is keyed by
//! that thread.
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use tokio::sync::Notify;

/// The task that owns a checkout, or the thread whose `block_on` drives it outside any task.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum OwnerKey {
    Task(tokio::task::Id),
    Thread(std::thread::ThreadId),
}

tokio::task_local! {
    /// The owner of a root task spawned on another task's behalf.
    static SPAWNED_FOR: OwnerKey;
}

/// The current owner: the owner a root task was spawned for, else the current task, else the
/// current thread.
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
/// How long an owner whose pass returned the holder keeps its place for its next pass.
const RETURN_GRACE: Duration = Duration::from_millis(50);

struct Waiter {
    owner: OwnerKey,
    ticket: u64,
    /// Set by a refused checkout while the owner waits for the holder; cleared by a served one.
    refused_at: Option<Instant>,
    /// When the owner's last served pass returned the holder.
    returned_at: Option<Instant>,
}

impl Waiter {
    fn refused(&self, now: Instant) -> bool {
        self.refused_at
            .is_some_and(|at| now.saturating_duration_since(at) < WAITING_LIVENESS)
    }

    /// Whether the owner holds later owners back: refused, or within its return grace.
    fn waiting(&self, now: Instant) -> bool {
        self.refused(now)
            || self
                .returned_at
                .is_some_and(|at| now.saturating_duration_since(at) < RETURN_GRACE)
    }
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

#[derive(Default)]
pub(super) struct HolderQueue {
    next_ticket: u64,
    waiters: Vec<Waiter>,
    /// The owner of the last served checkout.
    holding: Option<OwnerKey>,
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
        }
    }

    /// A served checkout: the owner's ticket holds no later owner back while it has the holder.
    pub(super) fn served(&mut self, owner: OwnerKey) {
        self.stop_waiting(owner);
        self.holding = Some(owner);
    }

    /// The last served checkout returned the holder to idle: its owner keeps its place for
    /// [`RETURN_GRACE`].
    pub(super) fn returned(&mut self, now: Instant) {
        if let Some(owner) = self.holding.take()
            && let Some(waiter) = self.waiter_mut(owner)
        {
            waiter.returned_at = Some(now);
        }
    }

    /// The owner holds no later owner back, as when it gave up its retries.
    fn stop_waiting(&mut self, owner: OwnerKey) {
        if let Some(waiter) = self.waiter_mut(owner) {
            waiter.refused_at = None;
            waiter.returned_at = None;
        }
    }

    fn waiter_mut(&mut self, owner: OwnerKey) -> Option<&mut Waiter> {
        self.waiters.iter_mut().find(|waiter| waiter.owner == owner)
    }

    /// Enqueues `owner` as refused now. `false` when it already holds a ticket.
    fn enqueue(&mut self, owner: OwnerKey, now: Instant) -> bool {
        if self.waiters.iter().any(|waiter| waiter.owner == owner) {
            return false;
        }
        let ticket = self.next_ticket;
        self.next_ticket = self.next_ticket.wrapping_add(1);
        self.waiters.push(Waiter {
            owner,
            ticket,
            refused_at: Some(now),
            returned_at: None,
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
    /// same owner already holds its place.
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

            // Served, the first owner holds nobody back; refused again, it keeps its old place.
            queue.served(first);
            assert_eq!(queue.turn(second, now), Turn::Mine);
            assert_eq!(queue.turn(first, now), Turn::Free);
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
    fn an_owner_whose_pass_returned_the_holder_keeps_its_place_for_its_next_pass() -> TestResult {
        run(async || {
            let (first, second, _) = owners().await?;
            let mut queue = HolderQueue::default();
            let now = Instant::now();
            assert!(queue.enqueue(first, now));
            assert!(queue.enqueue(second, now));
            queue.served(first);
            assert_eq!(queue.turn(second, now), Turn::Mine);
            queue.returned(now);
            // Its own retry keeps the backoff of a step that failed for another reason.
            assert_eq!(queue.turn(first, now), Turn::Free);
            assert_eq!(queue.turn(second, now), Turn::Kept);
            assert_eq!(queue.turn(second, now + RETURN_GRACE), Turn::Mine);
            // Only the owner of the returned pass gets the grace, once.
            queue.returned(now + RETURN_GRACE);
            assert_eq!(queue.turn(second, now + RETURN_GRACE), Turn::Mine);
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
            assert_eq!(
                queue.turn(later, now + WAITING_LIVENESS * 2),
                Turn::Free
            );
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
