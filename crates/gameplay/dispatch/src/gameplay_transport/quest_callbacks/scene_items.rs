//! Source scene custody changes reuse the map MINT and durable item-loop owners.
use super::*;
use crate::content::ReferenceItemField;
use crate::gameplay_transport::{item_move::LoopMoveResult, item_view::ItemKey, loop_move};
use crate::map::{overlay::AddedItem, view::EntryFacts};
use sha2::{Digest, Sha256};

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
    ) -> bool {
        let Some(scene) = callback
            .effects
            .scene
            .as_ref()
            .filter(|s| s.relocate_items_close)
        else {
            return true;
        };
        let Some((position, ordinal)) = crate::map::view::placement_of(key) else {
            return false;
        };
        let Some(base) = map
            .overlay
            .base()
            .tile(position.x, position.y, position.floor)
        else {
            return false;
        };
        let Some(id) = base
            .ids()
            .iter()
            .zip(base.depths())
            .filter(|(_, d)| **d == 0)
            .nth(usize::from(ordinal))
            .map(|(id, _)| *id)
        else {
            return false;
        };
        let Some(facts) = map.facts.base_entry(position, ordinal, id) else {
            return false;
        };
        if !callback
            .lever
            .is_some_and(|pair| facts.appearance_id == pair[1])
        {
            return true;
        }
        let Some(native_state) = self.active_generation.and_then(|g| g.native_gameplay()) else {
            return false;
        };
        let (index, context) = {
            let runtime = self.runtime.lock().await;
            let Ok(expected) = runtime.read_actor_position(actor) else {
                return false;
            };
            if map.facts.object_revision(key) != revision
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|f| f.control_loss.is_none())
                || !native(expected.position())
                    .is_some_and(|at| super::super::map_door::reachable(at, position))
                || self.prepare_quest_scene(map, callback, true).is_err()
            {
                return false;
            }
            let closed = crate::map::floor::BundleDoors(&map.facts);
            if let Err(error) = crate::map::floor::prepare_source_scene(
                &runtime,
                &self.movement_cells,
                scene
                    .evacuate_close
                    .iter()
                    .map(|m| crate::map::floor::SceneEvacuation {
                        from: local(m.from),
                        players_to: local(m.players_to),
                        creatures_to: local(m.creatures_to),
                        all: m.all,
                    })
                    .collect(),
                &closed,
            ) {
                #[cfg(test)]
                eprintln!("QUEST_SCENE_ITEMS_REFUSED world={error:?}");
                let _ = error;
                return false;
            }
            let Some(index) =
                super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)
            else {
                return false;
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
            return false;
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
            native_state.item_policy(key,revision).is_some_and(|policy|matches!(&policy.record().semantics.physical,ReferenceItemField::Known(p) if p.movable==ReferenceItemField::Known(true)))
        };
        let mut items = Vec::new();
        for movement in &scene.evacuate_close {
            let from = tile(movement.from);
            let to = tile(movement.players_to);
            let Some(entries) = super::super::paced_movement::special_movement::tile_entries(
                map,
                local(movement.from),
            ) else {
                return false;
            };
            for (key, facts) in entries.entries {
                if !movable(&facts) {
                    continue;
                }
                let Some((_, ordinal)) = crate::map::view::placement_of(key) else {
                    return false;
                };
                let Some(pending) = self.states.loop_ground.pending_origin(key) else {
                    return false;
                };
                if pending.is_some()
                    || !ground
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
                            return false;
                        };
                        if movable(&facts) {
                            items.push((ItemKey::Instance(item.item_instance_id), from, to));
                        }
                    }
                }
            }
        }
        if items.len() > 32 {
            return false;
        }
        if items.is_empty() {
            return true;
        }
        let (Some(fence), Some(catalogue)) = (fence, self.quest_catalogue.as_ref()) else {
            return false;
        };
        if !matches!(
            self.root
                .check_character_world_callback(
                    self.character,
                    self.holder,
                    fence,
                    &callback.key,
                    std::sync::Arc::clone(catalogue),
                    permit,
                    context
                )
                .await,
            Ok(true)
        ) {
            #[cfg(test)]
            eprintln!("QUEST_SCENE_ITEMS_REFUSED durable_gates");
            return false;
        }
        // Native scripts move items sequentially. Each consequence has a stable parent
        // ingress identity and its existing durable receipt; retries cannot duplicate MINT.
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
                _ => return false,
            }
            let digest = hash.finalize();
            let mut bytes = [0; 8];
            bytes.copy_from_slice(&digest[..8]);
            let consequence = UseCommand {
                command_id: u64::from_be_bytes(bytes).max(1),
                ..command
            };
            let outcome =
                loop_move::relocate_source_item(self, permit, actor, consequence, item, from, to)
                    .await;
            if outcome != LoopMoveResult::Committed {
                #[cfg(test)]
                eprintln!("QUEST_SCENE_ITEMS_REFUSED item={outcome:?}");
                return false;
            }
        }
        true
    }
}
