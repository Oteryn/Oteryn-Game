//! Opt-in diagnostic timing for the statements executed by one holder pass.
use std::cell::Cell;
use std::sync::atomic::{AtomicU64, Ordering::Relaxed};
use std::sync::Arc;
use std::time::{Duration, Instant};

thread_local! {
    static PROFILE: Cell<Option<Arc<PgQueryProfile>>> = const { Cell::new(None) };
}

/// A diagnostic scope installed only while a measured future is polled.
#[doc(hidden)]
#[derive(Default)]
pub struct PgQueryProfile {
    statements: AtomicU64,
    active: AtomicU64,
    elapsed_ns: AtomicU64,
}

impl PgQueryProfile {
    /// Scope one synchronous poll. The statement guard follows the future across threads.
    pub fn in_poll<T>(self: &Arc<Self>, poll: impl FnOnce() -> T) -> T {
        struct Restore(Option<Arc<PgQueryProfile>>);
        impl Drop for Restore {
            fn drop(&mut self) {
                PROFILE.set(self.0.take());
            }
        }
        let _restore = Restore(PROFILE.replace(Some(Arc::clone(self))));
        poll()
    }

    pub fn has_pending_statement(&self) -> bool {
        self.active.load(Relaxed) != 0
    }

    pub fn totals(&self) -> (u64, Duration) {
        (
            self.statements.load(Relaxed),
            Duration::from_nanos(self.elapsed_ns.load(Relaxed)),
        )
    }
}

pub(crate) struct StatementProfile {
    profile: Arc<PgQueryProfile>,
    started: Instant,
}

impl StatementProfile {
    pub(crate) fn start() -> Option<Self> {
        let profile = PROFILE.with(|slot| {
            let profile = slot.take();
            let current = profile.clone();
            slot.set(profile);
            current
        })?;
        profile.statements.fetch_add(1, Relaxed);
        profile.active.fetch_add(1, Relaxed);
        Some(Self {
            profile,
            started: Instant::now(),
        })
    }
}

impl Drop for StatementProfile {
    fn drop(&mut self) {
        self.profile.elapsed_ns.fetch_add(
            u64::try_from(self.started.elapsed().as_nanos()).unwrap_or(u64::MAX),
            Relaxed,
        );
        self.profile.active.fetch_sub(1, Relaxed);
    }
}
