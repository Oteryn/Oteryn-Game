//! Bounded admission for foreground owners between separate semantic passes.
use super::{
    DurabilityError, DurabilityRoot, HOLDER_RETURN_POLL, IssuedSemanticPass, ROOT_RECOVERY_WINDOW,
};
use crate::durability::SqlxFailureClass;
use tokio::time::Instant;

impl DurabilityRoot {
    /// Retry a foreground step whose typed refusal guarantees that it did no work.
    /// Keep its place across wakeups so later checkouts cannot race it for each return.
    pub async fn retry_foreground_holder<T, E>(
        &self,
        deadline: tokio::time::Instant,
        busy: impl Fn(&E) -> bool,
        mut step: impl AsyncFnMut() -> Result<T, E>,
    ) -> Result<T, E> {
        let mut waiting = None;
        loop {
            let outcome = step().await;
            let now = tokio::time::Instant::now();
            if !outcome.as_ref().is_err_and(&busy) || now >= deadline {
                return outcome;
            }
            // Scheduled passes still get their bounded fairness window. If the guard
            // capacity is full, the FIFO ticket continues to order foreground owners.
            waiting.get_or_insert_with(|| (self.foreground_demand().ok(), self.holder_waiter()));
            let latest = (now + std::time::Duration::from_millis(250)).min(deadline);
            let earliest = (now + std::time::Duration::from_millis(50)).min(latest);
            if !self.wait_holder_turn(earliest, latest).await {
                // A retired holder has no turn. Wait for maintenance within the same budget.
                self.wait_holder_retry(earliest, latest).await;
            }
            if tokio::time::Instant::now() >= deadline {
                return outcome;
            }
        }
    }

    /// Recover or wait for the holder before issuing work. A previous successful
    /// pass can retire its connection during cleanup. Maintenance remains the
    /// only connector, and the issued pass keeps its ordinary independent budget.
    pub async fn issue_ready_semantic_pass(&self) -> Result<IssuedSemanticPass, DurabilityError> {
        self.issue_ready_pass(Instant::now() + ROOT_RECOVERY_WINDOW, false)
            .await
    }

    /// Spend one recovery budget on both checkout and execution.
    pub(in crate::durability) async fn issue_ready_semantic_pass_by(
        &self,
        deadline: Instant,
    ) -> Result<IssuedSemanticPass, DurabilityError> {
        self.issue_ready_pass(deadline, true).await
    }

    async fn issue_ready_pass(
        &self,
        deadline: Instant,
        cap_pass: bool,
    ) -> Result<IssuedSemanticPass, DurabilityError> {
        let _foreground = self.foreground_demand()?;
        // From the first refusal on, a returned holder goes to this owner before owners
        // refused later (see `HolderWaiter`).
        let mut waiter = None;
        loop {
            if self.pool.is_closed() {
                return Err(DurabilityError::RootUnavailable);
            }
            if Instant::now() >= deadline {
                self.request_ready();
                return Err(if cap_pass && self.pool.size() > 0 {
                    DurabilityError::RootPassDeadlineExceeded
                } else {
                    DurabilityError::RootUnavailable
                });
            }
            let issued = if cap_pass {
                self.try_issue_semantic_pass_by(deadline.into_std())
            } else {
                self.try_issue_semantic_pass()
            };
            match issued {
                Ok(pass) => return Ok(pass),
                Err(DurabilityError::RootUnavailable) => {}
                Err(error) => return Err(error),
            }
            let _ = waiter.get_or_insert_with(|| self.holder_waiter());
            // A live holder, busy or kept for an older owner: wait for this owner's turn.
            let now = Instant::now();
            if self
                .wait_holder_turn(now, (now + HOLDER_RETURN_POLL).min(deadline))
                .await
            {
                continue;
            }
            match tokio::time::timeout_at(deadline, self.await_ready()).await {
                Ok(Ok(()))
                | Ok(Err(DurabilityError::RootUnavailable))
                | Ok(Err(DurabilityError::Database(SqlxFailureClass::Pool))) => {}
                Ok(Err(error)) => return Err(error),
                Err(_) => {
                    // Cancelling maintenance must not lose its recovery demand.
                    self.request_ready();
                    continue;
                }
            }
            tokio::task::yield_now().await;
        }
    }
}
