use std::alloc::Layout;
use std::sync::atomic::AtomicUsize;
use std::sync::Arc;

use crate::net::resource_budget::{BudgetError, ResourceBudget, ResourceReservation};

const OWNED_QUEUE_CAPACITY: usize = 8;
const OWNED_WORKER_STACK: usize = 2 * 1024 * 1024;

struct BudgetOwner {
    budget: Arc<dyn ResourceBudget>,
    // Acquired before Arc::new; the private runtime handle calls finalize on
    // every strong reference, so this drops after the Arc allocation is freed.
    _control_allocation: ResourceReservation,
}

impl BudgetOwner {
    fn arc_layout() -> Result<usize, BudgetError> {
        Layout::new::<[AtomicUsize; 2]>()
            .extend(Layout::new::<Self>())
            .map(|(layout, _)| layout.pad_to_align().size())
            .map_err(|_| BudgetError::Overflow)
    }
}

impl tokio::task::BlockingOwner for BudgetOwner {
    fn finalize(self: Arc<Self>) {
        // No Weak or raw Arc escapes construction. Exactly one concurrent
        // finalizer receives Self after Arc control-block deallocation.
        drop(Arc::into_inner(self));
    }

    fn try_reserve(&self, bytes: usize) -> bool {
        self.budget.try_reserve(bytes).is_ok()
    }

    fn release(&self, bytes: usize) {
        self.budget.release(bytes);
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
    runtime_owner: tokio::task::OterynBlockingOwner,
}

impl BlockingJobOwner {
    // Activated by the admitted TLS composition step; focused tests exercise it
    // before the currently excluded rustls owner hook is available.
    #[allow(dead_code)]
    fn new(budget: Arc<dyn ResourceBudget>) -> Result<Self, BudgetError> {
        let control_allocation =
            ResourceReservation::try_new(budget.clone(), BudgetOwner::arc_layout()?)?;
        let runtime_owner = Arc::new(BudgetOwner {
            budget: budget.clone(),
            _control_allocation: control_allocation,
        });
        Ok(Self {
            runtime_owner: runtime_owner.into(),
            budget,
        })
    }

    pub(crate) fn budget(&self) -> Arc<dyn ResourceBudget> {
        self.budget.clone()
    }
}

#[allow(dead_code)]
pub(crate) fn blocking_job_owner(
    budget: Arc<dyn ResourceBudget>,
) -> Result<BlockingJobOwner, BudgetError> {
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::Ordering;

    struct Ledger {
        limit: usize,
        held: AtomicUsize,
        releases: AtomicUsize,
    }

    impl ResourceBudget for Ledger {
        fn try_reserve(&self, bytes: usize) -> Result<(), BudgetError> {
            self.held
                .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |held| {
                    held.checked_add(bytes).filter(|next| *next <= self.limit)
                })
                .map(|_| ())
                .map_err(|_| BudgetError::Unavailable)
        }
        fn release(&self, bytes: usize) {
            assert!(self.held.fetch_sub(bytes, Ordering::SeqCst) >= bytes);
            self.releases.fetch_add(1, Ordering::SeqCst);
        }
    }

    fn ledger(limit: usize) -> Arc<Ledger> {
        Arc::new(Ledger {
            limit,
            held: AtomicUsize::new(0),
            releases: AtomicUsize::new(0),
        })
    }

    #[test]
    fn owner_control_exact_and_max_minus_one() {
        let exact = BudgetOwner::arc_layout().unwrap();
        let denied = ledger(exact - 1);
        assert!(matches!(
            BlockingJobOwner::new(denied.clone()),
            Err(BudgetError::Unavailable)
        ));
        assert_eq!(denied.held.load(Ordering::SeqCst), 0);
        assert_eq!(denied.releases.load(Ordering::SeqCst), 0);
        let funded = ledger(exact);
        let owner = BlockingJobOwner::new(funded.clone()).unwrap();
        assert_eq!(funded.held.load(Ordering::SeqCst), exact);
        drop(owner);
        assert_eq!(funded.held.load(Ordering::SeqCst), 0);
        assert_eq!(funded.releases.load(Ordering::SeqCst), 1);
    }

    #[test]
    fn wrapper_drop_keeps_control_live_through_task_and_worker() {
        let budget = ledger(usize::MAX);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap();
        let wrapper = BlockingJobOwner::new(budget.clone()).unwrap();
        let (release, wait) = std::sync::mpsc::channel();
        let job = {
            let _enter = runtime.enter();
            spawn_blocking_owned(&wrapper, move || wait.recv().unwrap()).unwrap()
        };
        drop(wrapper);
        assert!(budget.held.load(Ordering::SeqCst) > BudgetOwner::arc_layout().unwrap());
        release.send(()).unwrap();
        runtime.block_on(job).unwrap();
        // Queue/idle-worker custody may survive the completed task.
        drop(runtime);
        assert_eq!(budget.held.load(Ordering::SeqCst), 0);
    }

    #[test]
    fn wrapper_drop_preserves_same_owner_until_concurrent_descendants_end() {
        let exact = BudgetOwner::arc_layout().unwrap();
        let budget = ledger(exact);
        let wrapper = BlockingJobOwner::new(budget.clone()).unwrap();
        let first = wrapper.runtime_owner.clone();
        let second = wrapper.runtime_owner.clone();
        assert!(first.same_owner(&second));
        drop(wrapper);
        assert_eq!(budget.held.load(Ordering::SeqCst), exact);
        let barrier = std::sync::Barrier::new(2);
        std::thread::scope(|scope| {
            scope.spawn(|| {
                barrier.wait();
                drop(first);
            });
            scope.spawn(|| {
                barrier.wait();
                drop(second);
            });
        });
        assert_eq!(budget.held.load(Ordering::SeqCst), 0);
        assert_eq!(budget.releases.load(Ordering::SeqCst), 1);
    }
}
