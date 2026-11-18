//! Crystal 201189f8 movements_marziel_blood.lua:27-37. The stepper stays in place.
use super::*;
use crate::content::{CollisionClass, LogicalCell};

pub(super) struct Group<P> {
    pub proofs: Vec<P>,
    pub players: Vec<(ExactActorRef, GameSessionId, MovementPositionSnapshot)>,
}

impl<P> Group<P> {
    pub(super) fn sessions(&self) -> Vec<GameSessionId> {
        self.players
            .iter()
            .map(|(_, session, _)| *session)
            .collect()
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    // Crystal 201189f8 actions_lersatio_mirrors.lua:38 moves the area before spawning.
    pub(super) fn prepare_encounter_group<'a>(
        &'a self,
        runtime: &ChannelRuntimeV1,
        states: &ChannelSpellStates,
        closed: &'a dyn crate::map::floor::CellBlocker,
        callback: &Callback,
        initiator: GameSessionId,
    ) -> Result<Option<Group<impl crate::foundation::SourceStepCommitProof + 'a>>, UseDisposition>
    {
        let Some(area) = &callback.effects.encounter_group else {
            return Ok(None);
        };
        let census = runtime
            .positioned_melee_census()
            .map_err(|_| UseDisposition::Rejected)?;
        let mut players = Vec::new();
        let mut proofs = Vec::new();
        for (actor, expected, session) in census {
            let Some(session) = session else { continue };
            if !native(expected.position()).is_some_and(|at| {
                area.contains(Position {
                    x: at.x,
                    y: at.y,
                    floor: at.floor,
                })
            }) {
                continue;
            }
            if players.len() >= 16
                || states.is_dead(actor)
                || !states.spell_idle(runtime, actor, session)
                || !self.states.player_trades.physical(session, None)
                || self.party_member_has_foreign_pending(session, initiator)
            {
                return Err(UseDisposition::Occupied);
            }
            proofs.push(
                crate::map::floor::prepare_quest_encounter_ejection(
                    runtime,
                    self.movement_cells(),
                    actor,
                    session,
                    expected,
                    local(area.destination),
                    closed,
                )
                .map_err(|_| UseDisposition::NotPossible)?,
            );
            players.push((actor, session, expected));
        }
        if !players.iter().any(|(_, session, _)| *session == initiator) {
            return Err(UseDisposition::Rejected);
        }
        Ok(Some(Group { proofs, players }))
    }

    pub(super) fn prepare_stationary_encounter<'a>(
        &self,
        runtime: &mut ChannelRuntimeV1,
        map: &BundleMap,
        callback: &'a Callback,
    ) -> Result<Vec<(&'a str, i64, MovementLocalPosition)>, UseDisposition> {
        let rows = &callback.effects.stationary_encounter;
        if rows.is_empty() {
            return Ok(Vec::new());
        }
        let census = runtime
            .positioned_melee_census()
            .map_err(|_| UseDisposition::Rejected)?;
        let mut monsters = Vec::with_capacity(rows.len());
        for row in rows {
            let at = local(row.position);
            let cell = LogicalCell {
                x: at.x,
                y: at.y,
                z: i32::from(at.floor),
            };
            if self
                .movement_cells
                .index()
                .lookup(self.movement_cells.scope(), cell)
                != Ok(CollisionClass::Walkable)
                || map
                    .facts
                    .quest_callback_blocked(map.overlay.base(), tile(row.position), false)
                || census
                    .iter()
                    .any(|(_, position, _)| position.position() == at)
            {
                return Err(UseDisposition::Occupied);
            }
            let policy = runtime
                .companion_policy(&row.creature)
                .map_err(|_| UseDisposition::Rejected)?;
            monsters.push((row.creature.as_str(), policy.maximum_health, at));
        }
        runtime
            .preflight_quest_creature_spawn(&monsters)
            .map_err(|_| UseDisposition::Exhausted)?;
        Ok(monsters)
    }
}
