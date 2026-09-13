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

#[test]
fn owned_thread_cap_stays_closed_until_external_join() {
    use std::time::Duration;

    let first_owner = Arc::new(Witness::default());
    let second_owner = Arc::new(Witness::default());
    let gate = Arc::new((Mutex::new((false, false)), Condvar::new()));
    let stop_gate = gate.clone();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .thread_keep_alive(Duration::from_millis(5))
        .on_thread_stop(move || {
            let (lock, ready) = &*stop_gate;
            let mut state = lock.lock().unwrap();
            state.0 = true;
            ready.notify_all();
            while !state.1 {
                state = ready.wait(state).unwrap();
            }
        })
        .build()
        .unwrap();

    let first = runtime
        .block_on(async {
            spawn_blocking_owned(
                first_owner.clone(),
                BlockingOwnerConfig::new(8, 2 * 1024 * 1024),
                || 1usize,
            )
        })
        .unwrap();
    assert_eq!(runtime.block_on(first).unwrap(), 1);
    {
        let (lock, ready) = &*gate;
        let mut state = lock.lock().unwrap();
        while !state.0 {
            state = ready.wait(state).unwrap();
        }
    }

    let denied = runtime.block_on(async {
        spawn_blocking_owned(
            second_owner.clone(),
            BlockingOwnerConfig::new(8, 2 * 1024 * 1024),
            || 2usize,
        )
    });
    assert!(matches!(denied, Err(OwnedSpawnError::ThreadSpawn(_))));
    assert_eq!(second_owner.reservations.load(Ordering::SeqCst), 0);
    assert!(first_owner.held.load(Ordering::SeqCst) >= 2 * 1024 * 1024);

    {
        let (lock, ready) = &*gate;
        lock.lock().unwrap().1 = true;
        ready.notify_all();
    }
    let second = loop {
        let result = runtime.block_on(async {
            spawn_blocking_owned(
                second_owner.clone(),
                BlockingOwnerConfig::new(8, 2 * 1024 * 1024),
                || 2usize,
            )
        });
        match result {
            Ok(handle) => break handle,
            Err(OwnedSpawnError::ThreadSpawn(_)) => {
                std::thread::sleep(Duration::from_millis(5));
            }
            Err(error) => panic!("unexpected owned admission failure: {error}"),
        }
    };
    assert_eq!(first_owner.held.load(Ordering::SeqCst), 0);
    assert_eq!(runtime.block_on(second).unwrap(), 2);
    drop(runtime);
    assert_eq!(second_owner.held.load(Ordering::SeqCst), 0);
}

#[test]
fn owned_to_ordinary_handoff_retains_worker_charge_until_final_join() {
    use std::sync::atomic::AtomicBool;
    use std::time::Duration;

    let owner = Arc::new(Witness::default());
    let task_gate = Arc::new((Mutex::new((false, false)), Condvar::new()));
    let stopped = Arc::new(AtomicBool::new(false));
    let held_at_stop = Arc::new(AtomicUsize::new(0));
    let stop_owner = owner.clone();
    let stop_flag = stopped.clone();
    let stop_held = held_at_stop.clone();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .max_blocking_threads(1)
        .thread_keep_alive(Duration::from_millis(5))
        .on_thread_stop(move || {
            stop_held.store(stop_owner.held.load(Ordering::SeqCst), Ordering::SeqCst);
            stop_flag.store(true, Ordering::SeqCst);
        })
        .build()
        .unwrap();

    let worker_gate = task_gate.clone();
    let owned = runtime
        .block_on(async {
            spawn_blocking_owned(
                owner.clone(),
                BlockingOwnerConfig::new(8, 2 * 1024 * 1024),
                move || {
                    let (lock, ready) = &*worker_gate;
                    let mut state = lock.lock().unwrap();
                    state.0 = true;
                    ready.notify_all();
                    while !state.1 {
                        state = ready.wait(state).unwrap();
                    }
                    17usize
                },
            )
        })
        .unwrap();

    {
        let (lock, ready) = &*task_gate;
        let mut state = lock.lock().unwrap();
        while !state.0 {
            state = ready.wait(state).unwrap();
        }
    }

    let ordinary = {
        let _enter = runtime.enter();
        tokio::task::spawn_blocking(|| 19usize)
    };

    {
        let (lock, ready) = &*task_gate;
        lock.lock().unwrap().1 = true;
        ready.notify_all();
    }

    assert_eq!(runtime.block_on(owned).unwrap(), 17);
    assert_eq!(runtime.block_on(ordinary).unwrap(), 19);
    for _ in 0..200 {
        if stopped.load(Ordering::SeqCst) {
            break;
        }
        std::thread::sleep(Duration::from_millis(5));
    }
    assert!(stopped.load(Ordering::SeqCst));
    let retained = held_at_stop.load(Ordering::SeqCst);
    assert!(retained >= 2 * 1024 * 1024);
    assert_eq!(owner.held.load(Ordering::SeqCst), retained);

    drop(runtime);
    assert_eq!(owner.held.load(Ordering::SeqCst), 0);
}

#[cfg(feature = "net")]
#[test]
fn oteryn_io_registration_owner_tcp_custody_and_denial() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let owner = Arc::new(Witness::default());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let stream = tokio::net::TcpStream::connect_addr_oteryn_owned(addr, owner.clone())
            .await
            .unwrap();
        assert_eq!(owner.reservations.load(Ordering::SeqCst), 1);
        assert!(owner.held.load(Ordering::SeqCst) > 0);
        drop(stream);
        // Dropping the socket only enqueues driver cleanup.
        assert!(owner.held.load(Ordering::SeqCst) > 0);
        assert_eq!(owner.releases.load(Ordering::SeqCst), 0);
        assert!(
            tokio::net::TcpStream::connect_addr_oteryn_owned(addr, Arc::new(Deny))
                .await
                .is_err()
        );
    });
    drop(runtime);
    assert_eq!(owner.held.load(Ordering::SeqCst), 0);
    assert_eq!(owner.releases.load(Ordering::SeqCst), 1);
}

#[cfg(all(feature = "net", unix))]
#[test]
fn oteryn_io_registration_owner_unix_custody_and_denial() {
    let path = std::env::temp_dir().join(format!("oteryn-e2-{}", std::process::id()));
    let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
    let owner = Arc::new(Witness::default());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let stream = tokio::net::UnixStream::connect_oteryn_owned(&path, owner.clone())
            .await
            .unwrap();
        assert_eq!(owner.reservations.load(Ordering::SeqCst), 1);
        drop(stream);
        assert!(owner.held.load(Ordering::SeqCst) > 0);
        assert_eq!(owner.releases.load(Ordering::SeqCst), 0);
        assert!(
            tokio::net::UnixStream::connect_oteryn_owned(&path, Arc::new(Deny))
                .await
                .is_err()
        );
    });
    drop(runtime);
    assert_eq!(owner.held.load(Ordering::SeqCst), 0);
    assert_eq!(owner.releases.load(Ordering::SeqCst), 1);
    drop(listener);
    std::fs::remove_file(path).unwrap();
}

#[cfg(feature = "net")]
#[test]
fn oteryn_io_registration_owner_tcp_exact_backing_finality() {
    use std::alloc::{GlobalAlloc, Layout, System};
    use std::cell::Cell;
    use std::ptr;
    use std::sync::atomic::{AtomicBool, AtomicPtr};

    // Only this test arms tracking, on its own thread, from try_reserve.
    // Other test allocations continue directly through System.
    thread_local! { static ARMED: Cell<usize> = const { Cell::new(0) }; }
    static BACKING: AtomicPtr<u8> = AtomicPtr::new(ptr::null_mut());
    static DEALLOCATED: AtomicBool = AtomicBool::new(false);
    static CONTENT_DROPPED: AtomicBool = AtomicBool::new(true);
    static REQUESTED: AtomicUsize = AtomicUsize::new(0);
    struct Allocator;
    #[global_allocator]
    static ALLOCATOR: Allocator = Allocator;
    unsafe impl GlobalAlloc for Allocator {
        unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
            let ptr = unsafe { System.alloc(layout) };
            let track = ARMED
                .try_with(|armed| {
                    if armed.get() == layout.size() {
                        armed.set(0);
                        true
                    } else {
                        false
                    }
                })
                .unwrap_or(false);
            if track && !ptr.is_null() {
                REQUESTED.store(layout.size(), Ordering::SeqCst);
                BACKING.store(ptr, Ordering::SeqCst);
            }
            ptr
        }
        unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
            let tracked = BACKING
                .compare_exchange(ptr, ptr::null_mut(), Ordering::SeqCst, Ordering::SeqCst)
                .is_ok();
            unsafe { System.dealloc(ptr, layout) };
            if tracked {
                DEALLOCATED.store(true, Ordering::SeqCst);
            }
        }
    }
    struct Owner {
        limit: usize,
        held: AtomicUsize,
        requested: AtomicUsize,
        releases: AtomicUsize,
    }
    impl Owner {
        fn new(limit: usize) -> Arc<Self> {
            assert!(BACKING.load(Ordering::SeqCst).is_null());
            DEALLOCATED.store(false, Ordering::SeqCst);
            REQUESTED.store(0, Ordering::SeqCst);
            Arc::new(Self {
                limit,
                held: AtomicUsize::new(0),
                requested: AtomicUsize::new(0),
                releases: AtomicUsize::new(0),
            })
        }
    }
    impl BlockingOwner for Owner {
        fn try_reserve(&self, bytes: usize) -> bool {
            assert_eq!(self.requested.swap(bytes, Ordering::SeqCst), 0);
            if bytes > self.limit {
                return false;
            }
            self.held.store(bytes, Ordering::SeqCst);
            ARMED.with(|armed| armed.set(bytes));
            true
        }
        fn release(&self, bytes: usize) {
            assert!(
                DEALLOCATED.load(Ordering::SeqCst),
                "debit released before allocator deallocation"
            );
            assert!(
                CONTENT_DROPPED.load(Ordering::SeqCst),
                "registration waker survives debit release"
            );
            assert_eq!(REQUESTED.load(Ordering::SeqCst), bytes);
            assert_eq!(self.held.swap(0, Ordering::SeqCst), bytes);
            assert_eq!(self.releases.fetch_add(1, Ordering::SeqCst), 0);
        }
    }
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let mut exact = 0;
    for limit in [usize::MAX, 0, 1] {
        let limit = match limit {
            0 => exact,
            1 => exact - 1,
            other => other,
        };
        let owner = Owner::new(limit);
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        let stream = runtime.block_on(tokio::net::TcpStream::connect_addr_oteryn_owned(
            addr,
            owner.clone(),
        ));
        let requested = owner.requested.load(Ordering::SeqCst);
        if exact == 0 {
            exact = requested;
            eprintln!("E2 requested Arc registration backing: {exact} bytes");
        }
        assert_eq!(requested, exact);
        if limit < exact {
            assert!(stream.is_err());
            assert_eq!(REQUESTED.load(Ordering::SeqCst), 0);
            assert_eq!(owner.held.load(Ordering::SeqCst), 0);
            assert_eq!(owner.releases.load(Ordering::SeqCst), 0);
            ARMED.with(|armed| assert_eq!(armed.get(), 0));
        } else {
            let stream = stream.unwrap();
            struct Content;
            impl std::task::Wake for Content {
                fn wake(self: Arc<Self>) {}
            }
            impl Drop for Content {
                fn drop(&mut self) {
                    CONTENT_DROPPED.store(true, Ordering::SeqCst);
                }
            }
            CONTENT_DROPPED.store(false, Ordering::SeqCst);
            let waker = std::task::Waker::from(Arc::new(Content));
            assert!(stream
                .poll_read_ready(&mut std::task::Context::from_waker(&waker))
                .is_pending());
            drop(waker);
            assert!(!CONTENT_DROPPED.load(Ordering::SeqCst));
            assert_eq!(REQUESTED.load(Ordering::SeqCst), exact);
            assert!(!BACKING.load(Ordering::SeqCst).is_null());
            // Driver shutdown releases the list strong owner, but the socket
            // still owns the registration; final socket teardown must debit last.
            drop(runtime);
            assert_eq!(owner.held.load(Ordering::SeqCst), exact);
            assert!(!DEALLOCATED.load(Ordering::SeqCst));
            drop(stream);
            assert_eq!(owner.held.load(Ordering::SeqCst), 0);
            assert_eq!(owner.releases.load(Ordering::SeqCst), 1);
            continue;
        }
        drop(runtime);
    }
    // Concurrent socket and runtime teardown both consume the private strong
    // handle. Arc::into_inner supplies the universal exactly-one winner proof.
    let owner = Owner::new(exact);
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let stream = runtime
        .block_on(tokio::net::TcpStream::connect_addr_oteryn_owned(
            addr,
            owner.clone(),
        ))
        .unwrap();
    let barrier = std::sync::Barrier::new(2);
    std::thread::scope(|scope| {
        scope.spawn(|| {
            barrier.wait();
            drop(runtime);
        });
        scope.spawn(|| {
            barrier.wait();
            drop(stream);
        });
    });
    assert_eq!(owner.held.load(Ordering::SeqCst), 0);
    assert_eq!(owner.releases.load(Ordering::SeqCst), 1);

    // Exercise the owner control allocation with the same independent allocator
    // observer. Normal Arc destruction releases its inner debit too early;
    // every private capability drop must instead consume Arc::into_inner.
    struct Control {
        ledger: Arc<Owner>,
    }
    impl BlockingOwner for Control {
        fn try_reserve(&self, _bytes: usize) -> bool {
            false
        }
        fn release(&self, _bytes: usize) {
            unreachable!()
        }
        fn finalize(self: Arc<Self>) {
            drop(Arc::into_inner(self));
        }
    }
    impl Drop for Control {
        fn drop(&mut self) {
            self.ledger
                .release(self.ledger.requested.load(Ordering::SeqCst));
        }
    }
    let control_bytes = Layout::new::<[AtomicUsize; 2]>()
        .extend(Layout::new::<Control>())
        .unwrap()
        .0
        .pad_to_align()
        .size();
    let denied = Owner::new(control_bytes - 1);
    assert!(!denied.try_reserve(control_bytes));
    assert_eq!(REQUESTED.load(Ordering::SeqCst), 0);
    assert!(BACKING.load(Ordering::SeqCst).is_null());
    assert_eq!(denied.releases.load(Ordering::SeqCst), 0);

    let ledger = Owner::new(control_bytes);
    assert!(ledger.try_reserve(control_bytes));
    let control = tokio::task::OterynBlockingOwner::from(Arc::new(Control {
        ledger: ledger.clone(),
    }));
    assert_eq!(REQUESTED.load(Ordering::SeqCst), control_bytes);
    assert!(!BACKING.load(Ordering::SeqCst).is_null());
    let first = control.clone();
    let second = control.clone();
    assert!(first.same_owner(&second));
    drop(control);
    assert!(!DEALLOCATED.load(Ordering::SeqCst));
    assert_eq!(ledger.held.load(Ordering::SeqCst), control_bytes);
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
    assert!(DEALLOCATED.load(Ordering::SeqCst));
    assert_eq!(ledger.held.load(Ordering::SeqCst), 0);
    assert_eq!(ledger.releases.load(Ordering::SeqCst), 1);
    eprintln!("M01 owner Arc: exact allocation observed, debit released once after deallocation");
}

#[cfg(feature = "net")]
#[test]
fn oteryn_io_registration_owner_tcp_cancel_and_failed_connect() {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let cancelled = Arc::new(Witness::default());
    {
        let _enter = runtime.enter();
        let mut future = Box::pin(tokio::net::TcpStream::connect_addr_oteryn_owned(
            addr,
            cancelled.clone(),
        ));
        assert!(matches!(
            future
                .as_mut()
                .poll(&mut Context::from_waker(Waker::noop())),
            Poll::Pending
        ));
        assert!(cancelled.held.load(Ordering::SeqCst) > 0);
        drop(future);
        assert!(cancelled.held.load(Ordering::SeqCst) > 0);
        assert_eq!(cancelled.releases.load(Ordering::SeqCst), 0);
    }
    drop(runtime);
    assert_eq!(cancelled.held.load(Ordering::SeqCst), 0);
    assert_eq!(cancelled.releases.load(Ordering::SeqCst), 1);

    drop(listener);
    let failed = Arc::new(Witness::default());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    assert!(runtime
        .block_on(tokio::net::TcpStream::connect_addr_oteryn_owned(
            addr,
            failed.clone()
        ))
        .is_err());
    drop(runtime);
    assert_eq!(failed.held.load(Ordering::SeqCst), 0);
    assert_eq!(
        failed.reservations.load(Ordering::SeqCst),
        failed.releases.load(Ordering::SeqCst)
    );
}

#[cfg(all(feature = "net", unix))]
#[test]
fn oteryn_io_registration_owner_unix_exact_and_max_minus_one() {
    struct Limit {
        max: usize,
        requested: AtomicUsize,
        held: AtomicUsize,
        released: AtomicUsize,
    }
    impl BlockingOwner for Limit {
        fn try_reserve(&self, bytes: usize) -> bool {
            assert_eq!(self.requested.swap(bytes, Ordering::SeqCst), 0);
            if bytes > self.max {
                return false;
            }
            self.held.store(bytes, Ordering::SeqCst);
            true
        }
        fn release(&self, bytes: usize) {
            assert_eq!(self.held.swap(0, Ordering::SeqCst), bytes);
            assert_eq!(self.released.fetch_add(1, Ordering::SeqCst), 0);
        }
    }
    let path = std::env::temp_dir().join(format!("oteryn-e2-limit-{}", std::process::id()));
    let listener = std::os::unix::net::UnixListener::bind(&path).unwrap();
    let mut exact = 0;
    for selector in 0..3 {
        let max = match selector {
            0 => usize::MAX,
            1 => exact,
            _ => exact - 1,
        };
        let owner = Arc::new(Limit {
            max,
            requested: AtomicUsize::new(0),
            held: AtomicUsize::new(0),
            released: AtomicUsize::new(0),
        });
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .unwrap();
        runtime.block_on(async {
            let result = tokio::net::UnixStream::connect_oteryn_owned(&path, owner.clone()).await;
            if selector == 0 {
                exact = owner.requested.load(Ordering::SeqCst);
            }
            assert!(exact > 0);
            assert_eq!(owner.requested.load(Ordering::SeqCst), exact);
            if selector == 2 {
                assert!(result.is_err());
                assert_eq!(owner.held.load(Ordering::SeqCst), 0);
            } else {
                drop(result.unwrap());
                assert_eq!(owner.held.load(Ordering::SeqCst), exact);
                assert_eq!(owner.released.load(Ordering::SeqCst), 0);
            }
        });
        drop(runtime);
        assert_eq!(owner.held.load(Ordering::SeqCst), 0);
        assert_eq!(
            owner.released.load(Ordering::SeqCst),
            usize::from(selector != 2)
        );
    }
    drop(listener);
    std::fs::remove_file(path).unwrap();
}

#[cfg(all(feature = "net", feature = "time"))]
#[test]
fn oteryn_io_registration_owner_tcp_driver_pending_release() {
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let owner = Arc::new(Witness::default());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    runtime.block_on(async {
        let stream = tokio::net::TcpStream::connect_addr_oteryn_owned(addr, owner.clone())
            .await
            .unwrap();
        drop(stream);
        assert!(owner.held.load(Ordering::SeqCst) > 0);
        tokio::time::sleep(std::time::Duration::from_millis(1)).await;
        assert_eq!(owner.held.load(Ordering::SeqCst), 0);
        assert_eq!(owner.releases.load(Ordering::SeqCst), 1);
    });
    drop(runtime);
    assert_eq!(owner.releases.load(Ordering::SeqCst), 1);
}

#[cfg(feature = "net")]
#[test]
fn oteryn_io_registration_owner_tcp_registration_after_shutdown() {
    use std::future::Future;
    use std::task::{Context, Poll, Waker};
    let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let addr = listener.local_addr().unwrap();
    let owner = Arc::new(Witness::default());
    let runtime = tokio::runtime::Builder::new_current_thread()
        .enable_all()
        .build()
        .unwrap();
    let handle = runtime.handle().clone();
    drop(runtime);
    let _enter = handle.enter();
    let mut future = Box::pin(tokio::net::TcpStream::connect_addr_oteryn_owned(
        addr,
        owner.clone(),
    ));
    assert!(matches!(
        future
            .as_mut()
            .poll(&mut Context::from_waker(Waker::noop())),
        Poll::Ready(Err(_))
    ));
    drop(future);
    assert_eq!(owner.reservations.load(Ordering::SeqCst), 0);
    assert_eq!(owner.held.load(Ordering::SeqCst), 0);
    assert_eq!(owner.releases.load(Ordering::SeqCst), 0);
}
