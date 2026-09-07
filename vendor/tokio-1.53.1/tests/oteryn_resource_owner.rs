#![cfg(feature = "rt")]

use std::sync::Arc;

use tokio::task::{spawn_blocking_owned, BlockingOwner, BlockingOwnerConfig, OwnedSpawnError};

struct Deny;

impl BlockingOwner for Deny {
    fn try_reserve(&self, _bytes: usize) -> bool {
        false
    }

    fn release(&self, _bytes: usize) {}
}

#[test]
fn denial_precedes_task_and_queue_allocation() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let result = runtime.block_on(async {
        spawn_blocking_owned(
            Arc::new(Deny),
            BlockingOwnerConfig::new(8, 2 * 1024 * 1024),
            || 1usize,
        )
    });
    assert!(matches!(result, Err(OwnedSpawnError::InsufficientOwnerBalance)));
}
