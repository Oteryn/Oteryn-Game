//! Jimmy's first greeting at the end of the rooftop pursuit.
use super::*;
use crate::content::{CollisionClass, LogicalCell};
use crate::foundation::MovementLocalPosition;
use oteryn_gameplay_quests::quest::top_of_the_city as quest;

const ASSASSIN: &str = "oteryn:creature.assassin";

pub(in crate::gameplay_transport) fn greet(
    admission: &super::super::ComposedFreshAdmission<'_, '_, '_>,
    runtime: &mut ChannelRuntimeV1,
    session: GameSessionId,
) -> Option<Result<&'static str, ()>> {
    let kills = {
        let sessions = admission.states.quest_sessions.lock().ok()?;
        let tracks = sessions.get(&session)?.copy.as_ref()?.tracks();
        if tracks.get(quest::STAGE) != Some(&1)
            || tracks.get(quest::PURSUIT) != Some(&quest::PURSUIT_COMPLETE)
        {
            return None;
        }
        usize::try_from(*tracks.get(quest::ASSASSINS).unwrap_or(&0)).ok()?
    };
    let remaining = 3usize.checked_sub(kills).filter(|count| *count > 0)?;
    let Ok(mut roster) = admission.states.npc_conversations.jimmy_assassins.lock() else {
        return Some(Err(()));
    };
    // A repeated greeting cannot replace defenders lured out of the room. After a
    // channel restart the character's committed deaths determine the missing wave.
    roster.retain(|actor| runtime.matches_live_creature_identity(*actor, ASSASSIN.as_bytes()));
    let missing = remaining.saturating_sub(roster.len());
    if missing > 0 {
        let Ok(policy) = runtime.companion_policy(ASSASSIN) else {
            return Some(Err(()));
        };
        let Ok(occupied) = runtime.positioned_melee_census() else {
            return Some(Err(()));
        };
        let mut placements = Vec::new();
        for y in 31379..=31384 {
            for x in 32857..=32858 {
                let cell = LogicalCell { x, y, z: 6 };
                let at = MovementLocalPosition { x, y, floor: 6 };
                if (x, y) != (32857, 31383)
                    && !occupied.iter().any(|(_, pos, _)| pos.position() == at)
                    && admission
                        .movement_cells
                        .index()
                        .lookup(admission.movement_cells.scope(), cell)
                        == Ok(CollisionClass::Walkable)
                {
                    placements.push((ASSASSIN, policy.maximum_health, at));
                }
            }
        }
        if placements.len() < missing {
            return Some(Err(()));
        }
        let Ok(spawned) = runtime.commit_quest_creature_spawn(&placements[..missing]) else {
            return Some(Err(()));
        };
        roster.extend(spawned);
    }
    Some(Ok("Get him!!"))
}

#[cfg(test)]
impl NpcConversations {
    pub(in crate::gameplay_transport) fn jimmy_roster_for_fixture(
        &self,
    ) -> Result<Vec<ExactActorRef>, &'static str> {
        self.jimmy_assassins
            .lock()
            .map(|actors| actors.clone())
            .map_err(|_| "Jimmy roster")
    }
}
