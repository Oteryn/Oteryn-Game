//! Bounded admission for foreground owners between separate semantic passes.
use super::{
    DIAGNOSTIC_PROCESS, DurabilityError, DurabilityRoot, HOLDER_RETURN_POLL, IssuedSemanticPass, ROOT_RECOVERY_WINDOW, Site, site_text,
};
use crate::durability::SqlxFailureClass;

impl DurabilityRoot {
    /// Recover or wait for the holder before issuing work. A previous successful
    /// pass can retire its connection during cleanup. Maintenance remains the
    /// only connector, and the issued pass keeps its ordinary independent budget.
    pub async fn issue_ready_semantic_pass(&self) -> Result<IssuedSemanticPass, DurabilityError> {
        let _foreground = self.foreground_demand()?;
        let deadline = tokio::time::Instant::now() + ROOT_RECOVERY_WINDOW;
        // From the first refusal on, a returned holder goes to this owner before owners
        // refused later (see `HolderWaiter`).
        let mut waiter = None;
        loop {
            if self.pool.is_closed() {
                return Err(DurabilityError::RootUnavailable);
            }
            if tokio::time::Instant::now() >= deadline {
                self.request_ready();
                return Err(DurabilityError::RootUnavailable);
            }
            match self.try_issue_semantic_pass() {
                Ok(pass) => return Ok(pass),
                Err(DurabilityError::RootUnavailable) => {}
                Err(error) => return Err(error),
            }
            let _ = waiter.get_or_insert_with(|| self.holder_waiter());
            // A live holder, busy or kept for an older owner: wait for this owner's turn.
            let now = tokio::time::Instant::now();
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
                    return Err(DurabilityError::RootUnavailable);
                }
            }
            tokio::task::yield_now().await;
        }
    }
}

impl DurabilityRoot {
    /// Wait only for a live holder, using the existing checkout and queue. The
    /// checkout bound never cancels SQL once issued.
    #[track_caller]
    pub fn issue_foreground_pass_until(
        &self,
        deadline: tokio::time::Instant,
    ) -> impl Future<Output = Result<IssuedSemanticPass, DurabilityError>> + '_ {
        let caller = std::panic::Location::caller();
        async move {
            let deadline = deadline.min(tokio::time::Instant::now() + ROOT_RECOVERY_WINDOW);
            let mut waiter = None;
            loop {
                if self.pool.is_closed() || self.pool.size() == 0 {
                    self.request_ready();
                    return Err(DurabilityError::RootUnavailable);
                }
                let now = tokio::time::Instant::now();
                if now >= deadline {
                    drop(waiter);
                    self.report_holder_busy(caller);
                    return Err(DurabilityError::HolderBusy);
                }
                match self.try_issue_at(caller) {
                    Ok(pass) => return Ok(pass),
                    Err(DurabilityError::RootUnavailable) => {}
                    Err(error) => return Err(error),
                }
                let _ = waiter.get_or_insert_with(|| self.holder_waiter());
                self.wait_holder_turn(now, (now + HOLDER_RETURN_POLL).min(deadline))
                    .await;
            }
        }
    }

    fn report_holder_busy(&self, waiter: Site) {
        use oteryn_error_codes::{FieldValue, Level, Line};
        let owner = self.holder_owner.lock().ok().and_then(|slot| *slot);
        let line = Line::new(Level::Warn, "durability", "holder_busy_timeout")
            .field("waiter", FieldValue::Owned(site_text(Some(waiter))))
            .field("owner", FieldValue::Owned(site_text(owner.map(|(site, ..)| site))));
        oteryn_error_codes::emit(DIAGNOSTIC_PROCESS, &line);
    }
}

#[cfg(test)]
#[test]
fn holder_min_busy_timeout_log_names_owner() -> Result<(), Box<dyn std::error::Error>> {
    #[track_caller]
    fn site() -> Site {
        std::panic::Location::caller()
    }
    let waiter = site();
    let owner = site();
    let runtime = tokio::runtime::Builder::new_current_thread().enable_all().build()?;
    let _runtime = runtime.enter();
    let root = DurabilityRoot::connect_test_runtime("postgres://oteryn@127.0.0.1:1/oteryn")?;
    let mut slot = root.holder_owner.lock().map_err(|_| "owner slot poisoned")?;
    *slot = Some((owner, std::time::Instant::now(), None));
    drop(slot);
    let lines = oteryn_error_codes::capture(|| root.report_holder_busy(waiter));
    assert_eq!(lines.len(), 1, "{lines:?}");
    assert!(lines[0].contains("event=holder_busy_timeout"), "{lines:?}");
    assert!(lines[0].contains(&format!("owner={owner}")), "{lines:?}");
    assert!(lines[0].contains(&format!("waiter={waiter}")), "{lines:?}");
    Ok(())
}
