//! Source scene custody changes reuse the map MINT and durable item-loop owners.
use super::*;
use crate::content::ReferenceItemField;
use crate::durability::item_loop_move::{
    LoopDestination, LoopGround, LoopMoveRequest, SourceSceneItemMove, SourceSceneReceipt,
};
use crate::durability::{DurabilityError, SqlxFailureClass};
use crate::gameplay_transport::{
    item_move::{LoopMove, LoopTarget},
    item_ref_admission::NativeItemWeights,
    item_view::ItemKey,
    loop_move,
};
use crate::map::{overlay::AddedItem, view::EntryFacts};
use oteryn_protocol_oteryn::item_view::WorldTilePosition;
use sha2::{Digest, Sha256};
use std::{collections::HashMap, sync::Mutex, time::Duration};

type SceneKey = ([u8; 32], u64, u64);
const SCENE_RESOLUTION_BUDGET: Duration = Duration::from_secs(2);

#[derive(Clone)]
struct SceneReceiptIntent {
    character: crate::domain::CharacterId,
    original_use: CommandRef,
    command: CommandRef,
    request: LoopMoveRequest,
}

#[derive(Default)]
pub(super) struct SceneItemReceipts(Mutex<HashMap<SceneKey, (SceneReceiptIntent, bool)>>);

struct SceneReceiptAttempt<'a> {
    receipts: &'a SceneItemReceipts,
    key: SceneKey,
    intent: SceneReceiptIntent,
    settled: bool,
}

impl Drop for SceneReceiptAttempt<'_> {
    fn drop(&mut self) {
        let mut records = self
            .receipts
            .0
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if self.settled {
            records.remove(&self.key);
        } else if let Some((_, busy)) = records.get_mut(&self.key) {
            *busy = false;
        }
    }
}

impl SceneItemReceipts {
    pub(super) fn has_session(&self, session: GameSessionId) -> bool {
        self.0.lock().map_or(true, |records| {
            records
                .values()
                .any(|(intent, _)| intent.original_use.game_session_id() == session)
        })
    }

    /// Reserve one writer or receipt reader for this placement. Cancellation
    /// leaves the exact batch receipt available for the next caller.
    fn claim(
        &self,
        key: SceneKey,
        new: Option<SceneReceiptIntent>,
    ) -> Result<Option<SceneReceiptAttempt<'_>>, ()> {
        use std::collections::hash_map::Entry;
        let mut records = self.0.lock().map_err(|_| ())?;
        let intent = match (records.entry(key), new) {
            (Entry::Vacant(entry), Some(intent)) => {
                entry.insert((intent.clone(), true));
                intent
            }
            (Entry::Occupied(mut entry), None) if !entry.get().1 => {
                let (intent, busy) = entry.get_mut();
                *busy = true;
                intent.clone()
            }
            (Entry::Vacant(_), None) => return Ok(None),
            _ => return Err(()),
        };
        Ok(Some(SceneReceiptAttempt {
            receipts: self,
            key,
            intent,
            settled: false,
        }))
    }
}

pub(super) struct ScenePublication<'a> {
    pub(super) map: crate::map::facts::PreparedSceneClose<'a>,
    pub(super) expected: MovementPositionSnapshot,
    pub(super) fresh_effects: bool,
    pub(super) success_message: Option<String>,
}

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

    fn scene_receipt_key(&self, map: &BundleMap, key: u64) -> SceneKey {
        (map.overlay.base().digest(), self.reset_epoch, key)
    }

    pub(super) fn scene_receipt_pending(&self, map: &BundleMap, key: u64) -> bool {
        self.states
            .quest_callbacks
            .scene_item_receipts
            .0
            .lock()
            .map_or(true, |records| {
                records.contains_key(&self.scene_receipt_key(map, key))
            })
    }

    pub(super) async fn reconcile_pending_scene_items(&self, map: &BundleMap, key: u64) -> bool {
        self.reconcile_parked_scene_items(self.scene_receipt_key(map, key))
            .await
    }

    pub(super) async fn reconcile_scene_receipts_for_session(
        &self,
        session: GameSessionId,
    ) -> bool {
        let keys = match self.states.quest_callbacks.scene_item_receipts.0.lock() {
            Ok(records) => records
                .iter()
                .filter(|(_, (intent, _))| intent.original_use.game_session_id() == session)
                .map(|(&key, _)| key)
                .collect::<Vec<_>>(),
            Err(_) => return false,
        };
        for key in keys {
            if !self.reconcile_parked_scene_items(key).await {
                return false;
            }
        }
        true
    }

    async fn reconcile_parked_scene_items(&self, key: SceneKey) -> bool {
        let mut attempt = match self
            .states
            .quest_callbacks
            .scene_item_receipts
            .claim(key, None)
        {
            Ok(Some(attempt)) => attempt,
            Ok(None) => return true,
            Err(()) => return false,
        };
        let budget = super::super::AdmissionRetryBudget::for_join_read(self.root, "scene_receipt");
        match budget
            .retry_durability("scene_receipt", || {
                self.read_scene_item_receipt(&attempt.intent)
            })
            .await
        {
            Ok(SourceSceneReceipt::Committed | SourceSceneReceipt::Absent) => {
                self.states.loop_ground.invalidate();
                // The original USE cannot report a closed scene: only custody
                // was in doubt, and its staged visual publication was released.
                let original = attempt.intent.original_use;
                let ingress = &self.states.command_ingress;
                if ingress.pending(original)
                    || (ingress.reconcilable(original)
                        && ingress
                            .complete_quest_callback(original, UseDisposition::Rejected)
                            .is_none())
                {
                    return false;
                }
                attempt.settled = true;
                true
            }
            _ => false,
        }
    }

    async fn read_scene_item_receipt(
        &self,
        intent: &SceneReceiptIntent,
    ) -> Result<SourceSceneReceipt, DurabilityError> {
        self.root
            .reconcile_source_scene_move(
                self.character,
                intent.character,
                intent.command,
                &intent.request,
            )
            .await
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "the scene uses the invoking actor and exact prepared map and custody inputs"
    )]
    pub(super) async fn commit_scene_items(
        &self,
        permit: SpellLanePermit,
        map: &BundleMap,
        callback: &Callback,
        key: u64,
        actor: ExactActorRef,
        original_use: CommandRef,
        optional_pvp: Option<bool>,
        prepared: PreparedSceneItems,
        publication: ScenePublication<'_>,
    ) -> UseDisposition {
        let Some(native) = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            return UseDisposition::Rejected;
        };
        let Some(last) = prepared.moves.last() else {
            return UseDisposition::Rejected;
        };
        let intent = SceneReceiptIntent {
            character: prepared.fence.character_id,
            original_use,
            command: last.command,
            request: last.request.clone(),
        };
        let Ok(Some(mut attempt)) = self
            .states
            .quest_callbacks
            .scene_item_receipts
            .claim(self.scene_receipt_key(map, key), Some(intent))
        else {
            return UseDisposition::Rejected;
        };
        let cache = loop_move::GroundChange(Some(&self.states.loop_ground));
        let result = self
            .root
            .commit_source_scene_item_moves(
                &permit,
                self.character,
                self.holder,
                prepared.fence,
                &NativeItemWeights(native).with_wheel(self.premium_coordinator),
                prepared.moves,
            )
            .await;
        drop(cache);
        let ground_epoch = self.states.loop_ground.epoch();
        let session = prepared.fence.game_session_id;
        let permit = match result {
            Ok(Ok(())) => permit,
            Err(DurabilityError::CommitOutcomeUnknown) => {
                drop(permit);
                // A busy holder or lost connection may recover within one more
                // bounded pass. The exact receipt stays reserved if it does not.
                let mut receipt = self.read_scene_item_receipt(&attempt.intent).await;
                if matches!(
                    receipt,
                    Err(DurabilityError::RootUnavailable
                        | DurabilityError::RootPassDeadlineExceeded
                        | DurabilityError::CommitOutcomeUnknown
                        | DurabilityError::Database(
                            SqlxFailureClass::Transport
                                | SqlxFailureClass::Pool
                                | SqlxFailureClass::RetryableStatement
                        ))
                ) {
                    let _ = tokio::time::timeout(SCENE_RESOLUTION_BUDGET, self.root.await_ready())
                        .await;
                    receipt = self.read_scene_item_receipt(&attempt.intent).await;
                }
                match receipt {
                    Ok(SourceSceneReceipt::Committed) => attempt.settled = true,
                    Ok(SourceSceneReceipt::Absent) => {
                        self.states.loop_ground.invalidate();
                        attempt.settled = true;
                        return UseDisposition::Rejected;
                    }
                    Ok(SourceSceneReceipt::RecoverySuperseded) => {
                        // The successor can read the retained receipt with its
                        // own seal. Never install this owner's staged scene.
                        self.states.loop_ground.invalidate();
                        return UseDisposition::Rejected;
                    }
                    Ok(SourceSceneReceipt::Conflict) | Err(_) => return UseDisposition::Rejected,
                }
                let resumed = tokio::time::timeout(SCENE_RESOLUTION_BUDGET, async {
                    let permit = self.spell_lane_permit().await?;
                    let current = self.current_quest_fence(session).await.ok().flatten()?;
                    (current.item_fence() == prepared.fence).then_some(permit)
                })
                .await;
                let Ok(Some(permit)) = resumed else {
                    self.states.loop_ground.invalidate();
                    return UseDisposition::Rejected;
                };
                // A later ground writer may have placed another item on an
                // open scene cell. A fresh USE will take a new ground census.
                let current_epoch = self.states.loop_ground.epoch();
                self.states.loop_ground.invalidate();
                if ground_epoch.is_none() || current_epoch != ground_epoch {
                    return UseDisposition::Rejected;
                }
                permit
            }
            _ => {
                attempt.settled = true;
                return UseDisposition::Rejected;
            }
        };
        attempt.settled = true;
        #[cfg(test)]
        if self
            .states
            .quest_callbacks
            .delay_scene_commit_ack
            .swap(false, std::sync::atomic::Ordering::AcqRel)
        {
            // Hold a committed acknowledgement past the ten-second USE deadline.
            tokio::time::sleep(std::time::Duration::from_secs(11)).await;
        }
        for (item, destination) in prepared.destinations {
            self.states
                .browse_fields
                .committed_move(item, Some(destination));
        }
        let Ok(mut runtime) =
            tokio::time::timeout(SCENE_RESOLUTION_BUDGET, self.runtime.lock()).await
        else {
            return UseDisposition::Rejected;
        };
        let Ok(mut states) = self.states.spell_states.try_lock() else {
            return UseDisposition::Rejected;
        };
        if permit
            .check_scope(runtime.binding().runtime_scope())
            .is_err()
            || runtime.binding().scope_generation() != prepared.fence.scope_ownership_generation
            || runtime.read_actor_position(actor) != Ok(publication.expected)
            || !runtime
                .player_control_facts(actor, session)
                .is_ok_and(|facts| facts.control_loss.is_none())
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, session)
        {
            return UseDisposition::Rejected;
        }
        let Ok(Some(proof)) =
            self.prepare_quest_scene_moves(&runtime, &states, map, callback, true)
        else {
            return UseDisposition::Rejected;
        };
        let observations = proof.observations();
        let Ok(positions) = runtime.commit_source_scene_relocations(proof, || {
            publication.map.publish();
            true
        }) else {
            return UseDisposition::Rejected;
        };
        self.publish_scene_positions(
            &mut runtime,
            &mut states,
            map,
            optional_pvp,
            observations.into_iter().zip(positions),
        );
        for effect in &callback.effects {
            map.facts
                .emit_movement_effect(tile(effect.position), effect.effect);
        }
        if publication.fresh_effects
            && let Some(speech) = &callback.effects.player_speech
        {
            self.states
                .chat
                .publish_quest_speech(&runtime, actor, session, speech);
        }
        if let Some(message) = publication.success_message {
            self.states.chat.push_quest_feedback(session, &message);
        }
        UseDisposition::Committed
    }
}
