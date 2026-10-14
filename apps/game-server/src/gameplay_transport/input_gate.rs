//! The shutdown quiesce of admissions and player input (OPS-SAVE-2, server save and graceful
//! restart).
//!
//! Admissions, resumes and every player command run under a shared permit. The shutdown flush
//! closes the gate first and then waits for the permits in flight, so the last-position, monk and
//! training snapshots it takes afterwards cannot be overtaken by a command or by a session that
//! begins meanwhile. A closed gate refuses new work; it never reopens (a stopping node is not
//! resumed).

use std::sync::atomic::{AtomicBool, Ordering};

use tokio::sync::{RwLock, RwLockReadGuard};

/// Held while one admission or one command runs; releases the gate's drain on drop.
pub(crate) struct InputPermit<'a>(#[allow(dead_code)] Option<RwLockReadGuard<'a, ()>>);

impl InputPermit<'_> {
    /// A permit of an authority with no gate (transport-only fixtures).
    pub(crate) const fn ungated() -> Self {
        Self(None)
    }
}

#[derive(Debug, Default)]
pub(crate) struct InputGate {
    closed: AtomicBool,
    running: RwLock<()>,
}

impl InputGate {
    /// A permit to run one admission or command, or `None` once the gate is closed. The closed
    /// flag is read under the permit, so a permit granted before [`Self::quiesce`] returns is
    /// always drained by it.
    pub(crate) async fn enter(&self) -> Option<InputPermit<'_>> {
        // A closed gate refuses at once instead of queuing behind the drain.
        if self.closed.load(Ordering::Acquire) {
            return None;
        }
        let guard = self.running.read().await;
        if self.closed.load(Ordering::Acquire) {
            return None;
        }
        Some(InputPermit(Some(guard)))
    }

    /// Close the gate, then wait until every permit granted before is released. New work is
    /// refused from the first instruction on; the node bounds this wait with its flush budget.
    pub(crate) async fn quiesce(&self) {
        self.closed.store(true, Ordering::Release);
        drop(self.running.write().await);
    }
}

#[cfg(test)]
#[allow(clippy::expect_used)]
mod tests {
    use std::time::Duration;

    use super::InputGate;

    fn run(test: impl std::future::Future<Output = ()>) {
        tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .expect("runtime")
            .block_on(test);
    }

    #[test]
    fn open_gate_grants_concurrent_permits() {
        run(async {
            let gate = InputGate::default();
            let first = gate.enter().await;
            let second = gate.enter().await;
            assert!(first.is_some() && second.is_some());
        });
    }

    #[test]
    fn quiesce_waits_for_work_in_flight_then_refuses_new_work() {
        run(async {
            let gate = InputGate::default();
            let in_flight = gate.enter().await.expect("open gate");
            let mut quiesced = std::pin::pin!(gate.quiesce());
            // The drain is pending while a command runs.
            assert!(
                tokio::time::timeout(Duration::from_millis(50), quiesced.as_mut())
                    .await
                    .is_err()
            );
            // A command arriving meanwhile is refused, not queued behind the snapshot.
            assert!(gate.enter().await.is_none());
            drop(in_flight);
            tokio::time::timeout(Duration::from_secs(1), quiesced)
                .await
                .expect("drained once the command ended");
            assert!(gate.enter().await.is_none());
        });
    }

    #[test]
    fn quiesce_of_an_idle_gate_returns_at_once() {
        run(async {
            let gate = InputGate::default();
            tokio::time::timeout(Duration::from_secs(1), gate.quiesce())
                .await
                .expect("nothing in flight");
            assert!(gate.enter().await.is_none());
        });
    }
}
