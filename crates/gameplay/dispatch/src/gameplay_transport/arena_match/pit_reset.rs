//! M51 source reset before a new fight: Crystal201189f
//! data-global/lib/quests/svargrond_arena.lua:207-246.
use super::*;

pub(super) fn reset_monsters(
    runtime: &mut ChannelRuntimeV1,
    actor: ExactActorRef,
    session: GameSessionId,
    state: &MatchState,
) -> Option<()> {
    runtime.owner_fence().ok()?;
    if actor.channel_id().is_none()
        || state.phase != 1
        || state.session != *session.as_bytes()
        || state.scope_generation != runtime.binding().scope_generation().get()
        || runtime
            .player_control_facts(actor, session)
            .ok()?
            .control_loss
            .is_some()
    {
        return None;
    }
    let layout = oteryn_gameplay_quests::quest::barbarian_arena::Pit::new(state.pit)?.layout();
    let players = runtime.positioned_actor_census().ok()?;
    if layout.occupied_by_other(
        &session,
        players.into_iter().filter_map(|(_, at, player)| {
            let at = at.position();
            player.map(|player| (player, [at.x, at.y, -i32::from(at.floor)]))
        }),
    ) {
        return None;
    }
    // The native view separates Monsters from NPCs/Players. Hold this guard
    // through selection and removal; an administrative removal grants no kill.
    let monsters = runtime.visible_entities().creatures;
    let selected = layout.monsters_to_reset(monsters.into_iter().map(|monster| {
        let at = monster.position;
        (monster.actor, [at.x, at.y, -i32::from(at.floor)])
    }))?;
    for target in selected {
        // Removing a parent also removes its bounded summons. A planned child
        // already gone through that exact native owner is an idempotent absence.
        if runtime.contains_live_creature(target) {
            runtime.remove_arena_opponent(target).ok()?;
        }
    }
    Some(())
}
