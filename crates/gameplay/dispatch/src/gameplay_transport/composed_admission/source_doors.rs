//! Custom source doors retain the ordinary door CAS and sealed movement owners.
use super::*;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::actor_spell::ChannelSpellStates;
use crate::gameplay_transport::paced_movement::underwater_drown::underwater;
use crate::gameplay_transport::world_map::{MapViewSource, resolve_map_target};
use crate::map::overlay::TilePos;
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "one owner turn binds the actor/session, current storage, target and movement"
    )]
    pub(super) fn open_source_quest_door(
        &self,
        runtime: &mut ChannelRuntimeV1,
        states: &mut ChannelSpellStates,
        actor: ExactActorRef,
        session: GameSessionId,
        map: &crate::map::boot::BundleMap,
        target: &WorldObjectTarget,
        actor_tile: TilePos,
        actor_level: u32,
        storage_met: bool,
    ) -> Option<UseOutcome> {
        let digest = runtime.content_pin().client_artifact_digest();
        let source = MapViewSource {
            overlay: &map.overlay,
            facts: &map.facts,
            content_generation: digest,
            reset_epoch: 0,
        };
        let resolved = resolve_map_target(&source, target, |_| None::<()>).ok()?;
        let rule = oteryn_gameplay_quests::quest::source_doors::rule(
            map.facts.action_at(resolved.placement_key)?,
        )?;
        if !map_door::reachable(actor_tile, resolved.pos)
            || !map.facts.doors().contains(resolved.placement_key)
        {
            return None;
        }
        if !storage_met {
            return Some(UseOutcome {
                disposition: UseDisposition::Locked,
                ..UseOutcome::rejected()
            });
        }
        let base = map
            .overlay
            .base()
            .tile(resolved.pos.x, resolved.pos.y, resolved.pos.floor)?;
        let id = *base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, depth)| **depth == 0)
            .nth(usize::from(resolved.ordinal))?
            .0;
        let entry = map.facts.base_entry(resolved.pos, resolved.ordinal, id)?;
        let entry = map.facts.doors().facts(resolved.placement_key, entry, None);
        if !rule.appearances.contains(&entry.appearance_id) {
            return Some(UseOutcome {
                disposition: UseDisposition::NothingToUse,
                ..UseOutcome::rejected()
            });
        }
        if !rule.teleport_on_open
            || map.facts.doors().is_open(resolved.placement_key) != Some(false)
        {
            return None;
        }
        if !states.spell_idle(runtime, actor, session)
            || !self.states.player_trades.physical(session, None)
            || self.has_pending_quest_callback(session)
        {
            return Some(UseOutcome::rejected());
        }
        let expected = runtime.read_actor_position(actor).ok()?;
        let destination = MovementLocalPosition {
            x: i32::from(resolved.pos.x),
            y: i32::from(resolved.pos.y),
            floor: -i16::from(resolved.pos.floor),
        };
        let blockers =
            crate::map::floor::BundleDoorsExcept(map.facts.doors(), resolved.placement_key);
        let pz_blocked =
            super::super::pvp_live::pz_blocked(states, actor, session, self.owner_now());
        let Ok(proof) = crate::map::floor::prepare_quest_door_opening(
            runtime,
            self.movement_cells,
            actor,
            session,
            expected,
            destination,
            pz_blocked,
            &blockers,
            resolved.placement_key,
        ) else {
            return Some(UseOutcome {
                disposition: UseDisposition::NotPossible,
                ..UseOutcome::rejected()
            });
        };
        let occupied = map.overlay.has_added_item(resolved.pos)
            || map_door::doorway_occupied(runtime, actor, actor_tile, resolved.pos);
        let mut outcome = UseOutcome::rejected();
        let snapshot = runtime.commit_source_step_with_projection(proof, || {
            outcome = map_door::use_door(
                &map.overlay,
                &map.facts,
                digest,
                actor_tile,
                |_| occupied,
                actor_level,
                storage_met,
                target,
            );
            outcome.disposition == UseDisposition::Committed
        });
        if let Ok(snapshot) = snapshot {
            states.stage_special_movement_step(
                runtime,
                map,
                actor,
                session,
                self.owner_now().get(),
                Some(expected.position()),
                snapshot.position(),
                None,
            );
            states.stage_underwater_step(
                runtime,
                actor,
                session,
                self.owner_now().get(),
                underwater(map, expected.position()),
                underwater(map, snapshot.position()),
            );
        }
        Some(outcome)
    }
}
