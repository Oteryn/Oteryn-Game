//! First-come hand-off of the single ready durability holder to refused owners.
//!
//! Every checkout of the holder is non-blocking. Without an order, an owner that keeps
//! retrying a busy holder can lose every return to owners that arrived after it, and a
//! fresh admission did so until its recovery window ran out. A refused owner that retries
//! takes a ticket; while an older ticket is waiting, a later checkout is refused, so the
//! returned holder goes to the owner refused first. Tickets are keyed by the owning task;
//! root tasks spawned on its behalf inherit its key.
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

/// The task that owns a checkout.
pub(super) type OwnerKey = tokio::task::Id;

tokio::task_local! {
    /// The owner of a root task spawned on another task's behalf.
    static SPAWNED_FOR: OwnerKey;
}

/// The current owner: the task a root task was spawned for, else the current task.
pub(super) fn current_owner() -> Option<OwnerKey> {
    SPAWNED_FOR
        .try_with(|owner| *owner)
        .ok()
        .or_else(tokio::task::try_id)
}

/// Runs `future` as work of `owner`, so its checkouts keep the owner's place.
pub(super) fn spawned_for<F: Future>(
    owner: OwnerKey,
    future: F,
) -> tokio::task::futures::TaskLocalFuture<OwnerKey, F> {
    SPAWNED_FOR.scope(owner, future)
}

/// A refusal older than this no longer holds the queue: its owner stopped retrying without
/// dropping its ticket. Every retry waits less than this between attempts.
const WAITING_LIVENESS: Duration = Duration::from_secs(1);

struct Waiter {
    owner: OwnerKey,
    ticket: u64,
    /// Set while the owner waits for the holder after a refusal; cleared while it tries.
    refused_at: Option<Instant>,
}

#[derive(Default)]
pub(super) struct HolderQueue {
    next_ticket: u64,
    waiters: Vec<Waiter>,
}

impl HolderQueue {
    fn ticket_of(&self, owner: Option<OwnerKey>) -> Option<u64> {
        let owner = owner?;
        self.waiters
            .iter()
            .find(|waiter| waiter.owner == owner)
            .map(|waiter| waiter.ticket)
    }

    /// Whether an older ticket than `owner`'s is still waiting. An owner without a ticket
    /// arrived after every ticket.
    pub(super) fn blocks(&self, owner: Option<OwnerKey>, now: Instant) -> bool {
        let mine = self.ticket_of(owner);
        self.waiters.iter().any(|waiter| {
            Some(waiter.ticket) != mine
                && mine.is_none_or(|mine| waiter.ticket < mine)
                && waiter
                    .refused_at
                    .is_some_and(|at| now.saturating_duration_since(at) < WAITING_LIVENESS)
        })
    }

    /// A refused checkout keeps the owner's ticket waiting.
    pub(super) fn refused(&mut self, owner: Option<OwnerKey>, now: Instant) {
        self.set_refused(owner, Some(now));
    }

    /// A served checkout, or an owner retrying now: its ticket stops holding later owners.
    pub(super) fn trying(&mut self, owner: Option<OwnerKey>) {
        self.set_refused(owner, None);
    }

    fn set_refused(&mut self, owner: Option<OwnerKey>, refused_at: Option<Instant>) {
        let Some(owner) = owner else {
            return;
        };
        if let Some(waiter) = self.waiters.iter_mut().find(|waiter| waiter.owner == owner) {
            waiter.refused_at = refused_at;
        }
    }

    /// Enqueues `owner` as refused now. `false` when it already holds a ticket.
    fn enqueue(&mut self, owner: OwnerKey, now: Instant) -> bool {
        if self.waiters.iter().any(|waiter| waiter.owner == owner) {
            self.refused(Some(owner), now);
            return false;
        }
        let ticket = self.next_ticket;
        self.next_ticket = self.next_ticket.wrapping_add(1);
        self.waiters.push(Waiter {
            owner,
            ticket,
            refused_at: Some(now),
        });
        true
    }

    fn remove(&mut self, owner: OwnerKey) {
        self.waiters.retain(|waiter| waiter.owner != owner);
    }
}

/// An owner's place in the queue, from its first retried refusal until it finishes. Dropping
/// it, also by cancellation, leaves the queue.
#[must_use = "the queue place ends when the waiter is dropped"]
pub struct HolderWaiter {
    queue: Arc<Mutex<HolderQueue>>,
    owner: OwnerKey,
}

impl HolderWaiter {
    /// Takes a place for the current owner, refused now. `None` outside a task, or when an
    /// outer retry of the same owner already holds its place.
    pub(super) fn enqueue(queue: &Arc<Mutex<HolderQueue>>) -> Option<Self> {
        let owner = current_owner()?;
        let mut locked = queue.lock().ok()?;
        locked.enqueue(owner, Instant::now()).then(|| Self {
            queue: Arc::clone(queue),
            owner,
        })
    }
}

impl Drop for HolderWaiter {
    fn drop(&mut self) {
        if let Ok(mut queue) = self.queue.lock() {
            queue.remove(self.owner);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn owner() -> OwnerKey {
        tokio::spawn(async { tokio::task::id() }).await.unwrap_or_else(|_| tokio::task::id())
    }

    #[tokio::test]
    async fn the_oldest_waiting_owner_is_served_before_later_arrivals() {
        let mut queue = HolderQueue::default();
        let (first, second) = (owner().await, owner().await);
        let now = Instant::now();
        assert!(queue.enqueue(first, now));
        assert!(queue.enqueue(second, now));
        assert!(!queue.blocks(Some(first), now));
        assert!(queue.blocks(Some(second), now));
        assert!(queue.blocks(None, now), "a later arrival waits behind both");
        // The first owner is served; the second is next, still ahead of new arrivals.
        queue.trying(Some(first));
        assert!(!queue.blocks(Some(second), now));
        assert!(queue.blocks(None, now));
        queue.remove(first);
        queue.remove(second);
        assert!(!queue.blocks(None, now));
    }

    #[tokio::test]
    async fn an_abandoned_refusal_stops_holding_the_queue() {
        let mut queue = HolderQueue::default();
        let first = owner().await;
        let now = Instant::now();
        assert!(queue.enqueue(first, now));
        assert!(queue.blocks(None, now));
        assert!(!queue.blocks(None, now + WAITING_LIVENESS));
    }

    #[tokio::test]
    async fn a_second_place_for_the_same_owner_keeps_the_first_ticket() {
        let mut queue = HolderQueue::default();
        let (first, second) = (owner().await, owner().await);
        let now = Instant::now();
        assert!(queue.enqueue(first, now));
        assert!(queue.enqueue(second, now));
        assert!(!queue.enqueue(first, now));
        assert!(queue.blocks(Some(second), now));
    }
}
