//! Fixed native ladders borrow the existing source-step movement owner.
//! Canary04b83 data/scripts/actions/items/ladder_up.lua:5-17 SHA256
//! 730bc7664484e953b6efc3c8faeba9936d355c5cdde91d1f3e0686f714f4df13;
//! data/libs/functions/position.lua:22-49 SHA256
//! d933c6df3c0d0e0e974048bf0a58063f14d7f872adca2d5bb0250d6cdc261920.
use super::world_map::{MapTargetRefusal, MapViewSource, resolve_map_target};
use super::{
    ComposedFreshAdmission,
    connection::{UseCommand, UseOutcome},
};
use crate::content::{ReferenceItemField, ReferenceItemType};
use crate::foundation::{ExactActorRef, MovementLocalPosition};
use crate::map::{
    boot::BundleMap,
    view::{self, MapFacts},
};
use oteryn_protocol_oteryn::{
    world_map::MapDefinition,
    world_object::{UseDisposition, WorldObjectTarget},
};

// Position:moveUpstairs prefers south, then N/E/W/SW/SE/NW/NE. The donor's
// repeated west check has identical facts within this locked owner turn.
const LANDINGS: &[(i32, i32)] = &[
    (0, 1),
    (0, -1),
    (1, 0),
    (-1, 0),
    (-1, 1),
    (1, 1),
    (-1, -1),
    (1, -1),
];

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_bundle_ladder(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        let native = self.active_generation?.native_gameplay()?;
        let _permit = self.spell_lane_permit().await?;
        let resolved = {
            let facts = map.facts.pinned();
            let source = MapViewSource {
                overlay: &map.overlay,
                facts: &facts,
                content_generation: self.active_generation?.identity().client_artifact_digest(),
                reset_epoch: self.reset_epoch,
            };
            let resolved = match resolve_map_target(&source, target, |_| None::<()>) {
                Ok(value) => value,
                Err(MapTargetRefusal::StateRevisionMismatch) => {
                    return Some(result(UseDisposition::StaleState));
                }
                Err(MapTargetRefusal::Use(disposition)) => return Some(result(disposition)),
            };
            let tile = view::compose_tile(&map.overlay, &facts, resolved.pos).ok()??;
            let before = tile
                .entries
                .iter()
                .find(|entry| {
                    entry.source.key() == view::MapEntryKey::Base(resolved.placement_key)
                })?
                .facts;
            let MapDefinition::Item(reference) = before.definition else {
                return None;
            };
            let index = native.item_index()?;
            let definition = super::item_look::typed_definition(index, reference)?;
            let policy = native
                .item_policy(&definition.production_key, &definition.revision_ref)?
                .record();
            if !matches!(&policy.semantics.classification,ReferenceItemField::Known(value)
            if value.item_type==ReferenceItemField::Known(ReferenceItemType::Ladder))
            {
                return None;
            }
            if facts.house_tile(resolved.pos)
                || before.bound
                || !matches!(&policy.semantics.physical,
            ReferenceItemField::Known(value) if value.pickupable==ReferenceItemField::Known(false)
                && value.movable==ReferenceItemField::Known(false))
            {
                return Some(result(UseDisposition::NothingToUse));
            }
            resolved
        };
        if !self.refresh_pvp_locks(actor, command.game_session_id).await {
            return Some(result(UseDisposition::Rejected));
        }
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        if states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || super::item_ref_admission::pinned_index(&runtime, self.active_generation).is_none()
            || !runtime
                .player_control_facts(actor, command.game_session_id)
                .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return Some(result(UseDisposition::Rejected));
        }
        // Re-prove visibility and revision after the asynchronous PZ refresh, under
        // the same Channel turn as commit; a hidden or replaced base entry is stale.
        {
            let current = map.facts.pinned();
            let source = MapViewSource {
                overlay: &map.overlay,
                facts: &current,
                content_generation: self.active_generation?.identity().client_artifact_digest(),
                reset_epoch: self.reset_epoch,
            };
            match resolve_map_target(&source, target, |_| None::<()>) {
                Ok(_) => {}
                Err(MapTargetRefusal::StateRevisionMismatch) => {
                    return Some(result(UseDisposition::StaleState));
                }
                Err(MapTargetRefusal::Use(disposition)) => return Some(result(disposition)),
            }
        }
        let Ok(expected) = runtime.read_actor_position(actor) else {
            return Some(result(UseDisposition::Rejected));
        };
        let origin = expected.position();
        if origin.floor != -i16::from(resolved.pos.floor)
            || origin.x.abs_diff(i32::from(resolved.pos.x)) > 1
            || origin.y.abs_diff(i32::from(resolved.pos.y)) > 1
        {
            return Some(result(UseDisposition::TooFar));
        }
        if !self
            .states
            .world_gate
            .lock()
            .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
        {
            return Some(result(UseDisposition::Exhausted));
        }
        let Some(floor) = origin
            .floor
            .checked_sub(1)
            .filter(|floor| (0..=15).contains(floor))
        else {
            return Some(result(UseDisposition::NotPossible));
        };
        let closed = crate::map::floor::BundleDoors(&map.facts);
        let mut landing = None;
        for &(dx, dy) in LANDINGS {
            let destination = MovementLocalPosition {
                x: i32::from(resolved.pos.x) + dx,
                y: i32::from(resolved.pos.y) + dy,
                floor,
            };
            // Select geometry first. Source PZ refusal applies to that selected landing;
            // it must not search for a different exit around the protection-zone rule.
            if crate::map::floor::prepare_quest_callback_teleport(
                &runtime,
                &self.movement_cells,
                actor,
                command.game_session_id,
                expected,
                destination,
                false,
                &closed,
            )
            .is_ok()
            {
                landing = Some(destination);
                break;
            }
        }
        let Some(landing) = landing else {
            return Some(result(UseDisposition::NotPossible));
        };
        let now = self.owner_now();
        let blocked = super::pvp_live::pz_blocked(&states, actor, command.game_session_id, now);
        let Ok(proof) = crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            &self.movement_cells,
            actor,
            command.game_session_id,
            expected,
            landing,
            blocked,
            &closed,
        ) else {
            return Some(result(UseDisposition::NotPossible));
        };
        use oteryn_gameplay_spell_rules::cast::{STAIRHOP_COOLDOWN_KEY, STAIRHOP_MICROS};
        let Ok(until) = now.checked_add(STAIRHOP_MICROS) else {
            return Some(result(UseDisposition::Rejected));
        };
        let cooldowns = states
            .get(&runtime, actor, command.game_session_id)
            .map(|state| {
                let mut next = state.cooldowns.clone();
                next.groups.insert(STAIRHOP_COOLDOWN_KEY.into(), until);
                next
            });
        if runtime.commit_source_step(proof).is_err() {
            return Some(result(UseDisposition::Rejected));
        }
        if let Some(cooldowns) = cooldowns
            && let Some(state) = states.get_mut(&runtime, actor, command.game_session_id)
        {
            state.cooldowns = cooldowns;
        }
        self.states
            .chat
            .close_out_of_range(&runtime, command.game_session_id);
        Some(result(UseDisposition::Committed))
    }
}

fn result(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        ..UseOutcome::rejected()
    }
}
