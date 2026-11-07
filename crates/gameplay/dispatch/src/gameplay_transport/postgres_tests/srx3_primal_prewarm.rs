//! A real ready Instance can wait without starting Primal's player-driven clock.
use super::{
    TestResult,
    fixture::{Harness, debug},
};
use crate::content::bosses::activity_template::PinnedActivitySourceRoom;
use crate::foundation::{MovementLocalPosition, RuntimeScopeRefV1};
use crate::gameplay_transport::ComposedFreshAdmission;

pub(super) async fn assert_empty_ready_instance(
    h: &Harness,
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    scope: RuntimeScopeRefV1,
) -> TestResult {
    let ready = owner
        .ensure_pinned_source_runtime(
            PinnedActivitySourceRoom::Primal,
            Some((
                "oteryn:creature.the_primal_menace",
                MovementLocalPosition {
                    x: 33558,
                    y: 32759,
                    floor: 15,
                },
            )),
            1024,
        )
        .await
        .map_err(|error| format!("actual empty source allocation: {error:?}"))?;
    owner.clock_origin = owner
        .clock_origin
        .checked_sub(std::time::Duration::from_secs(120))
        .ok_or("prewarm owner clock")?;
    let view = owner.runtime_view(&ready);
    view.drain_monster_melee().await;
    let runtime = view.runtime.lock().await;
    assert_eq!(runtime.binding().runtime_scope(), scope);
    assert!(
        runtime
            .activity_participant_sessions()
            .map_err(debug)?
            .is_empty()
    );
    let census = runtime.positioned_actor_census().map_err(debug)?;
    assert_eq!(
        census.len(),
        1,
        "ready source room retains only its actual initial boss"
    );
    assert!(census.iter().all(|(actor, _, session)| session.is_none()
        && runtime.matches_live_creature_identity(*actor, b"oteryn:creature.the_primal_menace")));
    let states = view.states.spell_states.lock().await;
    assert!(
        !states.primal_callbacks.has_native_work(),
        "no source wave/Pod callback starts before any participant enters"
    );
    assert_eq!(states.primal_callbacks.next_due_us(), None);
    assert!(
        !states.primal_callbacks.source_started_for_test(),
        "prewarm cannot initialize the source first-onThink clock"
    );
    drop(states);
    drop(runtime);
    let RuntimeScopeRefV1::Instance { instance_id, .. } = scope else {
        return Err("actual ready Instance".into());
    };
    let callbacks: i64 = sqlx::query_scalar(
        "SELECT count(*) FROM game_primal_pod_callbacks WHERE instance_id=encode($1,'hex')::uuid",
    )
    .bind(instance_id.as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        callbacks, 0,
        "prewarming cannot create a durable Pod callback"
    );
    Ok(())
}
