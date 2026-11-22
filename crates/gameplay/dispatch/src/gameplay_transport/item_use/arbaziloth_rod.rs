//! Crystal 201189f8 fishing.lua routes the owned fishing rod into Arbaziloth's shield.
use super::*;
use oteryn_gameplay_world::map::boot::BundleCollisionIndexExt;
use oteryn_protocol_oteryn::world_object::{UseIntent, UseTarget, encode_item_use_intent};

use oteryn_gameplay_items::fishing::FISHING_ROD as ROD;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn use_arbaziloth_rod(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
    ) -> Option<UseOutcome> {
        let wanted = intent.use_with?;
        let fence = command.item_fence?;
        let session = command.game_session_id;
        let wire: [u8; 32] = Sha256::digest(encode_item_use_intent(intent).ok()?).into();
        let outcome = |disposition| {
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        };
        let resolve_boss = |runtime: &ChannelRuntimeV1| {
            runtime
                .visible_entities()
                .creatures
                .iter()
                .find(|entry| {
                    entry.actor.placement_identity() == wanted.identity
                        && entry.generation == wanted.generation
                        && runtime.arbaziloth_shields(entry.actor).is_some()
                })
                .map(|entry| entry.actor)
        };
        let boss = {
            let runtime = self.runtime.lock().await;
            if !rotten_essence::current(&runtime, actor, command) {
                return None;
            }
            let mut states = self.states.spell_states.lock().await;
            let gate = states.item_uses.gate(&runtime, actor, session)?;
            if let Some((id, previous, disposition)) = gate.arbaziloth_rod_last
                && id == command.command_id
            {
                return outcome(if previous == wire {
                    disposition
                } else {
                    UseDisposition::Rejected
                });
            }
            resolve_boss(&runtime)?
        };
        let _lane = self.spell_lane_permit().await?;
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .ok()?;
        let mut runtime = self.runtime.lock().await;
        if !rotten_essence::current(&runtime, actor, command) {
            return None;
        }
        let index =
            super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)?;
        let mut states = self.states.spell_states.lock().await;
        if states.is_dead(actor) || !states.spell_idle(&runtime, actor, session) {
            return None;
        }
        let gate = states.item_uses.gate(&runtime, actor, session)?;
        let item = match intent.target {
            UseTarget::Item(_) => match source? {
                ItemKey::Instance(id) => id,
                _ => return None,
            },
            UseTarget::ItemByDefinition(reference) => {
                let (key, revision) = index.definition_at(reference)?;
                if key != ROD {
                    return None;
                }
                inventory
                    .first_unreserved(
                        &TypedDefinitionRef {
                            family: "Item".into(),
                            production_key: key.into(),
                            revision_ref: revision.into(),
                        },
                        &gate.unspendable,
                    )
                    .ok()?
            }
            UseTarget::WorldObject(_) => return None,
        };
        if gate.unspendable.contains(&item)
            || !self.states.player_trades.physical(session, Some(item))
            || !inventory
                .backpack
                .as_ref()?
                .entries
                .iter()
                .chain(inventory.nested.iter().map(|(_, entry)| entry))
                .any(|entry| {
                    entry.item.item_instance_id == item
                        && entry.item.definition.production_key == ROD
                })
        {
            return None;
        }
        if resolve_boss(&runtime) != Some(boss) {
            return outcome(UseDisposition::NotPossible);
        }
        let own = runtime.read_actor_position(actor).ok()?.position();
        let target = runtime.read_actor_position(boss).ok()?.position();
        // Crystal fishing.lua enables far use; Actions::canUseFar checks <7, 5>,
        // same floor and sight.
        let cells = self.movement_cells();
        let crate::content::native_cell_lookup::NativeMovementCollisionIndex::Bundle(index) =
            cells.index()
        else {
            return outcome(UseDisposition::NotPossible);
        };
        let doors = crate::map::floor::BundleDoors(&index.map().facts);
        if own.floor != target.floor
            || own.x.abs_diff(target.x) > 7
            || own.y.abs_diff(target.y) > 5
            || !crate::movement::relocate::creature_sight_clear(cells, &doors, own, target)
        {
            return outcome(UseDisposition::NotPossible);
        }
        let now_ms = self.owner_ms();
        if let Err(disposition) = gate.admit(command.command_id, now_ms) {
            return outcome(disposition);
        }
        let digest = Sha256::new()
            .chain_update(session.as_bytes())
            .chain_update(command.command_id.to_be_bytes())
            .chain_update(wire)
            .finalize();
        let mut occurrence = [0; 16];
        occurrence.copy_from_slice(&digest[..16]);
        let draw = deterministic_decision_u64(
            &GameplayDecisionRoot::from_bytes(runtime.content_pin().server_artifact_digest()),
            DecisionOccurrenceId::from_bytes(occurrence),
            "arbaziloth.fishing_rod",
            0,
        )
        .ok()?;
        let roll = u8::try_from(crate::spell::uniform_draw(draw, 1, 100)).ok()?;
        let disposition = if runtime.arbaziloth_shields_mut(boss)?.use_rod(roll, now_ms) {
            UseDisposition::Committed
        } else {
            UseDisposition::NotPossible
        };
        gate.arbaziloth_rod_last = Some((command.command_id, wire, disposition));
        outcome(disposition)
    }
}
