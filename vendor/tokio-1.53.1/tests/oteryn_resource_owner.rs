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
    reservations: AtomicUsize,
    releases: AtomicUsize,
    force_spawn_failure: bool,
}

impl BlockingOwner for Witness {
    fn try_reserve(&self, bytes: usize) -> bool {
        self.reservations.fetch_add(1, Ordering::SeqCst);
        let held = self.held.fetch_add(bytes, Ordering::SeqCst) + bytes;
        self.peak.fetch_max(held, Ordering::SeqCst);
        true
    }

    fn release(&self, bytes: usize) {
        self.releases.fetch_add(1, Ordering::SeqCst);
        self.held.fetch_sub(bytes, Ordering::SeqCst);
    }

    fn force_thread_spawn_failure(&self) -> bool {
        self.force_spawn_failure
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

#[test]
fn full_owner_queue_denies_before_task_reservation_and_never_spills() {
    let owner = Arc::new(Witness::default());
    let gate = Arc::new((Mutex::new((false, false)), Condvar::new()));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    let first_gate = gate.clone();
    let first = runtime
        .block_on(async {
            spawn_blocking_owned(
                owner.clone(),
                BlockingOwnerConfig::new(1, 2 * 1024 * 1024),
                move || {
                    let (lock, ready) = &*first_gate;
                    let mut state = lock.lock().unwrap();
                    state.0 = true;
                    ready.notify_one();
                    while !state.1 {
                        state = ready.wait(state).unwrap();
                    }
                },
            )
        })
        .unwrap();
    {
        let (lock, ready) = &*gate;
        let mut state = lock.lock().unwrap();
        while !state.0 {
            state = ready.wait(state).unwrap();
        }
    }
    let queued = runtime
        .block_on(async {
            spawn_blocking_owned(
                owner.clone(),
                BlockingOwnerConfig::new(1, 2 * 1024 * 1024),
                || (),
            )
        })
        .unwrap();
    let reservations = owner.reservations.load(Ordering::SeqCst);
    let denied = runtime.block_on(async {
        spawn_blocking_owned(
            owner.clone(),
            BlockingOwnerConfig::new(1, 2 * 1024 * 1024),
            || panic!("full owned queue spilled into a backend queue"),
        )
    });
    assert!(matches!(denied, Err(OwnedSpawnError::QueueFull)));
    assert_eq!(owner.reservations.load(Ordering::SeqCst), reservations);

    let (lock, ready) = &*gate;
    lock.lock().unwrap().1 = true;
    ready.notify_one();
    runtime.block_on(first).unwrap();
    runtime.block_on(queued).unwrap();
    drop(runtime);
    assert_eq!(owner.held.load(Ordering::SeqCst), 0);
}

#[test]
fn queued_abort_releases_only_after_queue_removal_and_task_destruction() {
    let owner = Arc::new(Witness::default());
    let gate = Arc::new((Mutex::new((false, false)), Condvar::new()));
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let first_gate = gate.clone();
    let first = runtime
        .block_on(async {
            spawn_blocking_owned(
                owner.clone(),
                BlockingOwnerConfig::new(1, 2 * 1024 * 1024),
                move || {
                    let (lock, ready) = &*first_gate;
                    let mut state = lock.lock().unwrap();
                    state.0 = true;
                    ready.notify_one();
                    while !state.1 {
                        state = ready.wait(state).unwrap();
                    }
                },
            )
        })
        .unwrap();
    {
        let (lock, ready) = &*gate;
        let mut state = lock.lock().unwrap();
        while !state.0 {
            state = ready.wait(state).unwrap();
        }
    }
    let queued = runtime
        .block_on(async {
            spawn_blocking_owned(
                owner.clone(),
                BlockingOwnerConfig::new(1, 2 * 1024 * 1024),
                || (),
            )
        })
        .unwrap();
    let held_while_queued = owner.held.load(Ordering::SeqCst);
    queued.abort();
    assert_eq!(owner.held.load(Ordering::SeqCst), held_while_queued);

    let (lock, ready) = &*gate;
    lock.lock().unwrap().1 = true;
    ready.notify_one();
    runtime.block_on(first).unwrap();
    assert!(runtime.block_on(queued).unwrap_err().is_cancelled());
    assert!(owner.held.load(Ordering::SeqCst) < held_while_queued);
    drop(runtime);
    assert_eq!(owner.held.load(Ordering::SeqCst), 0);
}

#[test]
fn forced_worker_spawn_failure_releases_never_created_backing_once() {
    let owner = Arc::new(Witness {
        force_spawn_failure: true,
        ..Witness::default()
    });
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();
    let result = runtime.block_on(async {
        spawn_blocking_owned(
            owner.clone(),
            BlockingOwnerConfig::new(8, 2 * 1024 * 1024),
            || (),
        )
    });
    assert!(matches!(result, Err(OwnedSpawnError::ThreadSpawn(_))));
    assert_eq!(owner.reservations.load(Ordering::SeqCst), 4);
    assert_eq!(owner.releases.load(Ordering::SeqCst), 4);
    assert_eq!(owner.held.load(Ordering::SeqCst), 0);
    drop(runtime);
    assert_eq!(owner.releases.load(Ordering::SeqCst), 4);
    assert_eq!(owner.held.load(Ordering::SeqCst), 0);
}

#[test]
fn distinct_operation_owners_share_runtime_without_cross_charging() {
    let first = Arc::new(Witness::default());
    let second = Arc::new(Witness::default());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .build()
        .unwrap();

    let first_job = runtime
        .block_on(async {
            spawn_blocking_owned(
                first.clone(),
                BlockingOwnerConfig::new(8, 2 * 1024 * 1024),
                || 1,
            )
        })
        .unwrap();
    assert_eq!(runtime.block_on(first_job).unwrap(), 1);

    let second_job = runtime
        .block_on(async {
            spawn_blocking_owned(
                second.clone(),
                BlockingOwnerConfig::new(8, 2 * 1024 * 1024),
                || 2,
            )
        })
        .unwrap();
    assert_eq!(runtime.block_on(second_job).unwrap(), 2);
    assert!(first.held.load(Ordering::SeqCst) > 0);
    assert!(second.held.load(Ordering::SeqCst) > 0);

    drop(runtime);
    assert_eq!(first.held.load(Ordering::SeqCst), 0);
    assert_eq!(second.held.load(Ordering::SeqCst), 0);
}

#[test]
fn concurrent_owned_admission_releases_every_reservation() {
    let owner = Arc::new(Witness::default());
    let runtime = Arc::new(
        tokio::runtime::Builder::new_current_thread()
            .build()
            .unwrap(),
    );
    let mut submitters = Vec::new();
    for value in 0..8 {
        let owner = owner.clone();
        let runtime = runtime.clone();
        submitters.push(std::thread::spawn(move || {
            runtime
                .block_on(async {
                    spawn_blocking_owned(
                        owner,
                        BlockingOwnerConfig::new(8, 2 * 1024 * 1024),
                        move || value,
                    )
                })
                .unwrap()
        }));
    }
    let handles: Vec<_> = submitters
        .into_iter()
        .map(|thread| thread.join().unwrap())
        .collect();
    for (value, handle) in handles.into_iter().enumerate() {
        assert_eq!(runtime.block_on(handle).unwrap(), value);
    }
    drop(runtime);
    assert_eq!(owner.held.load(Ordering::SeqCst), 0);
    assert_eq!(
        owner.reservations.load(Ordering::SeqCst),
        owner.releases.load(Ordering::SeqCst)
    );
}

#[test]
fn owned_worker_charge_survives_stop_until_external_join() {
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    let owner = Arc::new(Witness::default());
    let stopped = Arc::new(AtomicBool::new(false));
    let held_at_stop = Arc::new(AtomicUsize::new(0));
    let stop_owner = owner.clone();
    let stop_flag = stopped.clone();
    let stop_held = held_at_stop.clone();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .thread_keep_alive(Duration::from_millis(5))
        .on_thread_stop(move || {
            stop_held.store(stop_owner.held.load(Ordering::SeqCst), Ordering::SeqCst);
            stop_flag.store(true, Ordering::SeqCst);
        })
        .build()
        .unwrap();

    let handle = runtime
        .block_on(async {
            spawn_blocking_owned(
                owner.clone(),
                BlockingOwnerConfig::new(8, 2 * 1024 * 1024),
                || 7usize,
            )
        })
        .unwrap();
    assert_eq!(runtime.block_on(handle).unwrap(), 7);

    for _ in 0..200 {
        if stopped.load(Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(stopped.load(Ordering::SeqCst));
    let retained = held_at_stop.load(Ordering::SeqCst);
    assert!(retained >= 2 * 1024 * 1024);
    std::thread::sleep(Duration::from_millis(20));
    assert_eq!(owner.held.load(Ordering::SeqCst), retained);

    drop(runtime);
    assert_eq!(owner.held.load(Ordering::SeqCst), 0);
}

#[test]
fn shutdown_timeout_never_releases_live_owned_worker_charge() {
    use std::time::Duration;

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

    let release_gate = gate.clone();
    let releaser = std::thread::spawn(move || {
        std::thread::sleep(Duration::from_millis(100));
        let (lock, ready) = &*release_gate;
        *lock.lock().unwrap() = true;
        ready.notify_one();
    });

    runtime.shutdown_timeout(Duration::from_millis(10));
    assert!(owner.held.load(Ordering::SeqCst) > 0);
    releaser.join().unwrap();
    std::thread::sleep(Duration::from_millis(100));
    assert!(
        owner.held.load(Ordering::SeqCst) > 0,
        "timed shutdown must conservatively retain the leaked worker debit"
    );
}

#[test]
fn finished_owned_worker_reaps_before_reusing_thread_cap() {
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    let owner = Arc::new(Witness::default());
    let stopped = Arc::new(AtomicBool::new(false));
    let stop_flag = stopped.clone();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .thread_keep_alive(Duration::from_millis(5))
        .on_thread_stop(move || {
            stop_flag.store(true, Ordering::SeqCst);
        })
        .build()
        .unwrap();

    let owned = runtime
        .block_on(async {
            spawn_blocking_owned(
                owner.clone(),
                BlockingOwnerConfig::new(8, 2 * 1024 * 1024),
                || 11usize,
            )
        })
        .unwrap();
    assert_eq!(runtime.block_on(owned).unwrap(), 11);
    for _ in 0..200 {
        if stopped.load(Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(stopped.load(Ordering::SeqCst));
    assert!(owner.held.load(Ordering::SeqCst) >= 2 * 1024 * 1024);

    let ordinary = runtime.block_on(async {
        let ordinary = tokio::task::spawn_blocking(|| 13usize);
        assert_eq!(
            owner.held.load(Ordering::SeqCst),
            0,
            "ordinary admission must reap the finished owned worker before reopening the cap"
        );
        ordinary.await.unwrap()
    });
    assert_eq!(ordinary, 13);
    drop(runtime);
}
