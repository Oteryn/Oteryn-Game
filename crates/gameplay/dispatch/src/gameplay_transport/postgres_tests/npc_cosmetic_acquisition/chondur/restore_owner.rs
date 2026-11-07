//! Actual committed assignment and fresh-admission continuation after fresh admission.
use super::*;
use crate::durability::fresh_admission::FreshAdmissionStore;
use crate::foundation::{ChannelId, ChannelRuntimeV1, WorldId};
use crate::gameplay_transport::connection::{
    AdmittedSession, ControllerBinding, SessionContinuity,
};
pub(super) async fn replace_runtime(
    h: &Harness,
    deps: &mut Dependencies,
    room: &crate::content::QualifiedNativeEntryRoom,
    world: WorldId,
    channel: ChannelId,
    generation: u64,
    capacity: usize,
) -> TestResult {
    let revision:String=sqlx::query_scalar("SELECT source_revision::text FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state=1 AND ownership_generation=$3::text::numeric")
  .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice()).bind(generation.to_string()).fetch_one(&h.pool).await?;
    let source = revision.parse::<u64>()?;
    let start = room.entry_start();
    let content = crate::foundation::ChannelContentPin::from_activation(
        world,
        generation,
        room.compiled().server_digest(),
        room.compiled().client_digest(),
        room.frame_binding().digest(),
        room.map_revision_digest(),
        (start.x, start.y, start.floor),
    );
    deps.runtime = tokio::sync::Mutex::new(
        ChannelRuntimeV1::from_committed_assignment(
            world,
            channel,
            h.node.fact().node_id(),
            h.node.fact().registration_revision(),
            generation,
            source,
            &format!("runtime-scope-assignment:{source}"),
            capacity,
            content,
        )
        .map_err(debug)?,
    );
    Ok(())
}
pub(super) async fn attach(
    h: &Harness,
    owner: &crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    previous: AdmittedSession,
    session: crate::foundation::GameSessionId,
    account: [u8; 16],
) -> TestResult<AdmittedSession> {
    let current = FreshAdmissionStore::from_root(h.root.clone())
        .current_session(session)
        .await?;
    let item_fence =
        crate::gameplay_transport::item_fence_of(current).ok_or("actual fresh item fence")?;
    let actor = {
        let mut runtime = owner.runtime.lock().await;
        let reservation = runtime.reserve_fresh_session(session).map_err(debug)?;
        let actor = runtime.commit_fresh_session(reservation).map_err(debug)?;
        // Match production first_entry.rs: bind the SAME durable current lease
        // before publishing this restored actor to any gameplay/appearance owner.
        runtime
            .bind_attacker_lease(actor, session, current.current_character_lease())
            .map_err(debug)?;
        assert_eq!(
            runtime
                .bound_attacker_lease(actor, session)
                .map_err(debug)?,
            current.current_character_lease()
        );
        actor
    };
    let mut continuity = SessionContinuity::FRESH;
    continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(
            owner.offered_capabilities(),
            previous.continuity.selected_capabilities.as_slice(),
        )
        .ok_or("same actual fresh offers")?;
    let mut admitted = AdmittedSession {
        game_session_id: session,
        runtime_actor: Some(actor),
        item_fence: Some(item_fence),
        continuity,
        controller: Some(ControllerBinding {
            transport: current
                .current_transport()
                .ok_or("actual current fresh transport")?,
            account_id: account,
        }),
        ..previous
    };
    owner.admit_progression_session(&mut admitted).await;
    let fence = owner
        .current_quest_fence(session)
        .await
        .map_err(debug)?
        .ok_or("actual fresh gameplay fence")?;
    let decided = owner.read_admission_position(fence).await.map_err(debug)?;
    {
        let mut runtime = owner.runtime.lock().await;
        owner
            .initialize_admitted_position(&mut runtime, actor, session, &decided)
            .await
            .map_err(debug)?;
    }
    assert!(owner.track_restored(session, actor, &decided, fence));
    admitted.first_entry = crate::gameplay_transport::connection::FirstEntryOutcome::Positioned;
    let (crate::gameplay_transport::spell_character_facts::CastFactsLoad::Ready { facts, .. }
    | crate::gameplay_transport::spell_character_facts::CastFactsLoad::NoVocation {
        facts, ..
    }) = crate::gameplay_transport::spell_character_facts::load_character_cast_facts(
        &h.root,
        owner.character,
        fence.character_id,
    )
    .await
    else {
        return Err("actual persisted class facts after fresh admission".into());
    };
    {
        let runtime = owner.runtime.lock().await;
        assert!(
            owner
                .states
                .spell_states
                .lock()
                .await
                .initialize(&runtime, actor, session, facts, (0, 0), owner.owner_now())
                .is_some()
        );
    }
    owner.admit_loop_training(&mut admitted).await;
    owner.refresh_quest_session(session, false).await;
    Ok(admitted)
}
