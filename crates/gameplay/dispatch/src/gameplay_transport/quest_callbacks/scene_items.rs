//! Source scene custody changes reuse the map MINT and durable item-loop owners.
use super::*;
use crate::content::ReferenceItemField;
use crate::durability::item_loop_move::{
    LoopDestination, LoopGround, LoopMoveRequest, SourceSceneItemMove,
};
use crate::gameplay_transport::{
    item_move::{LoopMove, LoopTarget},
    item_ref_admission::NativeItemWeights,
    item_view::ItemKey,
    loop_move,
};
use crate::map::{overlay::AddedItem, view::EntryFacts};
use oteryn_protocol_oteryn::item_view::WorldTilePosition;
use sha2::{Digest, Sha256};

pub(super) struct PreparedSceneItems {
    fence: crate::durability::item_transfer::CurrentCharacterItemFence,
    moves: Vec<SourceSceneItemMove>,
    destinations: Vec<(ItemKey, WorldTilePosition)>,
}

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(clippy::too_many_arguments)]
    pub(super) async fn prepare_scene_items(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        command: UseCommand,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        revision: u64,
        fence: Option<CurrentCharacterGameplayFence>,
    ) -> Result<Option<PreparedSceneItems>, UseDisposition> {
        let refusal = UseDisposition::Rejected;
        let Some(scene) = callback
            .effects
            .scene
            .as_ref()
            .filter(|s| s.relocate_items_close)
        else {
            return Ok(None);
        };
        let Some((position, ordinal)) = crate::map::view::placement_of(key) else {
            return Err(refusal);
        };
        let Some(base) = map
            .overlay
            .base()
            .tile(position.x, position.y, position.floor)
        else {
            return Err(refusal);
        };
        let Some(id) = base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, d)| **d == 0)
            .nth(usize::from(ordinal))
            .map(|(id, _)| *id)
        else {
            return Err(refusal);
        };
        let Some(facts) = map.facts.base_entry(position, ordinal, id) else {
            return Err(refusal);
        };
        if !callback
            .lever
            .is_some_and(|pair| facts.appearance_id == pair[1])
        {
            return Ok(None);
        }
        let Some(native_state) = self.active_generation.and_then(|g| g.native_gameplay()) else {
            return Err(refusal);
        };
        let (index, context) = {
            let runtime = self.runtime.lock().await;
            let Ok(expected) = runtime.read_actor_position(actor) else {
                return Err(refusal);
            };
            if map.facts.object_revision(key) != revision
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|f| f.control_loss.is_none())
                || !native(expected.position())
                    .is_some_and(|at| super::super::map_door::reachable(at, position))
            {
                return Err(refusal);
            }
            let Some(index) =
                super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)
            else {
                return Err(refusal);
            };
            (
                index,
                QuestWorldContext {
                    map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                    content_revision: format!(
                        "sha256:{}",
                        hex(&runtime.content_pin().server_artifact_digest())
                    ),
                    placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                    reset_epoch: self.reset_epoch,
                    actor_placement: actor.placement_identity(),
                    movement_revision: expected.revision(),
                    party: Vec::new(),
                },
            )
        };
        let Some(ground) = loop_move::loop_ground(self, map.overlay.base(), Some(index)).await
        else {
            return Err(refusal);
        };
        let movable = |facts: &EntryFacts| {
            let oteryn_protocol_oteryn::world_map::MapDefinition::Item(reference) =
                facts.definition
            else {
                return false;
            };
            let Some((key, revision)) = index.definition_at(reference) else {
                return false;
            };
            native_state
                .item_policy(key, revision)
                .is_some_and(|policy| {
                    matches!(&policy.record().semantics.physical,
                    ReferenceItemField::Known(physical)
                    if physical.movable == ReferenceItemField::Known(true))
                })
        };
        let mut items = Vec::new();
        for movement in &scene.evacuate_close {
            let from = tile(movement.from);
            let to = tile(movement.players_to);
            let Some(entries) = super::super::paced_movement::special_movement::tile_entries(
                map,
                local(movement.from),
            ) else {
                return Err(refusal);
            };
            for (key, facts) in entries.entries {
                if !movable(&facts) {
                    continue;
                }
                let Some((_, ordinal)) = crate::map::view::placement_of(key) else {
                    return Err(refusal);
                };
                let Some(pending) = self.states.loop_ground.pending_origin(key) else {
                    return Err(refusal);
                };
                if pending.is_some() {
                    return Err(refusal);
                }
                if !ground
                    .overlay()
                    .tile(from)
                    .is_some_and(|tile| tile.is_hidden(ordinal))
                {
                    items.push((
                        ItemKey::MapBase {
                            bundle_digest: map.overlay.base().digest(),
                            placement_key: key,
                            reset_epoch: self.reset_epoch,
                        },
                        from,
                        to,
                    ));
                }
            }
            if let Some(tile) = ground.overlay().tile(from) {
                for entry in tile.added() {
                    if let AddedItem::Ground(item) = entry.item() {
                        let Some(facts) = ground.facts(&map.facts).added_entry(entry.item()) else {
                            return Err(refusal);
                        };
                        if movable(&facts) {
                            items.push((ItemKey::Instance(item.item_instance_id), from, to));
                        }
                    }
                }
            }
        }
        if items.len() > 32 {
            return Err(refusal);
        }
        if items.is_empty() {
            return Ok(None);
        }
        // Stateless scenes need Character authority only when moving item custody.
        let fence = match fence {
            Some(fence) => fence,
            None => match self.current_quest_fence(command.game_session_id).await {
                Ok(Some(fence)) => fence,
                _ => return Err(refusal),
            },
        };
        let Some(catalogue) = self.quest_catalogue.as_ref() else {
            return Err(refusal);
        };
        if command.item_fence != Some(fence.item_fence()) {
            return Err(refusal);
        }
        if !matches!(
            self.root
                .check_character_world_callback(
                    self.character,
                    self.holder,
                    fence,
                    &callback.key,
                    std::sync::Arc::clone(catalogue),
                    permit,
                    context.clone()
                )
                .await,
            Ok(true)
        ) {
            #[cfg(test)]
            eprintln!("QUEST_SCENE_ITEMS_REFUSED durable_gates");
            return Err(refusal);
        }
        let mut moves = Vec::with_capacity(items.len());
        let mut destinations = Vec::with_capacity(items.len());
        for (item, from, to) in items {
            let mut hash = Sha256::new()
                .chain_update(b"oteryn:quest-scene-item:v1")
                .chain_update(command.command_id.to_be_bytes())
                .chain_update(key.to_be_bytes())
                .chain_update(self.reset_epoch.to_be_bytes());
            match item {
                ItemKey::Instance(id) => hash.update(id),
                ItemKey::MapBase {
                    bundle_digest,
                    placement_key,
                    ..
                } => {
                    hash.update(bundle_digest);
                    hash.update(placement_key.to_be_bytes());
                }
                _ => return Err(refusal),
            }
            let digest = hash.finalize();
            let mut bytes = [0; 8];
            bytes.copy_from_slice(&digest[..8]);
            let id = CommandId::new(u64::from_be_bytes(bytes).max(1)).map_err(|_| refusal)?;
            let command_ref = CommandRef::new(command.game_session_id, id);
            let to = WorldTilePosition {
                x: i32::from(to.x),
                y: i32::from(to.y),
                floor: i16::from(to.floor),
            };
            let spatial_position = loop_move::source_drop_cell(
                map.overlay.base(),
                (i32::from(from.x), i32::from(from.y), i16::from(from.floor)),
                to,
            )
            .map_err(|_| refusal)?;
            let destination = LoopDestination::Ground(LoopGround {
                spatial_position,
                map_revision: context.map_revision.clone(),
                content_revision: context.content_revision.clone(),
                placement_context: context.placement_context.clone(),
            });
            let (instance, mint) = match item {
                ItemKey::Instance(id) => (id, None),
                ItemKey::MapBase { .. } => {
                    let prepared = loop_move::map_pickup::prepare_map_item_mint(
                        map,
                        native_state,
                        index,
                        LoopMove {
                            item,
                            count: None,
                            target: LoopTarget::Ground(to),
                            slot: None,
                        },
                        command_ref,
                        self.reset_epoch,
                        Some(from),
                        &destination,
                    )
                    .map_err(|_| refusal)?;
                    let mint = self
                        .root
                        .freeze_map_item_mint(
                            self.character,
                            self.holder,
                            fence.item_fence(),
                            prepared.mint,
                        )
                        .await
                        .map_err(|_| refusal)?;
                    (*mint.item_instance_id(), Some(mint))
                }
                _ => return Err(refusal),
            };
            moves.push(SourceSceneItemMove {
                command: command_ref,
                request: LoopMoveRequest {
                    item: instance,
                    count: None,
                    pickup_cell: Some(
                        crate::durability::map_item_mint_audit::map_item_spatial_position(
                            from.x, from.y, from.floor,
                        ),
                    ),
                    destination,
                },
                mint,
            });
            destinations.push((ItemKey::Instance(instance), to));
        }
        Ok(Some(PreparedSceneItems {
            fence: fence.item_fence(),
            moves,
            destinations,
        }))
    }

    pub(super) async fn commit_scene_items(
        &self,
        permit: &SpellLanePermit,
        prepared: PreparedSceneItems,
    ) -> bool {
        let Some(native) = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            return false;
        };
        let result = self
            .root
            .commit_source_scene_item_moves(
                permit,
                self.character,
                self.holder,
                prepared.fence,
                &NativeItemWeights(native).with_wheel(self.premium_coordinator),
                prepared.moves,
            )
            .await;
        if !matches!(result, Ok(Err(_))) {
            self.states.loop_ground.invalidate();
        }
        if !matches!(result, Ok(Ok(()))) {
            #[cfg(test)]
            eprintln!("QUEST_SCENE_ITEMS_REFUSED batch={result:?}");
            return false;
        }
        for (item, destination) in prepared.destinations {
            self.states
                .browse_fields
                .committed_move(item, Some(destination));
        }
        true
    }
}
