//! Bounded foreground preference for the single ready durability holder.
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[derive(Default)]
struct State {
    foreground: usize,
    until: Option<Instant>,
}

/// One fixed-size arbitration state shared by all clones of a root.
#[derive(Default)]
pub(super) struct HolderPriority {
    state: Mutex<State>,
}

tokio::task_local! {
    static BACKGROUND_OPERATION: Option<(Arc<HolderPriority>, Arc<AtomicBool>)>;
}

pub(super) async fn operation<F: Future>(priority: &Arc<HolderPriority>, future: F) -> F::Output {
    BACKGROUND_OPERATION
        .scope(
            Some((Arc::clone(priority), Arc::new(AtomicBool::new(false)))),
            future,
        )
        .await
}

pub(super) fn inherit_operation<F: Future>(future: F) -> impl Future<Output = F::Output> {
    let operation = BACKGROUND_OPERATION.try_with(Clone::clone).ok().flatten();
    BACKGROUND_OPERATION.scope(operation, future)
}

/// Keeps a foreground owner's demand visible across waits and separate passes.
/// Dropping the owner future also drops its demand; no holder is reserved.
#[must_use = "foreground preference ends when the guard is dropped"]
pub(super) struct ForegroundDemand {
    priority: Option<Arc<HolderPriority>>,
}

impl Drop for ForegroundDemand {
    fn drop(&mut self) {
        if let Some(priority) = &self.priority
            && let Ok(mut state) = priority.state.lock()
        {
            state.foreground -= 1;
        }
    }
}

impl HolderPriority {
    pub(super) fn foreground(
        self: &Arc<Self>,
        now: Instant,
        window: Duration,
    ) -> Option<ForegroundDemand> {
        // Shared observation helpers may request foreground preference, but a
        // scheduled caller must neither promote itself nor block its own passes.
        if super::holder_queue::is_background() {
            return Some(ForegroundDemand { priority: None });
        }
        let mut state = self.state.lock().ok()?;
        state.foreground = state.foreground.checked_add(1)?;
        // Arrivals never extend a window, even if the previous demand just ended.
        // Only a successful background checkout can renew or clear it.
        state.until.get_or_insert(now + window);
        Some(ForegroundDemand {
            priority: Some(Arc::clone(self)),
        })
    }

    /// Check priority and take an already-idle holder in one short critical section.
    /// A continuous foreground stream still permits one background operation per window.
    pub(super) fn background<T>(
        &self,
        now: Instant,
        window: Duration,
        acquire: impl FnOnce() -> Option<T>,
    ) -> Option<T> {
        let mut state = self.state.lock().ok()?;
        let grant = BACKGROUND_OPERATION
            .try_with(|operation| {
                operation.as_ref().and_then(|(priority, granted)| {
                    std::ptr::eq(self, Arc::as_ptr(priority)).then(|| Arc::clone(granted))
                })
            })
            .ok()
            .flatten();
        let granted = grant
            .as_ref()
            .is_some_and(|grant| grant.load(Ordering::Relaxed));
        if !granted && state.foreground > 0 && state.until.is_some_and(|until| now < until) {
            return None;
        }
        let holder = acquire()?;
        if !granted {
            state.until = (state.foreground > 0).then_some(now + window);
            if let Some(grant) = grant {
                grant.store(true, Ordering::Relaxed);
            }
        }
        Some(holder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Duration = Duration::from_secs(5);

    #[test]
    fn nbhold_background_observation_does_not_create_foreground_demand()
    -> Result<(), Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_current_thread().build()?;
        runtime.block_on(async {
            let priority = Arc::new(HolderPriority::default());
            let now = Instant::now();
            super::super::DurabilityRoot::with_background_holder(async {
                let demand = priority.foreground(now, WINDOW);
                assert!(demand.is_some());
                assert_eq!(priority.background(now, WINDOW, || Some(())), Some(()));
                let owner = super::super::holder_queue::current_owner();
                super::super::DurabilityRoot::with_background_holder(async {
                    assert_ne!(super::super::holder_queue::current_owner(), owner);
                })
                .await;
                assert_eq!(super::super::holder_queue::current_owner(), owner);
            })
            .await;
            let demand = priority.foreground(now, WINDOW);
            assert!(demand.is_some());
            assert_eq!(priority.background(now, WINDOW, || Some(())), None);
        });
        Ok(())
    }

    #[test]
    fn nbhold_background_scope_inherits_root_tasks_and_cancels_without_a_ticket()
    -> Result<(), Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
        runtime.block_on(async {
            use super::super::{DurabilityRoot, holder_queue};
            let root = DurabilityRoot::connect_test_runtime("postgres://oteryn@127.0.0.1:1/oteryn")?;
            let owner = holder_queue::current_owner();
            let ready = std::cell::Cell::new(false);
            let mut background = Box::pin(DurabilityRoot::with_background_holder(async {
                let background_owner = holder_queue::current_owner();
                assert_ne!(background_owner, owner);
                assert!(root.holder_waiter().is_none());
                let spawned = root.clone();
                let inherited = root.spawn_task(async move {
                    assert!(spawned.holder_waiter().is_none());
                    holder_queue::current_owner()
                }).await?;
                assert_eq!(inherited, background_owner);
                ready.set(true);
                std::future::pending::<()>().await;
                Ok::<(), Box<dyn std::error::Error>>(())
            }));
            // Poll until the spawned task has completed and the scope is parked.
            tokio::time::timeout(Duration::from_secs(1), std::future::poll_fn(|cx| {
                if let std::task::Poll::Ready(result) = background.as_mut().poll(cx) {
                    return std::task::Poll::Ready(result.and(
                        Err("background scope ended instead of parking".into()),
                    ));
                }
                if ready.get() {
                    std::task::Poll::Ready(Ok(()))
                } else {
                    std::task::Poll::Pending
                }
            })).await??;
            drop(background);
            assert_eq!(holder_queue::current_owner(), owner);
            let ticket = root.holder_waiter();
            assert!(ticket.is_some(), "cancellation restores ordinary FIFO ownership");
            Ok::<(), Box<dyn std::error::Error>>(())
        })
    }

    #[test]
    fn background_operation_keeps_one_grant_across_passes() -> Result<(), Box<dyn std::error::Error>> {
        let runtime = tokio::runtime::Builder::new_current_thread().build()?;
        runtime.block_on(async {
            let priority = Arc::new(HolderPriority::default());
            let other = Arc::new(HolderPriority::default());
            let now = Instant::now();
            let demand = priority.foreground(now, WINDOW);
            assert!(demand.is_some());
            operation(&priority, async {
                assert_eq!(priority.background(now, WINDOW, || Some(())), None);
                assert_eq!(priority.background::<()>(now + WINDOW, WINDOW, || None), None);
                assert_eq!(
                    priority.background(now + WINDOW, WINDOW, || Some(())),
                    Some(())
                );
                let other_demand = other.foreground(now + WINDOW, WINDOW);
                assert!(other_demand.is_some());
                assert_eq!(other.background(now + WINDOW, WINDOW, || Some(())), None);
                let inherited = Arc::clone(&priority);
                tokio::spawn(inherit_operation(async move {
                    assert_eq!(
                        inherited.background(now + WINDOW, WINDOW, || Some(())),
                        Some(())
                    );
                }))
                .await?;
                Ok::<(), Box<dyn std::error::Error>>(())
            })
            .await?;
            operation(&priority, async {
                assert_eq!(priority.background(now + WINDOW, WINDOW, || Some(())), None);
            })
            .await;
            Ok::<(), Box<dyn std::error::Error>>(())
        })
    }

    #[test]
    fn returned_holder_cannot_be_retaken_by_background_before_foreground() {
        let priority = Arc::new(HolderPriority::default());
        let now = Instant::now();
        assert_eq!(priority.background(now, WINDOW, || Some(1)), Some(1));
        let demand = priority.foreground(now, WINDOW);
        assert!(demand.is_some());
        // The prior pass just returned. Reacquisition is attempted immediately,
        // without any sleep or scheduler opportunity for the foreground owner.
        for _ in 0..16 {
            let mut checked_out = false;
            assert_eq!(
                priority.background(now, WINDOW, || {
                    checked_out = true;
                    Some(2)
                }),
                None
            );
            assert!(!checked_out);
        }
        drop(demand);
        assert_eq!(priority.background(now, WINDOW, || Some(3)), Some(3));
    }

    #[test]
    fn cancellation_and_nested_demands_release_only_their_own_preference() {
        let priority = Arc::new(HolderPriority::default());
        let now = Instant::now();
        let outer = priority.foreground(now, WINDOW);
        let inner = priority.foreground(now, WINDOW);
        assert!(outer.is_some() && inner.is_some());
        drop(inner);
        assert_eq!(priority.background(now, WINDOW, || Some(())), None);
        drop(outer);
        assert_eq!(priority.background(now, WINDOW, || Some(())), Some(()));
    }

    #[test]
    fn new_demands_do_not_renew_the_window() {
        let priority = Arc::new(HolderPriority::default());
        let now = Instant::now();
        let first = priority.foreground(now, WINDOW);
        assert!(first.is_some());
        drop(first);
        let later = priority.foreground(now + WINDOW / 2, WINDOW);
        assert!(later.is_some());
        assert_eq!(
            priority.background(now + WINDOW, WINDOW, || Some(())),
            Some(())
        );
    }

    #[test]
    fn expired_window_allows_background_before_renewing_preference() {
        let priority = Arc::new(HolderPriority::default());
        let now = Instant::now();
        let demand = priority.foreground(now, WINDOW);
        assert!(demand.is_some());
        assert_eq!(
            priority.background(now + WINDOW, WINDOW, || Some(1)),
            Some(1)
        );
        assert_eq!(
            priority.background(now + WINDOW, WINDOW, || Some(2)),
            None
        );
        assert_eq!(
            priority.background(now + WINDOW * 2, WINDOW, || Some(3)),
            Some(3)
        );
    }

    #[test]
    fn unavailable_background_holder_does_not_renew_the_window() {
        let priority = Arc::new(HolderPriority::default());
        let now = Instant::now();
        let demand = priority.foreground(now, WINDOW);
        assert!(demand.is_some());
        assert_eq!(priority.background::<()>(now + WINDOW, WINDOW, || None), None);
        assert_eq!(
            priority.background(now + WINDOW, WINDOW, || Some(())),
            Some(())
        );
    }

    #[test]
    fn clones_share_demand_but_distinct_roots_do_not() {
        let priority = Arc::new(HolderPriority::default());
        let clone = Arc::clone(&priority);
        let other = HolderPriority::default();
        let now = Instant::now();
        let demand = priority.foreground(now, WINDOW);
        assert!(demand.is_some());
        assert_eq!(clone.background(now, WINDOW, || Some(())), None);
        assert_eq!(other.background(now, WINDOW, || Some(())), Some(()));
    }
}
