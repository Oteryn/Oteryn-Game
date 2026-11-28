//! A sword cuts an untagged base placement through the current Channel owner.
use super::{destroy_roll, destroy_roll_succeeds, is_destroy_weapon, item_id};
use crate::content::ReferenceItemField;
use crate::foundation::{ExactActorRef, RuntimeScopeRefV1};
use crate::gameplay_transport::connection::{UseCommand, UseOutcome};
use crate::gameplay_transport::item_view::ItemKey;
use crate::gameplay_transport::world_map::{
    MapHandleTarget, MapViewSource, resolve_map_handle, resolve_map_target,
};
use crate::gameplay_transport::{ComposedFreshAdmission, bundle_map_position, item_ref_admission};
use crate::map::boot::BundleMap;
use crate::map::overlay::TilePos;
use crate::map::view::{EntryFacts, MapFacts};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};
use sha2::{Digest, Sha256};

pub enum Target {
    Object(WorldObjectTarget),
    Handle(ItemKey),
}

// Crystal 201189f8 register_actions.lua:110-116. The map timer restores the
// exact original placement even when several bar appearances share debris.
fn debris(appearance: u16) -> Option<u16> {
    match appearance {
        2291 | 2292 => Some(3146),
        2293..=2296 => Some(3145),
        _ => None,
    }
}

struct Bar {
    key: u64,
    pos: TilePos,
    original: u16,
    expected: u64,
    current: u64,
    before: EntryFacts,
    debris: u16,
}

impl Bar {
    fn read(
        map: &BundleMap,
        content_generation: [u8; 32],
        reset_epoch: u64,
        target: Target,
    ) -> Option<Self> {
        let facts = map.facts.pinned();
        let source = MapViewSource {
            overlay: &map.overlay,
            facts: &facts,
            content_generation,
            reset_epoch,
        };
        let (pos, ordinal, key, expected) = match target {
            Target::Object(mut object) => {
                // Still identify a bar for stale requests, so unrelated base-object
                // uses with an outdated revision can fall through to their owner.
                let key = u64::from_be_bytes(object.placement.get(32..)?.try_into().ok()?);
                let expected = object.expected_revision;
                object.expected_revision = facts.object_revision(key);
                let resolved = resolve_map_target(&source, &object, |_| None::<()>).ok()?;
                (resolved.pos, resolved.ordinal, key, expected)
            }
            Target::Handle(handle) => {
                let MapHandleTarget::Base {
                    pos,
                    ordinal,
                    placement_key,
                } = resolve_map_handle(&source, handle).ok()?
                else {
                    return None;
                };
                (
                    pos,
                    ordinal,
                    placement_key,
                    facts.object_revision(placement_key),
                )
            }
        };
        let tile = map.overlay.base().tile(pos.x, pos.y, pos.floor)?;
        let id = tile
            .ids()
            .iter()
            .copied()
            .zip(tile.depths())
            .filter_map(|(id, depth)| (*depth == 0).then_some(id))
            .nth(usize::from(ordinal))?;
        let original = map.facts.quest_callback_base_appearance(pos, ordinal, id)?;
        let debris = debris(original)?;
        if !map.facts.quest_callback_native_object(key, id, original) {
            return None;
        }
        Some(Self {
            key,
            pos,
            original,
            expected,
            current: facts.object_revision(key),
            before: facts.base_entry(pos, ordinal, id)?,
            debris,
        })
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn destroy_map_bars_impl(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        source: ItemKey,
        target: Target,
    ) -> Option<UseOutcome> {
        let ItemKey::Instance(weapon) = source else {
            return None;
        };
        let fence = command.item_fence?;
        if !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
            || fence.game_session_id != command.game_session_id
        {
            return None;
        }
        let map = self.bundle_map()?;
        let generation = self.active_generation?;
        let bar = Bar::read(
            map,
            generation.identity().client_artifact_digest(),
            self.reset_epoch,
            target,
        )?;
        let native = generation.native_gameplay()?;
        let _permit = self.spell_lane_permit().await?;
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .ok()?;
        let definition = &inventory
            .equipment
            .iter()
            .chain(
                inventory
                    .backpack
                    .iter()
                    .flat_map(|bag| &bag.entries)
                    .map(|entry| &entry.item),
            )
            .chain(inventory.nested.iter().map(|(_, entry)| &entry.item))
            .find(|item| item.item_instance_id == weapon)?
            .definition;
        if !is_destroy_weapon(item_id(definition)?) {
            return None;
        }
        let policy = native.item_policy(&definition.production_key, &definition.revision_ref)?;
        let ReferenceItemField::Known(weapon) = &policy.record().semantics.weapon else {
            return None;
        };
        let ReferenceItemField::Known(attack) = weapon.attack else {
            return None;
        };
        if !(0..=65535).contains(&attack.0) {
            return None;
        }

        let outcome = |disposition| UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        };
        let mut runtime = self.runtime.lock().await;
        let binding = runtime.binding();
        let states = self.states.spell_states.lock().await;
        let session = command.game_session_id;
        if runtime.owner_fence().is_err()
            || binding.runtime_scope() != fence.runtime_scope
            || binding.scope_generation() != fence.scope_ownership_generation
            || !runtime
                .bound_attacker_lease(actor, session)
                .is_ok_and(|lease| {
                    lease.generation() == fence.character_lease_generation
                        && lease.character_id().as_bytes() == fence.character_id.as_bytes()
                })
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|f| f.control_loss.is_none())
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, session)
            || item_ref_admission::pinned_index(&runtime, self.active_generation).is_none()
        {
            return Some(UseOutcome::rejected());
        }
        let position = bundle_map_position(&mut runtime, self.movement_cells(), actor)?;
        if !crate::gameplay_transport::map_door::reachable(
            crate::gameplay_transport::world_map::tile_pos(position.at)?,
            bar.pos,
        ) {
            return Some(outcome(UseDisposition::TooFar));
        }
        if bar.expected != bar.current || map.facts.object_revision(bar.key) != bar.current {
            return Some(outcome(UseDisposition::StaleState));
        }
        if bar.before.appearance_id != bar.original {
            return Some(outcome(UseDisposition::NothingToUse));
        }
        let remains = map.facts.movement_item_facts(bar.debris)?;
        let index = native.item_index()?;
        let oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference) = remains.definition
        else {
            return None;
        };
        let (key, revision) = index.definition_at(reference)?;
        let policy = native.item_policy(key, revision)?;
        let ReferenceItemField::Known(temporal) = &policy.record().semantics.temporal else {
            return None;
        };
        let ReferenceItemField::Known(duration) = temporal.duration else {
            return None;
        };
        let due = self
            .owner_now()
            .get()
            .checked_add(duration.0.checked_mul(1000)?)?;
        if !self
            .states
            .world_gate
            .lock()
            .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
        {
            return Some(outcome(UseDisposition::Exhausted));
        }
        let digest = Sha256::new()
            .chain_update(b"oteryn.channel.wooden_bars.v1")
            .chain_update(session.as_bytes())
            .chain_update(command.command_id.to_be_bytes())
            .chain_update(bar.key.to_be_bytes())
            .finalize();
        let roll = destroy_roll(
            native.source_digest(),
            digest[..16].try_into().ok()?,
            "channel.wooden_bars.destroy.roll",
        )?;
        if destroy_roll_succeeds(roll, attack.0) {
            if !map
                .facts
                .transform_base_item_until(bar.key, bar.current, bar.before, remains, due)
            {
                return Some(outcome(UseDisposition::StaleState));
            }
            states.owner_wake.notify_one();
        }
        Some(outcome(UseDisposition::Committed))
    }
}
