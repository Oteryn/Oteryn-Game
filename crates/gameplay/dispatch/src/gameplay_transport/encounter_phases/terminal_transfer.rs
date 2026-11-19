//! Crystal 201189f8, the_first_dragon/creaturescripts_death_first_dragon.lua:21-30.
use super::{CarrierError, ChannelRuntimeV1, ChannelSpellStates, ExactActorRef, GameSessionId};
use crate::content::NativeEntryMovementCells;
use crate::foundation::MovementLocalPosition;
use crate::map::{boot::BundleMap, floor::BundleDoors};
use oteryn_gameplay_quests::quest::world_callbacks::{Position, TerminalTransfer};

use super::super::paced_movement::underwater_drown::underwater;
use super::super::quest_callbacks::{local, native};

#[derive(Debug)]
pub(super) struct Transfer {
    plan: TerminalTransfer,
    spectators: Vec<(ExactActorRef, GameSessionId)>,
}

fn inside(plan: TerminalTransfer, at: MovementLocalPosition) -> bool {
    native(at).is_some_and(|tile| {
        plan.contains(Position {
            x: tile.x,
            y: tile.y,
            floor: tile.floor,
        })
    })
}

impl Transfer {
    pub(super) fn capture(
        runtime: &ChannelRuntimeV1,
        plan: TerminalTransfer,
    ) -> Result<Self, CarrierError> {
        let spectators = runtime
            .positioned_melee_census()?
            .into_iter()
            .filter_map(|(actor, at, session)| {
                session
                    .filter(|_| inside(plan, at.position()))
                    .map(|session| (actor, session))
            })
            .collect();
        Ok(Self { plan, spectators })
    }

    pub(super) fn is_pending(&self) -> bool {
        !self.spectators.is_empty()
    }

    pub(super) fn credit_recipients(&self) -> Option<&[(ExactActorRef, GameSessionId)]> {
        self.plan
            .credit_spectators
            .then_some(self.spectators.as_slice())
    }

    pub(super) fn project(
        &mut self,
        runtime: &mut ChannelRuntimeV1,
        states: &ChannelSpellStates,
        cells: &NativeEntryMovementCells,
        map: &BundleMap,
        published: &mut Vec<Movement>,
    ) {
        let to = local(self.plan.destination);
        let closed = BundleDoors(&map.facts);
        self.spectators.retain(|&(actor, session)| {
            let Ok(expected) = runtime.read_actor_position(actor) else {
                return false;
            };
            let from = expected.position();
            if states.is_dead(actor)
                || !inside(self.plan, from)
                || runtime.bound_attacker_lease(actor, session).is_err()
            {
                return false;
            }
            let Ok(proof) = crate::map::floor::prepare_quest_encounter_ejection(
                runtime, cells, actor, session, expected, to, &closed,
            ) else {
                return true;
            };
            let Ok(snapshot) = runtime.commit_source_step(proof) else {
                return true;
            };
            published.push(Movement {
                actor,
                session,
                from,
                to: snapshot.position(),
            });
            false
        });
    }
}

pub(super) struct Movement {
    actor: ExactActorRef,
    session: GameSessionId,
    from: MovementLocalPosition,
    to: MovementLocalPosition,
}

impl Movement {
    pub(super) fn publish(
        self,
        runtime: &ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
        now_us: u64,
    ) {
        states.stage_special_movement_step(
            runtime,
            map,
            self.actor,
            self.session,
            now_us,
            Some(self.from),
            self.to,
            None,
        );
        states.stage_underwater_step(
            runtime,
            self.actor,
            self.session,
            now_us,
            underwater(map, self.from),
            underwater(map, self.to),
        );
        if let Some(tile) = native(self.to) {
            map.facts.emit_movement_effect(tile, 11);
        }
    }
}
