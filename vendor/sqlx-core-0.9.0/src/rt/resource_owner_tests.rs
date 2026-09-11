use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use crate::net::resource_budget::{BudgetError, ResourceBudget};

struct Ledger {
    limit: usize,
    used: AtomicUsize,
}

impl ResourceBudget for Ledger {
    fn try_reserve(&self, bytes: usize) -> Result<(), BudgetError> {
        self.used
            .fetch_update(Ordering::SeqCst, Ordering::SeqCst, |used| {
                used.checked_add(bytes).filter(|next| *next <= self.limit)
            })
            .map(|_| ())
            .map_err(|_| BudgetError::Unavailable)
    }

    fn release(&self, bytes: usize) {
        self.used.fetch_sub(bytes, Ordering::SeqCst);
    }
}

#[test]
fn owned_adapter_denies_before_blocking_owner_arc_allocation() {
    let budget = Arc::new(Ledger {
        limit: 0,
        used: AtomicUsize::new(0),
    });
    let result = super::resource_owner::blocking_job_owner(budget.clone());
    assert!(matches!(result, Err(BudgetError::Unavailable)));
    assert_eq!(budget.used.load(Ordering::SeqCst), 0);
}

#[test]
fn owned_adapter_runs_when_funded_and_releases_on_shutdown() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let budget = Arc::new(Ledger {
        limit: 4 * 1024 * 1024,
        used: AtomicUsize::new(0),
    });
    let owner = super::resource_owner::blocking_job_owner(budget.clone()).unwrap();
    let owner_control_charge = budget.used.load(Ordering::SeqCst);
    assert!(owner_control_charge > 0);
    let handle = runtime
        .block_on(async { super::resource_owner::spawn_blocking_owned(&owner, || 42) })
        .unwrap();
    assert_eq!(runtime.block_on(handle).unwrap(), 42);
    let second = runtime
        .block_on(async { super::resource_owner::spawn_blocking_owned(&owner, || 43) })
        .unwrap();
    assert_eq!(runtime.block_on(second).unwrap(), 43);
    assert!(budget.used.load(Ordering::SeqCst) > owner_control_charge);
    drop(runtime);
    assert_eq!(
        budget.used.load(Ordering::SeqCst),
        owner_control_charge,
        "runtime backing must release before the still-live owner Arc control"
    );
    drop(owner);
    assert_eq!(budget.used.load(Ordering::SeqCst), 0);
}
