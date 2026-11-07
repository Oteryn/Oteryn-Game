use crate::content::{ProjectV2DefinitionRef as Ref, ProjectV2Family};
use crate::foundation::{MovementLocalPosition, owner_timer::SemanticTimeMicros};
use crate::gameplay_transport::actor_spell::tests::runtime_with_player;
use crate::source_callback_cast::{CallbackCastOwner, SourceCallbackCast};
use crate::source_callback_cast_tests::{draft, fence, proposal};

#[test]
fn srx3_native_cancelled_current_map_read_retains_original_source_timer() -> Result<(), String> {
    use super::NativeCallbacks;
    use std::{
        future::Future,
        task::{Context, Poll, Waker},
    };
    let (mut runtime, _, _) = runtime_with_player(0x72);
    let creature = Ref {
        family: ProjectV2Family::Creature,
        key: "oteryn:creature.foam_stalker".into(),
        revision: "definition-r1".into(),
    };
    let caster = runtime
        .admit_monster_lab_creature(
            MovementLocalPosition {
                x: 100,
                y: 100,
                floor: 7,
            },
            &creature.key,
            300000,
        )
        .map_err(|error| format!("actual actor: {error:?}"))?;
    let source = SourceCallbackCast::qualify(
        &draft(&runtime),
        &creature,
        4,
        runtime.content_pin().server_artifact_digest(),
    )
    .map_err(|error| format!("source closure: {error:?}"))?
    .ok_or("Foam callback")?;
    let (fence, stamp) = fence(&runtime);
    let mut owner = CallbackCastOwner::new(&runtime, &fence)
        .map_err(|error| format!("current timer owner: {error:?}"))?;
    owner
        .schedule(
            &runtime,
            &fence,
            stamp,
            &source,
            &proposal(caster, &source, 0),
            SemanticTimeMicros::from_micros(0),
        )
        .map_err(|error| format!("original native source stage: {error:?}"))?;
    let callbacks = NativeCallbacks::with_source_cast_owner_for_test(owner);
    assert_eq!(callbacks.next_due_time(), Some(1_000_000));
    let retained = callbacks.retained_owner();
    let mut work = Box::pin(async move {
        let guard = retained.lock_owned().await;
        assert_eq!(guard.source_cast_status_for_test(caster), (true, true));
        std::future::pending::<()>().await;
        drop(guard);
    });
    let mut context = Context::from_waker(Waker::noop());
    assert!(matches!(work.as_mut().poll(&mut context), Poll::Pending));
    assert!(callbacks.has_native_work());
    assert!(callbacks.has_native_due(0));
    drop(work);
    assert_eq!(callbacks.source_cast_status_for_test(caster), (true, true));
    assert_eq!(
        callbacks.next_due_time(),
        Some(1_000_000),
        "cancellation must retain the original due time"
    );
    Ok(())
}
