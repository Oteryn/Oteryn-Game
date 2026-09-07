use std::sync::Arc;

use crate::net::resource_budget::ResourceBudget;

const OWNED_QUEUE_CAPACITY: usize = 8;
const OWNED_WORKER_STACK: usize = 2 * 1024 * 1024;

struct BudgetOwner(Arc<dyn ResourceBudget>);

impl tokio::task::BlockingOwner for BudgetOwner {
    fn try_reserve(&self, bytes: usize) -> bool {
        self.0.try_reserve(bytes).is_ok()
    }

    fn release(&self, bytes: usize) {
        self.0.release(bytes);
    }
}

/// Reusable operation capability for the resource-owned Tokio path.
///
/// Tokio's separately charged owner queue compares owner identity. Retaining
/// this Arc across all certificate loads in one operation prevents later loads
/// from being incorrectly rejected as a different queue owner.
#[derive(Clone)]
pub(crate) struct BlockingJobOwner {
    budget: Arc<dyn ResourceBudget>,
    runtime_owner: Arc<BudgetOwner>,
}

impl BlockingJobOwner {
    // Activated by the admitted TLS composition step; focused tests exercise it
    // before the currently excluded rustls owner hook is available.
    #[allow(dead_code)]
    fn new(budget: Arc<dyn ResourceBudget>) -> Self {
        Self {
            runtime_owner: Arc::new(BudgetOwner(budget.clone())),
            budget,
        }
    }

    pub(crate) fn budget(&self) -> Arc<dyn ResourceBudget> {
        self.budget.clone()
    }
}

#[allow(dead_code)]
pub(crate) fn blocking_job_owner(budget: Arc<dyn ResourceBudget>) -> BlockingJobOwner {
    BlockingJobOwner::new(budget)
}

/// Admit blocking work only through the resource-owned Tokio path.
///
/// The configured Game graph enables Tokio. If execution is dispatched through
/// another compiled runtime, fail closed rather than falling back to its
/// unowned blocking executor.
pub(crate) fn spawn_blocking_owned<F, R>(
    owner: &BlockingJobOwner,
    func: F,
) -> Result<tokio::task::JoinHandle<R>, tokio::task::OwnedSpawnError>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    tokio::runtime::Handle::try_current()
        .map_err(|_| tokio::task::OwnedSpawnError::RuntimeShuttingDown)?;
    tokio::task::spawn_blocking_owned(
        owner.runtime_owner.clone(),
        tokio::task::BlockingOwnerConfig::new(OWNED_QUEUE_CAPACITY, OWNED_WORKER_STACK),
        func,
    )
}
