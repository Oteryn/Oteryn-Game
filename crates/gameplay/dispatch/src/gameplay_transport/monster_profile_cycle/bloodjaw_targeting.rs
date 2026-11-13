//! Current player-owned vocation/vitals -> donor Bloodjaw priority -> native AI owner.
use super::*;
use crate::ai_think::CreatureAiTable;
use crate::foundation::ExactActorRef;
use oteryn_gameplay_content::content::spell_data::Vocation;
use oteryn_gameplay_encounters::bosses::rotten_blood::{
    RoomPosition,
    elder_bloodjaw_think::{BloodjawCandidate, ELDER_BLOODJAW_KEY, best_target},
};

pub(super) struct PlayerFacts {
    actor: ExactActorRef,
    vocation: Vocation,
    health: u32,
    maximum: u32,
}

/// Missing owner facts withhold this callback; never rank an incomplete roster.
pub(super) fn current_players(
    runtime: &ChannelRuntimeV1,
    states: &ChannelSpellStates,
) -> Option<Vec<PlayerFacts>> {
    let mut players = Vec::with_capacity(5);
    for (actor, _, session) in runtime.positioned_melee_census().ok()? {
        let Some(session) = session else {
            continue;
        };
        if states.is_dead(actor) {
            continue;
        }
        let state = states.get(runtime, actor, session)?;
        let (health, maximum) = state.source_party_vitals();
        if health == 0 {
            continue;
        }
        if players.len() == 5 {
            return None;
        }
        players.push(PlayerFacts {
            actor,
            vocation: state.source_party_vocation(),
            health,
            maximum,
        });
    }
    Some(players)
}

pub(super) fn retarget(
    runtime: &mut ChannelRuntimeV1,
    table: &mut CreatureAiTable,
    actor: ExactActorRef,
    players: Option<&[PlayerFacts]>,
    now: u64,
) -> Result<(), ()> {
    if !table
        .state(actor)
        .is_some_and(|state| state.creature_key() == Some(ELDER_BLOODJAW_KEY))
    {
        return Ok(());
    }
    let players = players.ok_or(())?;
    let position = runtime
        .read_actor_position(actor)
        .map_err(|_| ())?
        .position();
    let own = RoomPosition {
        x: position.x,
        y: position.y,
        floor: position.floor,
    };
    let mut candidates = Vec::with_capacity(players.len());
    for (index, player) in players.iter().enumerate() {
        // Chase may have moved players after the owner facts were borrowed.
        let position = runtime
            .read_actor_position(player.actor)
            .map_err(|_| ())?
            .position();
        candidates.push(BloodjawCandidate {
            actor: u64::try_from(index).map_err(|_| ())?,
            position: RoomPosition {
                x: position.x,
                y: position.y,
                floor: position.floor,
            },
            vocation: player.vocation,
            health: player.health,
            max_health: player.maximum,
        });
    }
    let Some(best) = best_target(own, &candidates) else {
        return Ok(());
    };
    let target = players
        .get(usize::try_from(best).map_err(|_| ())?)
        .ok_or(())?
        .actor;
    if table.state(actor).and_then(|state| state.target()) != Some(target) {
        let stamp = runtime.issue_owner_work().map_err(|_| ())?;
        table
            .set_current_bloodjaw_target(runtime, stamp, actor, target, now)
            .map_err(|_| ())?;
    }
    Ok(())
}
