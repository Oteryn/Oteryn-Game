//! Native source initialization and data-only recovery of a committed House.
use super::{AdmittedSession, ComposedFreshAdmission, FreshAdmissionStore, GameSessionState,
    Harness, TestResult, debug};
use crate::foundation::GameSessionId;

pub(super) async fn assert_initialized(
    h: &Harness, owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession, source: GameSessionId,
) -> TestResult {
    let store = FreshAdmissionStore::from_root(h.root.clone());
    let (retired, _) = store.current_session_at(source).await.map_err(debug)?;
    let (current, _) = store.current_session_at(admitted.game_session_id).await.map_err(debug)?;
    assert_ne!(source, admitted.game_session_id);
    assert_eq!(retired.session_state(), GameSessionState::Terminal);
    assert_eq!(current.session_state(), GameSessionState::Active);
    assert_eq!(current.commit().game_session_id(), source,
        "the destination keeps the actual fresh source as immutable provenance");
    let fence = admitted.item_fence.ok_or("current House fence")?;
    let (source_markers, copied_markers): (i64, i64) = sqlx::query_as(
        "SELECT count(*) FILTER(WHERE game_session_id=encode($2,'hex')::uuid), \
         count(*) FILTER(WHERE game_session_id=encode($3,'hex')::uuid) \
         FROM game_character_training_markers \
         WHERE character_id=encode($1,'hex')::uuid AND cause='offline_settlement'",
    ).bind(fence.character_id.as_bytes().as_slice()).bind(source.as_bytes().as_slice())
     .bind(admitted.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!((source_markers, copied_markers), (1, 0),
        "native settlement belongs to the real login source, never a copied House marker");
    assert!(h.root.read_house_hazard_selection(owner.character, owner.holder, fence,
        admitted.channel_id).await.map_err(debug)?.is_some());

    let quests = owner.states.quest_sessions.lock().map_err(debug)?;
    assert!(!quests.contains_key(&source));
    assert!(quests.get(&admitted.game_session_id).is_some_and(|entry| entry.copy.is_some()),
        "fresh House admission retains the Character-owned quest observation");
    drop(quests);
    assert!(!owner.states.progression_sessions.lock().map_err(debug)?.contains_key(&source));
    for source_present in [
        owner.states.wheel_sessions.lock().map_err(debug)?.contains_key(&source),
        owner.states.atelier_sessions.lock().map_err(debug)?.contains_key(&source),
    ] { assert!(!source_present); }
    assert!(owner.states.wheel_sessions.lock().map_err(debug)?.contains_key(&admitted.game_session_id));
    assert!(owner.states.atelier_sessions.lock().map_err(debug)?.contains_key(&admitted.game_session_id));
    Ok(())
}

pub(super) async fn recover_lost_views(
    h: &Harness, owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession, source: GameSessionId,
) -> TestResult<AdmittedSession> {
    let committed = h.root.house_transfer_recovery(admitted.game_session_id).await
        .map_err(debug)?.ok_or("actual fresh House handoff")?;
    assert!(!committed.started,
        "this models acknowledgement loss before the destination command namespace starts");
    assert_eq!(committed.source, source);
    assert!(committed.admission_restore);
    assert_eq!(committed.command, 0,
        "fresh restoration has authentic server provenance, never a client CommandId");
    assert_eq!(admitted.continuity.server_sequence, 0);
    let before = durable_observation(h, admitted).await?;
    // Lose only process-local observations. The authentic destination, actor,
    // native handoff, saved position and command stream remain untouched.
    owner.states.progression_sessions.lock().map_err(debug)?.remove(&admitted.game_session_id);
    owner.states.quest_sessions.lock().map_err(debug)?.remove(&admitted.game_session_id);
    owner.states.wheel_sessions.lock().map_err(debug)?.remove(&admitted.game_session_id);
    owner.states.atelier_sessions.lock().map_err(debug)?.remove(&admitted.game_session_id);
    {
        let mut states = owner.states.spell_states.lock().await;
        states.clear_prey(admitted.game_session_id);
        states.charm_states.retain(|(_, session, _)| *session != admitted.game_session_id);
    }
    let recovered = super::resume(owner, admitted, admitted.game_session_id, 31, 19).await?;
    assert_eq!(recovered.game_session_id, admitted.game_session_id);
    assert_eq!(recovered.runtime_actor, admitted.runtime_actor);
    assert_eq!(recovered.continuity.next_command_id, 1);
    assert_eq!(recovered.continuity.server_sequence, 0,
        "ServerResumeAccepted cannot manufacture a CommandResult for native admission provenance");
    assert_initialized(h, owner, recovered, source).await?;
    assert!(owner.progression_binding(recovered.game_session_id).is_some(),
        "read-only recovery reconstructs the already qualified policy without initialization");
    assert_eq!(durable_observation(h, recovered).await?, before,
        "ACK recovery reads observations without another root write, settlement, build or Prey receipt");
    if owner.prey_catalogue().is_some() {
        assert_eq!(owner.states.spell_states.lock().await.prey.len(), 1,
            "the one actual House actor regains its bounded Prey observation");
    }
    let fence = recovered.item_fence.ok_or("recovered actual fence")?;
    let mut stale = fence;
    stale.character_lease_generation += 1;
    assert!(matches!(h.root.read_house_hazard_selection(owner.character, owner.holder, stale,
        recovered.channel_id).await,
        Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)));
    let mut old_scope = fence;
    old_scope.runtime_scope = crate::foundation::RuntimeScopeRefV1::channel(
        recovered.world_id, recovered.channel_id);
    assert!(matches!(h.root.read_house_hazard_selection(owner.character, owner.holder, old_scope,
        recovered.channel_id).await,
        Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)));
    if owner.prey_catalogue().is_some() {
        let mut stale_admission = recovered;
        stale_admission.item_fence = Some(stale);
        assert!(matches!(owner.read_house_admission_prey(&stale_admission).await,
            Err(crate::durability::character_progression::CharacterProgressionError::AuthorityRejected)));
    }
    assert_eq!(durable_observation(h, recovered).await?, before,
        "one-field stale lease and relabeled scope refusals leave native data untouched");
    // A real control loss after the ACK was recovered belongs to the ordinary
    // CompleteReconnect owner, even while the native destination is unstarted.
    assert_eq!(owner.lose_control_for_cause(recovered, std::time::Duration::ZERO, None).await,
        crate::gameplay_transport::connection::ControlLossResult::Recorded);
    let resumed = super::resume(owner, recovered, recovered.game_session_id, 32, 20).await?;
    assert_eq!(resumed.game_session_id, recovered.game_session_id);
    assert_eq!(resumed.runtime_actor, recovered.runtime_actor);
    assert_eq!(resumed.continuity.next_command_id, 1);
    assert_eq!(resumed.continuity.connection_generation, recovered.continuity.connection_generation + 1);
    let actor = resumed.runtime_actor.ok_or("retained House actor")?;
    assert!(owner.runtime.lock().await.house_player_control_facts(actor, resumed.game_session_id)
        .map_err(debug)?.control_loss.is_none());
    assert!(FreshAdmissionStore::from_root(h.root.clone()).with_house_lifecycle_owner(owner.holder)
        .resumed_history(resumed.game_session_id).await.map_err(debug)?.is_some(),
        "actual loss records CompleteReconnect history instead of another ACK recovery");
    assert_eq!(durable_observation(h, resumed).await?, before,
        "pre-command CompleteReconnect retains the same native Character state");
    Ok(resumed)
}

/// Compare the real owning rows and receipt counts, not only the wire result.
async fn durable_observation(h: &Harness, admitted: AdmittedSession)
    -> TestResult<(String, String, String, i64, i64, i64)>
{
    let character = admitted.item_fence.ok_or("observation character")?.character_id;
    Ok(sqlx::query_as(
        "SELECT r.character_revision::text, \
         coalesce((SELECT jsonb_agg(to_jsonb(p) ORDER BY p.slot)::text \
           FROM game_character_prey_slots p WHERE p.character_id=r.character_id),'[]'), \
         coalesce((SELECT to_jsonb(h)::text FROM game_character_hazard h \
           WHERE h.character_id=r.character_id),'null'), \
         (SELECT count(*) FROM game_character_training_markers m WHERE m.character_id=r.character_id), \
         (SELECT count(*) FROM game_character_build_receipts b WHERE b.character_id=r.character_id), \
         (SELECT count(*) FROM game_character_prey_receipts p WHERE p.character_id=r.character_id) \
         FROM game_character_roots r WHERE r.character_id=encode($1,'hex')::uuid",
    ).bind(character.as_bytes().as_slice()).fetch_one(&h.pool).await?)
}
