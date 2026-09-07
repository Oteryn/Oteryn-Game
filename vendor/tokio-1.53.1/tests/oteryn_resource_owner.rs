#![cfg(feature = "rt")]

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};

use tokio::task::{spawn_blocking_owned, BlockingOwner, BlockingOwnerConfig, OwnedSpawnError};

struct Deny;

impl BlockingOwner for Deny {
    fn try_reserve(&self, _bytes: usize) -> bool {
        false
    }

    fn release(&self, _bytes: usize) {}
}

#[derive(Default)]
struct Witness {
    held: AtomicUsize,
    peak: AtomicUsize,
}

impl BlockingOwner for Witness {
    fn try_reserve(&self, bytes: usize) -> bool {
        let held = self.held.fetch_add(bytes, Ordering::SeqCst) + bytes;
        self.peak.fetch_max(held, Ordering::SeqCst);
        true
    }

    fn release(&self, bytes: usize) {
        self.held.fetch_sub(bytes, Ordering::SeqCst);
    }
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
    assert!(matches!(
        result,
        Err(OwnedSpawnError::InsufficientOwnerBalance)
    ));
}

#[test]
fn funded_job_runs_and_releases_after_runtime_shutdown() {
    let owner = Arc::new(Witness::default());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let handle = runtime
        .block_on(async {
            spawn_blocking_owned(
                owner.clone(),
                BlockingOwnerConfig::new(8, 2 * 1024 * 1024),
                || 42,
            )
        })
        .unwrap();
    assert_eq!(runtime.block_on(handle).unwrap(), 42);
    assert!(
        owner.held.load(Ordering::SeqCst) > 0,
        "queue and idle worker stay charged"
    );
    drop(runtime);
    assert_eq!(owner.held.load(Ordering::SeqCst), 0);
    assert!(owner.peak.load(Ordering::SeqCst) >= 2 * 1024 * 1024);
}

#[test]
fn dropped_handle_keeps_started_job_charged() {
    let owner = Arc::new(Witness::default());
    let gate = Arc::new((Mutex::new(false), Condvar::new()));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let worker_gate = gate.clone();
    let handle = runtime
        .block_on(async {
            spawn_blocking_owned(
                owner.clone(),
                BlockingOwnerConfig::new(8, 2 * 1024 * 1024),
                move || {
                    let (lock, ready) = &*worker_gate;
                    let mut released = lock.lock().unwrap();
                    while !*released {
                        released = ready.wait(released).unwrap();
                    }
                },
            )
        })
        .unwrap();
    drop(handle);
    assert!(owner.held.load(Ordering::SeqCst) > 0);
    let (lock, ready) = &*gate;
    *lock.lock().unwrap() = true;
    ready.notify_one();
    drop(runtime);
    assert_eq!(owner.held.load(Ordering::SeqCst), 0);
}

#[test]
fn overflow_fails_closed() {
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let result = runtime.block_on(async {
        spawn_blocking_owned(
            Arc::new(Witness::default()),
            BlockingOwnerConfig::new(usize::MAX, usize::MAX),
            || (),
        )
    });
    assert!(matches!(result, Err(OwnedSpawnError::AccountingOverflow)));
}
