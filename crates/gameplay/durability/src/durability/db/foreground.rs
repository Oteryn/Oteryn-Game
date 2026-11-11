//! Bounded foreground preference for the single ready durability holder.
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

/// Keeps a foreground owner's demand visible across waits and separate passes.
/// Dropping the owner future also drops its demand; no holder is reserved.
#[must_use = "foreground preference ends when the guard is dropped"]
pub(super) struct ForegroundDemand {
    priority: Arc<HolderPriority>,
}

impl Drop for ForegroundDemand {
    fn drop(&mut self) {
        if let Ok(mut state) = self.priority.state.lock() {
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
        let mut state = self.state.lock().ok()?;
        state.foreground = state.foreground.checked_add(1)?;
        // Arrivals never extend a window, even if the previous demand just ended.
        // Only a successful background checkout can renew or clear it.
        state.until.get_or_insert(now + window);
        Some(ForegroundDemand {
            priority: Arc::clone(self),
        })
    }

    /// Check priority and take an already-idle holder in one short critical section.
    /// A continuous foreground stream still permits one background pass per window.
    pub(super) fn background<T>(
        &self,
        now: Instant,
        window: Duration,
        acquire: impl FnOnce() -> Option<T>,
    ) -> Option<T> {
        let mut state = self.state.lock().ok()?;
        if state.foreground > 0 && state.until.is_some_and(|until| now < until) {
            return None;
        }
        let holder = acquire()?;
        state.until = (state.foreground > 0).then_some(now + window);
        Some(holder)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const WINDOW: Duration = Duration::from_secs(5);

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
