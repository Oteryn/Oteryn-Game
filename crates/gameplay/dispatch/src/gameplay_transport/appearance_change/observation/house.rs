//! Observation-only adaptation of the existing appearance owner to native House custody.
use super::*;
use crate::foundation::HouseHirelingEntity;

pub(super) async fn read_state(
    admission: &ComposedFreshAdmission<'_, '_, '_>,
    fence: CurrentCharacterGameplayFence,
    grant_keys: Vec<String>,
) -> Option<CurrentCharacterAppearance> {
    match fence.runtime_scope {
        RuntimeScopeRefV1::Channel { .. } => admission.root.read_current_character_appearance_state(
            admission.character, admission.holder, fence, grant_keys,
        ).await.ok(),
        RuntimeScopeRefV1::Instance { .. } => admission.root.read_current_house_character_appearance_state(
            admission.character, admission.holder, fence, *admission.channel_id.as_bytes(), grant_keys,
        ).await.ok(),
    }
}

pub(super) fn current_runtime_for_read(
    runtime: &ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    fence: &CurrentCharacterGameplayFence,
    content: &NativeCosmeticsView,
) -> bool {
    if matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. }) {
        return current_runtime(runtime, actor, session, fence, content);
    }
    runtime.house_actor_state(actor, session).is_ok_and(|state| {
        state.scope == fence.runtime_scope && state.generation == fence.scope_ownership_generation
            && state.lease.character_id().as_bytes() == fence.character_id.as_bytes()
            && state.lease.generation() == fence.character_lease_generation
    }) && runtime.house_player_control_facts(actor, session).is_ok_and(|facts| facts.control_loss.is_none())
        && runtime.content_pin().server_artifact_digest() == content.source_digest()
}

type ActorCensus = Vec<(ExactActorRef, Option<GameSessionId>)>;
pub(super) fn census(
    runtime: &ChannelRuntimeV1,
    observer: ExactActorRef,
    session: GameSessionId,
) -> Option<(ActorCensus, Vec<HouseHirelingEntity>)> {
    if runtime.house_actor_state(observer, session).is_ok() {
        let players = runtime.house_visible_players(observer, session).ok()?.into_iter().map(|entry| {
            let (session, _) = runtime.house_observer(entry.actor).ok()??;
            Some((entry.actor, Some(session)))
        }).collect::<Option<Vec<_>>>()?;
        Some((players, runtime.house_hireling_entities(observer, session).ok()?))
    } else {
        Some((runtime.positioned_melee_census().ok()?.into_iter().map(|(actor, _, session)| (actor, session)).collect(), Vec::new()))
    }
}
