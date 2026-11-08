//! Bounded admission for foreground owners between separate semantic passes.
use super::{DurabilityError, DurabilityRoot, IssuedSemanticPass, ROOT_RECOVERY_WINDOW};
use crate::durability::SqlxFailureClass;

impl DurabilityRoot {
    /// Recover or wait for the holder before issuing work. A previous successful
    /// pass can retire its connection during cleanup. Maintenance remains the
    /// only connector, and the issued pass keeps its ordinary independent budget.
    pub async fn issue_ready_semantic_pass(&self) -> Result<IssuedSemanticPass, DurabilityError> {
        let deadline = tokio::time::Instant::now() + ROOT_RECOVERY_WINDOW;
        loop {
            if self.pool.is_closed() {
                return Err(DurabilityError::RootUnavailable);
            }
            if tokio::time::Instant::now() >= deadline {
                if self.pool.size() == 0 {
                    self.request_ready();
                }
                return Err(DurabilityError::RootUnavailable);
            }
            match self.try_issue_semantic_pass() {
                Ok(pass) => return Ok(pass),
                Err(DurabilityError::RootUnavailable) => {}
                Err(error) => return Err(error),
            }
            if self.pool.size() == 0 {
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
            } else {
                let mut returned = std::pin::pin!(self.holder_returned.notified());
                returned.as_mut().enable();
                if self.is_ready() {
                    continue;
                }
                if tokio::time::timeout_at(deadline, returned).await.is_err() {
                    if self.pool.size() == 0 {
                        self.request_ready();
                    }
                    return Err(DurabilityError::RootUnavailable);
                }
            }
            tokio::task::yield_now().await;
        }
    }
}
