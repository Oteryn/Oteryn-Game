//! Native First Dragon phase hooks on the shared boss activity.
use super::ComposedFreshAdmission;
use crate::foundation::{
    ExactActorRef, GameSessionId, MovementLocalPosition, MovementPositionSnapshot,
};

pub(super) async fn step_in(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    actor: ExactActorRef,
    session: GameSessionId,
    committed: MovementPositionSnapshot,
    now_ms: u64,
) {
    let Some(map) = owner.bundle_map() else {
        return;
    };
    let mut runtime = owner.runtime.lock().await;
    let states = owner.states.spell_states.lock().await;
    if !states
        .encounter_phases
        .first_dragon_portal_at(&runtime, committed.position(), now_ms)
    {
        return;
    }
    let destination = MovementLocalPosition {
        x: 33621,
        y: 31023,
        floor: committed.position().floor,
    };
    let closed = crate::map::floor::BundleDoors(&map.facts);
    let blocked = super::pvp_live::pz_blocked(&states, actor, session, owner.owner_now());
    let Ok(proof) = crate::map::floor::prepare_scripted_quest_teleport_facing(
        &runtime,
        owner.movement_cells(),
        actor,
        session,
        committed,
        destination,
        blocked,
        &closed,
        committed.facing(),
        true,
    ) else {
        return;
    };
    let _ = runtime.commit_source_step(proof);
}
