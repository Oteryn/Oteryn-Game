use super::*;
type TestResult<T = ()> = Result<T, String>;
use std::{
    future::Future,
    task::{Context, Poll, Waker},
};

#[test]
fn primal_native_cancelled_map_read_keeps_original_callback_owner() -> TestResult {
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
            .map_err(|_| "source beast callback queued before current Map read".to_owned())?;
        std::future::pending::<()>().await;
        Ok::<(), String>(())
    });
    let mut context = Context::from_waker(Waker::noop());
    let polled = work.as_mut().poll(&mut context);
    if let Poll::Ready(Err(error)) = &polled {
        return Err(error.clone());
    }
    assert!(matches!(polled, Poll::Pending));
    assert!(
        callbacks.has_native_work(),
        "busy retained owner cannot look empty"
    );
    assert!(callbacks.has_native_due(0));
    assert_eq!(callbacks.next_due_us(), Some(0));
    assert_eq!(callbacks.next_source_due_time(), Err(()));
    drop(work);
    assert_eq!(callbacks.next_source_due_time(), Ok(Some(24_000_000)));
    let retained = callbacks.retained_owner();
    let owner = retained
        .try_lock()
        .map_err(|_| "cancellation released only the lease".to_owned())?;
    assert_eq!(owner.deferred.0.len(), 1);
    assert_eq!(owner.deferred.0[0].effect, effect);
    assert_eq!(owner.deferred.0[0].id, 1);
    Ok(())
}

#[test]
fn primal_native_deferred_peek_preserves_queue_until_physical_ack() -> TestResult {
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
    callbacks
        .enqueue(effect)
        .map_err(|_| "source retained Pod".to_owned())?;
    assert!(callbacks.peek_due(23_999, false).is_none());
    let first = callbacks
        .peek_due(24_000, false)
        .ok_or("Pod outlives its issuer")?;
    assert_eq!(
        callbacks.0.len(),
        1,
        "Map await cannot own the only occurrence"
    );
    assert_eq!(callbacks.peek_due(25_000, false), Some(first));
    callbacks
        .acknowledge(first.id)
        .map_err(|_| "physical source callback committed".to_owned())?;
    assert!(callbacks.peek_due(25_000, false).is_none());
    assert!(
        callbacks.acknowledge(first.id).is_err(),
        "duplicate acknowledgement has no successor"
    );
    Ok(())
}
