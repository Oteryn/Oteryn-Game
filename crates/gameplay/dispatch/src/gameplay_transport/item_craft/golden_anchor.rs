//! Crystal201189f871d23597b82caf88fd611454a2ddefb8,
//! data-global/scripts/quests/deeplings_worldchange/actions_golden_anchor.lua5–14.
//! SHA256c05680d1ca8ac6ea98dd204cfaaaef13167a29ac7d78adb97877487c49d5b254.
//! The source item is retained. Existing fenced movement/conditions own the transfer.
use super::*;
use crate::foundation::MovementLocalPosition;
use crate::gameplay_transport::paced_movement::underwater_drown::underwater;

const SMALL_ANCHOR: &str = "oteryn:item.tibia.i14019";
const DESTINATION_MAP: crate::map::overlay::TilePos = crate::map::overlay::TilePos {
    x: 33640,
    y: 31379,
    floor: -9,
};

fn runtime_destination() -> Option<MovementLocalPosition> {
    Some(MovementLocalPosition {
        x: i32::from(DESTINATION_MAP.x),
        y: i32::from(DESTINATION_MAP.y),
        floor: i16::from(DESTINATION_MAP.floor).checked_neg()?,
    })
}

fn target_key(key: &str) -> bool {
    matches!(key, "oteryn:item.tibia.i13941" | "oteryn:item.tibia.i13942")
}
fn carried_anchor(
    inventory: &crate::durability::item_use::ItemUseInventory,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    intent: &UseIntent,
    source: Option<ItemKey>,
) -> Option<[u8; 16]> {
    let uuid = match intent.target {
        UseTarget::Item(_) => match source {
            Some(ItemKey::Instance(uuid)) => uuid,
            _ => return None,
        },
        UseTarget::ItemByDefinition(wanted) => {
            let (key, revision) = index.definition_at(wanted)?;
            if key != SMALL_ANCHOR {
                return None;
            }
            inventory
                .first_unreserved(
                    &TypedDefinitionRef {
                        family: "Item".into(),
                        production_key: key.into(),
                        revision_ref: revision.into(),
                    },
                    &[],
                )
                .ok()?
        }
        _ => return None,
    };
    inventory
        .backpack
        .iter()
        .flat_map(|bag| &bag.entries)
        .chain(inventory.nested.iter().map(|(_, entry)| entry))
        .find(|entry| {
            entry.item.item_instance_id == uuid
                && entry.item.quantity == 1
                && entry.item.definition.family == "Item"
                && entry.item.definition.production_key == SMALL_ANCHOR
                && index
                    .definition_ref("Item", SMALL_ANCHOR, &entry.item.definition.revision_ref)
                    .is_some()
        })
        .map(|entry| entry.item.item_instance_id)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_golden_anchor(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<ItemKey>,
        target: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, ()> {
        if intent.use_with.is_some()
            || intent.use_with_position.is_some()
            || intent.use_with_map_item.is_none()
        {
            return Ok(None);
        }
        let Some(fence) = command.item_fence else {
            return Ok(None);
        };
        let Some(native) = self
            .active_generation
            .and_then(crate::content::ActiveGeneration::native_gameplay)
        else {
            return Ok(None);
        };
        let Some(map) = self.bundle_map() else {
            return Ok(None);
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return Err(());
        };
        let index = {
            let runtime = self.runtime.lock().await;
            pinned_index(&runtime, self.active_generation).ok_or(())?
        };
        // Cheap source-target classification avoids reading carried inventory for every
        // unrelated USE-with. Resolution is repeated under the final movement locks.
        {
            let mut runtime = self.runtime.lock().await;
            let Some(at) =
                super::super::bundle_map_position(&mut runtime, self.movement_cells, actor)
            else {
                return Ok(None);
            };
            let facts = at.map.facts.pinned();
            let view = super::super::world_map::MapViewSource {
                overlay: &at.map.overlay,
                facts: &facts,
                content_generation: at.content_generation,
                reset_epoch: self.reset_epoch,
            };
            let resolved = resolve_anchor(&view, index, intent, target);
            if !matches!(resolved, Ok(Some(_))) {
                return Ok(None);
            }
        }
        let inventory = self
            .root
            .read_item_use_inventory(self.character, self.holder, fence)
            .await
            .map_err(|_| ())?;
        let Some(uuid) = carried_anchor(&inventory, index, intent, source) else {
            return Ok(None);
        };
        if !self
            .states
            .player_trades
            .physical(command.game_session_id, Some(uuid))
        {
            return Ok(Some(UseOutcome::rejected()));
        }
        if native.item_policy(SMALL_ANCHOR, "definition-r1").is_none() {
            return Ok(Some(UseOutcome::rejected()));
        }
        let optional_pvp = self
            .special_movement_policy(&permit, actor, command.game_session_id)
            .await;
        if !self.refresh_pvp_locks(actor, command.game_session_id).await {
            return Ok(Some(UseOutcome::rejected()));
        }
        let mut runtime = self.runtime.lock().await;
        let mut states = self.states.spell_states.lock().await;
        let session = command.game_session_id;
        if runtime.binding().runtime_scope() != fence.runtime_scope
            || runtime.binding().scope_generation() != fence.scope_ownership_generation
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, session)
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|f| f.control_loss.is_none())
            || !runtime
                .bound_attacker_lease(actor, session)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
        {
            return Ok(Some(UseOutcome::rejected()));
        }
        let Some(at) = super::super::bundle_map_position(&mut runtime, self.movement_cells, actor)
        else {
            return Ok(Some(UseOutcome::rejected()));
        };
        let facts = at.map.facts.pinned();
        let view = super::super::world_map::MapViewSource {
            overlay: &at.map.overlay,
            facts: &facts,
            content_generation: at.content_generation,
            reset_epoch: self.reset_epoch,
        };
        let position = match resolve_anchor(&view, index, intent, target) {
            Ok(Some(position)) => position,
            Ok(None) => {
                return Ok(Some(UseOutcome {
                    disposition: UseDisposition::NothingToUse,
                    ..UseOutcome::rejected()
                }));
            }
            Err(()) => {
                return Ok(Some(UseOutcome {
                    disposition: UseDisposition::StaleState,
                    ..UseOutcome::rejected()
                }));
            }
        };
        // Map targets use native negative floors; the movement owner uses
        // legacy runtime z. Convert once, with checked arithmetic at the boundary.
        let expected = runtime.read_actor_position(actor).map_err(|_| ())?;
        let target_runtime_floor = i16::from(position.floor).checked_neg().ok_or(())?;
        let origin = expected.position();
        if target_runtime_floor != origin.floor
            || origin.x.abs_diff(i32::from(position.x)) > 1
            || origin.y.abs_diff(i32::from(position.y)) > 1
        {
            return Ok(Some(UseOutcome {
                disposition: UseDisposition::TooFar,
                ..UseOutcome::rejected()
            }));
        }
        let destination = runtime_destination().ok_or(())?;
        let closed = crate::map::floor::BundleDoors(map.facts.doors());
        let blocked = super::super::pvp_live::pz_blocked(&states, actor, session, self.owner_now());
        let proof = match crate::map::floor::prepare_quest_callback_teleport(
            &runtime,
            self.movement_cells,
            actor,
            session,
            expected,
            destination,
            blocked,
            &closed,
        ) {
            Ok(proof) => proof,
            Err(_) => {
                return Ok(Some(UseOutcome {
                    disposition: UseDisposition::NotPossible,
                    ..UseOutcome::rejected()
                }));
            }
        };
        let snapshot = match runtime.commit_source_step(proof) {
            Ok(snapshot) => snapshot,
            Err(_) => return Ok(Some(UseOutcome::rejected())),
        };
        states.stage_special_movement_step(
            &runtime,
            map,
            actor,
            session,
            self.owner_now().get(),
            Some(expected.position()),
            snapshot.position(),
            optional_pvp,
        );
        states.stage_underwater_step(
            &runtime,
            actor,
            session,
            self.owner_now().get(),
            underwater(map, expected.position()),
            underwater(map, snapshot.position()),
        );
        // Source CONST_ME_TELEPORT=11 (utils_definitions.hpp66), at the destination.
        states
            .sounds
            .record_item_effect(&runtime, actor, snapshot.position(), 11, false);
        self.states.chat.close_out_of_range(&runtime, session);
        Ok(Some(UseOutcome {
            disposition: UseDisposition::Committed,
            ..UseOutcome::rejected()
        }))
    }
}

fn resolve_anchor<F: crate::map::view::MapFacts>(
    map: &super::super::world_map::MapViewSource<'_, F>,
    index: &crate::content::item_ref::ItemDefinitionIndex,
    intent: &UseIntent,
    target: Option<ItemKey>,
) -> Result<Option<crate::map::overlay::TilePos>, ()> {
    let (pos, key) = match &intent.use_with_map_item {
        Some(MapUseTarget::Object(object)) => {
            let t = super::super::world_map::resolve_map_target(map, object, |_| None::<()>)
                .map_err(|_| ())?;
            (t.pos, MapEntryKey::Base(t.placement_key))
        }
        Some(MapUseTarget::Item(_)) => match target
            .and_then(|key| super::super::world_map::resolve_map_handle(map, key).ok())
        {
            Some(super::super::world_map::MapHandleTarget::Base {
                pos, placement_key, ..
            }) => (pos, MapEntryKey::Base(placement_key)),
            _ => return Err(()),
        },
        None => return Ok(None),
    };
    let tile = view::compose_tile(map.overlay, map.facts, pos)
        .map_err(|_| ())?
        .ok_or(())?;
    let entry = tile
        .entries
        .iter()
        .find(|entry| entry.source.key() == key)
        .ok_or(())?;
    let MapDefinition::Item(reference) = entry.facts.definition else {
        return Ok(None);
    };
    let Some((key, _)) = index.definition_at(reference) else {
        return Ok(None);
    };
    if !target_key(key)
        || entry.facts.bound
        || map.facts.house_tile(pos)
        || entry.facts.count != 1
        || entry.facts.sub_type != 0
    {
        return Ok(None);
    }
    Ok(Some(pos))
}
