//! Current raw source eligibility, observed under the retained actual actor guards.
use crate::durability::npc_cosmetics::NpcSourceActorObservation;
use oteryn_gameplay_domain::domain::condition::ConditionType;
pub(super) fn capture(
    runtime: &crate::foundation::ChannelRuntimeV1,
    states: &crate::gameplay_transport::actor_spell::ChannelSpellStates,
    actor: crate::foundation::ExactActorRef,
    session: crate::foundation::GameSessionId,
    now: u64,
) -> Option<NpcSourceActorObservation> {
    let state = states.get(runtime, actor, session)?;
    let player = state.owned_conditions();
    if !player.accepts_time(now) {
        return None;
    }
    let native = runtime
        .actor_conditions_at(actor, Some(session), now)
        .ok()?;
    Some(NpcSourceActorObservation {
        actor_identity: actor.placement_identity(),
        movement_revision: runtime.read_actor_position(actor).ok()?.revision(),
        state_revision: state.revision(),
        observed_owner_us: now,
        player_conditions: player.persistent_snapshot(now).ok()?,
        player_statuses: active_conditions(player, now),
        native_statuses: active_conditions(native, now),
    })
}

fn active_conditions<S: Clone>(
    store: &oteryn_gameplay_foundation::foundation::condition::ConditionStore<S>,
    now: u64,
) -> Vec<(ConditionType, i64)> {
    store
        .instances()
        .iter()
        .filter_map(|instance| {
            instance
                .status_remaining_ms(now)
                .filter(|remaining| *remaining != 0)
                .map(|remaining| (instance.definition().condition_type(), remaining))
        })
        .collect()
}
