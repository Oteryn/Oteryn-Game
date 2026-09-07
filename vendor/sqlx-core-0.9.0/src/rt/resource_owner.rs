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

/// Admit blocking work only through the resource-owned Tokio path.
///
/// The configured Game graph enables Tokio. If execution is dispatched through
/// another compiled runtime, fail closed rather than falling back to its
/// unowned blocking executor.
pub(crate) fn spawn_blocking_owned<F, R>(
    budget: Arc<dyn ResourceBudget>,
    func: F,
) -> Result<tokio::task::JoinHandle<R>, tokio::task::OwnedSpawnError>
where
    F: FnOnce() -> R + Send + 'static,
    R: Send + 'static,
{
    tokio::runtime::Handle::try_current()
        .map_err(|_| tokio::task::OwnedSpawnError::RuntimeShuttingDown)?;
    tokio::task::spawn_blocking_owned(
        Arc::new(BudgetOwner(budget)),
        tokio::task::BlockingOwnerConfig::new(OWNED_QUEUE_CAPACITY, OWNED_WORKER_STACK),
        func,
    )
}
