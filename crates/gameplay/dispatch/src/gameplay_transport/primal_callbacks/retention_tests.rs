use super::*;
use std::{
    future::Future,
    task::{Context, Poll, Waker},
};

#[test]
fn primal_native_cancelled_map_read_keeps_original_callback_owner() {
    let callbacks = PrimalCallbacks::default();
    let retained = callbacks.retained_owner();
    let effect = PrimalMenaceEffect::ScheduledSpawn {
        kind: WaveKind::Beast,
        position: MovementLocalPosition {
            x: 33558,
            y: 32759,
            floor: 15,
        },
        at_ms: 24_000,
    };
    let mut work = Box::pin(async move {
        let mut owner = retained.lock_owned().await;
        owner
            .deferred
            .enqueue(effect)
            .expect("source beast callback queued before current Map read");
        std::future::pending::<()>().await;
    });
    let mut context = Context::from_waker(Waker::noop());
    assert!(matches!(work.as_mut().poll(&mut context), Poll::Pending));
    assert!(
        callbacks.has_native_work(),
        "busy retained owner cannot look empty"
    );
    assert!(callbacks.has_native_due(0));
    assert_eq!(callbacks.next_due_us(), Some(0));
    drop(work);
    let retained = callbacks.retained_owner();
    let owner = retained
        .try_lock()
        .expect("cancellation released only the lease");
    assert_eq!(owner.deferred.0.len(), 1);
    assert_eq!(owner.deferred.0[0].effect, effect);
    assert_eq!(owner.deferred.0[0].id, 1);
}

#[test]
fn primal_native_deferred_peek_preserves_queue_until_physical_ack() {
    let mut callbacks = DeferredCallbacks::default();
    let effect = PrimalMenaceEffect::ScheduledSpawn {
        kind: WaveKind::Pod,
        position: MovementLocalPosition {
            x: 33558,
            y: 32759,
            floor: 15,
        },
        at_ms: 24_000,
    };
    callbacks.enqueue(effect).expect("source retained Pod");
    assert!(callbacks.peek_due(23_999, false).is_none());
    let first = callbacks
        .peek_due(24_000, false)
        .expect("Pod outlives its issuer");
    assert_eq!(
        callbacks.0.len(),
        1,
        "Map await cannot own the only occurrence"
    );
    assert_eq!(callbacks.peek_due(25_000, false), Some(first));
    callbacks
        .acknowledge(first.id)
        .expect("physical source callback committed");
    assert!(callbacks.peek_due(25_000, false).is_none());
    assert!(
        callbacks.acknowledge(first.id).is_err(),
        "duplicate acknowledgement has no successor"
    );
}
