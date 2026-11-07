//! Source environmental moves share the runtime's staged physical/map transaction.
use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "the source plan and its atomic projection share one exact runtime owner turn"
    )]
    pub(super) fn eject_quest_occupants(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        optional_pvp: Option<bool>,
        project: impl FnOnce() -> bool,
    ) -> UseDisposition {
        let Some(ejection) = &callback.eject_occupants else {
            return UseDisposition::Rejected;
        };
        let destination = tile(ejection.destination);
        if map.facts.house_tile(destination)
            || crate::map::view::placement_of(key).is_none_or(|(pos, _)| pos == destination)
            || map
                .overlay
                .base()
                .tile(destination.x, destination.y, destination.floor)
                .is_none()
        {
            return UseDisposition::Rejected;
        }
        // This census also includes control-lost positioned players. They must
        // refuse the whole operation rather than being silently left in the water.
        let Ok(census) = runtime.positioned_melee_census() else {
            return UseDisposition::Rejected;
        };
        let closed = crate::map::floor::BundleDoors(map.facts.doors());
        let mut occupants = Vec::new();
        let mut proofs = Vec::new();
        if occupants.try_reserve(census.len()).is_err() || proofs.try_reserve(census.len()).is_err()
        {
            return UseDisposition::Exhausted;
        }
        for (who, expected, session) in census {
            if !ejection
                .positions
                .iter()
                .any(|p| local(*p) == expected.position())
            {
                continue;
            }
            // Source getTopCreature requires the existing sole-occupant proof.
            // The runtime deliberately has no same-cell actor stack ordering.
            if !runtime
                .is_sole_current_actor_on_cell(who)
                .is_ok_and(|sole| sole)
            {
                return UseDisposition::Occupied;
            }
            if let Some(session) = session
                && (!runtime
                    .player_control_facts(who, session)
                    .is_ok_and(|facts| facts.control_loss.is_none())
                    || states.is_dead(who)
                    || !states.spell_idle(runtime, who, session)
                    || self.has_pending_quest_callback(session)
                    || !self.states.player_trades.physical(session, None))
            {
                return UseDisposition::Occupied;
            }
            let blocked = session.is_some_and(|session| {
                super::super::pvp_live::pz_blocked(states, who, session, self.owner_now())
            });
            let Ok(proof) = crate::map::floor::prepare_quest_callback_ejection(
                runtime,
                self.movement_cells,
                who,
                session,
                expected,
                local(ejection.destination),
                blocked,
                &closed,
            ) else {
                return UseDisposition::NotPossible;
            };
            occupants.push((who, session, expected));
            proofs.push(proof);
        }
        let Ok(moved) = runtime.commit_quest_world_projection(proofs, project) else {
            return UseDisposition::Rejected;
        };
        for ((who, session, before), after) in occupants.into_iter().zip(moved) {
            if let Some(session) = session {
                states.stage_special_movement_step(
                    runtime,
                    map,
                    who,
                    session,
                    self.owner_now().get(),
                    Some(before.position()),
                    after.position(),
                    optional_pvp,
                );
                states.stage_underwater_step(
                    runtime,
                    who,
                    session,
                    self.owner_now().get(),
                    underwater(map, before.position()),
                    underwater(map, after.position()),
                );
            }
        }
        UseDisposition::Committed
    }
}
