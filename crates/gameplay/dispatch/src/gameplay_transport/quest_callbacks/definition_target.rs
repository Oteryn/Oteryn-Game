//! Ground handles share the World callback's selected-tool transaction.
use super::*;
use crate::durability::item_use::ItemUseInventory;
use crate::foundation::{
    SourceActorReservation, SourceActorReservationProof, source_reservation_seal,
};
use oteryn_protocol_oteryn::world_object::{MapUseTarget, UseIntent};

struct GroundUseProof {
    actor: ExactActorRef,
    command: CommandRef,
    fence: CurrentCharacterGameplayFence,
    at: MovementPositionSnapshot,
    binding: [u8; 32],
    affected: Vec<ExactActorRef>,
}

impl source_reservation_seal::Sealed for GroundUseProof {}
impl SourceActorReservationProof for GroundUseProof {
    fn owner(&self) -> (ExactActorRef, GameSessionId) {
        (self.actor, self.command.game_session_id())
    }

    fn event_identity(&self) -> [u8; 16] {
        let mut id = [0; 16];
        id[8..].copy_from_slice(&self.command.command_id().get().to_be_bytes());
        id
    }

    fn source_binding_digest(&self) -> [u8; 32] {
        self.binding
    }

    fn affected_actors(&self) -> &[ExactActorRef] {
        &self.affected
    }

    fn current_for(&self, runtime: &ChannelRuntimeV1) -> bool {
        runtime.binding().runtime_scope() == self.fence.runtime_scope
            && runtime.binding().scope_generation() == self.fence.scope_ownership_generation
            && runtime
                .bound_attacker_lease(self.actor, self.command.game_session_id())
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == self.fence.character_id.as_bytes()
                        && lease.generation() == self.fence.character_lease_generation
                })
            && runtime
                .read_actor_position(self.actor)
                .is_ok_and(|at| at == self.at)
    }
}

pub(super) struct GroundUseTurn(GroundUseProof, SourceActorReservation);

impl GroundUseTurn {
    pub(super) fn release(self, runtime: &mut ChannelRuntimeV1) {
        if runtime
            .release_source_actors_after_observed_outcome(self.1, &self.0)
            .is_err()
        {
            runtime.retire_owner_cycle();
        }
    }
}

#[derive(Clone)]
pub(super) struct PendingGroundUse {
    character: crate::domain::CharacterId,
    request: QuestTransitionRequest,
    intent: Vec<u8>,
}

pub(super) struct GroundUseGuard<'a, 'r> {
    owner:
        &'a super::super::activity_runtime::SharedOwner<'r, tokio::sync::Mutex<ChannelRuntimeV1>>,
    cancelled: &'a std::sync::Mutex<Vec<GroundUseTurn>>,
    turn: Option<GroundUseTurn>,
}

impl GroundUseGuard<'_, '_> {
    pub(super) fn affected(&self) -> Option<&[ExactActorRef]> {
        Some(&self.turn.as_ref()?.0.affected)
    }
}

#[derive(Clone)]
pub(super) struct PendingTomb {
    pub(super) actors: Vec<ExactActorRef>,
    turn: std::sync::Arc<std::sync::Mutex<Option<GroundUseTurn>>>,
}

impl PendingTomb {
    pub(super) fn new(actors: &[ExactActorRef]) -> Self {
        Self {
            actors: actors.to_vec(),
            turn: Default::default(),
        }
    }

    // Once the write can be sent, the receipt owner retains the installed
    // reservation even if the caller is cancelled or the commit ACK is lost.
    pub(super) fn retain(&self, guard: &mut GroundUseGuard<'_, '_>) -> bool {
        let mut pending = self
            .turn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        if pending.is_some() {
            return false;
        }
        *pending = guard.turn.take();
        pending.is_some()
    }

    pub(super) fn release(&self, runtime: &mut ChannelRuntimeV1) {
        let turn = self
            .turn
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .take();
        if let Some(turn) = turn {
            turn.release(runtime);
        }
    }
}

impl Drop for GroundUseGuard<'_, '_> {
    fn drop(&mut self) {
        if let Some(turn) = self.turn.take() {
            if let Ok(mut runtime) = self.owner.try_lock() {
                turn.release(&mut runtime);
            } else if let Ok(mut pending) = self.cancelled.lock() {
                pending.push(turn);
            }
        }
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) fn reserve_tomb_entry(
        &self,
        runtime: &mut ChannelRuntimeV1,
        actor: ExactActorRef,
        command: CommandRef,
        fence: CurrentCharacterGameplayFence,
        callback: &Callback,
    ) -> Option<GroundUseGuard<'_, '_>> {
        use sha2::{Digest, Sha256};
        let encounter = callback.effects.encounter.as_ref()?;
        let census = runtime.positioned_melee_census().ok()?;
        let mut affected = vec![encounter::koshei_defender(runtime, encounter, &census).ok()?];
        for member in &encounter.players {
            let who = census
                .iter()
                .find(|(_, at, session)| session.is_some() && at.position() == local(member.from))?
                .0;
            if who != actor {
                affected.push(who);
            }
        }
        let proof = GroundUseProof {
            actor,
            command,
            fence,
            at: runtime.read_actor_position(actor).ok()?,
            binding: Sha256::digest(callback.key.as_bytes()).into(),
            affected,
        };
        let mut reservation = runtime.prepare_source_actor_reservation(&proof).ok()?;
        runtime
            .reserve_source_actors(&mut reservation, &proof)
            .ok()?;
        Some(GroundUseGuard {
            owner: &self.runtime,
            cancelled: &self.states.quest_callbacks.ground_use_cleanup,
            turn: Some(GroundUseTurn(proof, reservation)),
        })
    }

    pub(super) async fn reconcile_ground_use_receipt(&self, session: GameSessionId) -> bool {
        let pending = match self.states.quest_callbacks.ground_uses.lock() {
            Ok(uses) => uses.get(&session).cloned(),
            Err(_) => return false,
        };
        let Some(pending) = pending else {
            return true;
        };
        let QuestCause::Use(command) = pending.request.cause else {
            return false;
        };
        if !self.states.command_ingress.original_terminalized(command) {
            if !self.states.command_ingress.pending_reconciliation(command) {
                return false;
            }
            let Some(_permit) = self.spell_lane_permit().await else {
                return false;
            };
            let _revision = self.revision_sequencer.acquire(pending.character).await;
            let disposition = match self
                .root
                .reconcile_quest_ground_use(
                    self.character,
                    pending.character,
                    pending.request,
                    pending.intent,
                )
                .await
            {
                Ok(Some(true)) => UseDisposition::Committed,
                Ok(None) => UseDisposition::Rejected,
                Ok(Some(false)) | Err(_) => return false,
            };
            let mut runtime = self.runtime.lock().await;
            self.states
                .quest_callbacks
                .finish_cancelled_ground_uses(&mut runtime);
            drop(runtime);
            if self
                .states
                .command_ingress
                .complete_quest_callback(command, disposition)
                .is_none()
            {
                return false;
            }
        }
        self.states
            .quest_callbacks
            .ground_uses
            .lock()
            .is_ok_and(|mut uses| {
                uses.remove(&session);
                true
            })
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "reuses the carried quest receipt probe and inventory read"
    )]
    pub(in crate::gameplay_transport) async fn use_ground_definition_callback(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        intent: &UseIntent,
        source: Option<[u8; 16]>,
        target: Option<[u8; 16]>,
        inventory: &ItemUseInventory,
        encoded: &[u8],
    ) -> Result<Option<UseOutcome>, ()> {
        let (Some(fence), Some(target), Some(MapUseTarget::Item(_))) =
            (command.item_fence, target, &intent.use_with_map_item)
        else {
            return Ok(None);
        };
        let backpack = || {
            inventory
                .backpack
                .iter()
                .flat_map(|bag| &bag.entries)
                .chain(inventory.nested.iter().map(|(_, entry)| entry))
                .map(|entry| &entry.item)
        };
        let entries = || inventory.equipment.iter().chain(backpack());
        if entries().any(|entry| entry.item_instance_id == target) {
            return Ok(None);
        }
        let item = if let Some(source) = source {
            entries().find(|entry| entry.item_instance_id == source)
        } else {
            let index = self
                .active_generation
                .and_then(|active| active.native_gameplay())
                .and_then(|native| native.item_index());
            entries().find(|entry| {
                super::super::mount_taming::quest_objects::matches_source(
                    &intent.target,
                    None,
                    entry.item_instance_id,
                    index.and_then(|index| {
                        index.definition_ref(
                            &entry.definition.family,
                            &entry.definition.production_key,
                            &entry.definition.revision_ref,
                        )
                    }),
                )
            })
        };
        let Some(item) = item else {
            return Ok(None);
        };
        let source = item.item_instance_id;
        let callbacks = self.callback_content();
        let accepts = |callback: &&Callback| {
            callback.item_on_target.as_ref().is_some_and(|tool| {
                tool.item == item.definition.production_key
                    || tool.alternatives.contains(&item.definition.production_key)
            })
        };
        let owns_tool = callbacks
            .iter()
            .filter(|c| c.selector.target_definition.is_some())
            .any(|c| accepts(&c));
        let ground = self
            .root
            .read_world_callback_ground_target(fence.runtime_scope, target)
            .await;
        let (definition, ground_map, spatial_position) = match ground {
            Ok(Some(ground)) => ground,
            Ok(None) => return Ok(owns_tool.then(UseOutcome::rejected)),
            Err(_) if !owns_tool => return Ok(None),
            Err(_) => return Err(()),
        };
        let mut candidates = callbacks
            .iter()
            .filter(|callback| {
                callback.selector.target_definition.as_deref() == Some(definition.as_str())
            })
            .peekable();
        let has_target = candidates.peek().is_some();
        if !has_target && !owns_tool {
            return Ok(None);
        }
        let outcome = |disposition| UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        };
        let mut matches = candidates.filter(accepts);
        let callback = matches.next().filter(|_| matches.next().is_none());
        use oteryn_gameplay_quests::quest::koshei_amulet::{KEY, TOMB_ENTERED};
        if callback.is_some_and(|callback| callback.key == KEY) {
            // The authored tomb's seal must prove its encounter kill and
            // sample the corpse. Leave that use to the carried quest owner.
            let progress = self
                .root
                .read_character_quest_state(self.character, fence.character_id)
                .await
                .map_err(|_| ())?;
            if progress.tracks().get(TOMB_ENTERED) == Some(&1) {
                return Ok(None);
            }
        }
        let command_ref = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        let Some(map) = self.bundle_map() else {
            return Ok(Some(UseOutcome::rejected()));
        };
        use sha2::{Digest, Sha256};
        let map_revision = crate::map::overlay::map_revision(map.overlay.base());
        let native_revision = format!("sha256:{}", hex(&Sha256::digest(map_revision.as_bytes())));
        let cell = crate::durability::spell_items_abi::decode_ground_cell(&spatial_position)
            .map_err(|_| ())?;
        let floor = if ground_map == native_revision {
            cell.z
        } else if ground_map == map_revision {
            -cell.z
        } else {
            return Ok(Some(UseOutcome::rejected()));
        };
        let target_position = crate::foundation::MovementLocalPosition {
            x: cell.x,
            y: cell.y,
            floor: i16::try_from(floor).map_err(|_| ())?,
        };
        let Some(catalogue) = self.quest_catalogue.as_ref() else {
            return Ok(Some(UseOutcome::rejected()));
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return Ok(Some(UseOutcome::rejected()));
        };
        let Some(current) = self
            .current_quest_fence(command.game_session_id)
            .await
            .map_err(|_| ())?
        else {
            return Ok(Some(UseOutcome::rejected()));
        };
        if current.item_fence() != fence {
            return Ok(Some(UseOutcome::rejected()));
        }
        let mut revision = self.revision_sequencer.acquire(current.character_id).await;
        let (callback, context, turn) = {
            let (mut runtime, states) = self.lock_callback_guards().await;
            self.states
                .quest_callbacks
                .finish_cancelled_ground_uses(&mut runtime);
            if runtime.owner_fence().is_err()
                || !runtime
                    .player_control_facts(actor, command.game_session_id)
                    .is_ok_and(|facts| facts.control_loss.is_none())
            {
                return Ok(Some(UseOutcome::rejected()));
            }
            let at = runtime.read_actor_position(actor).map_err(|_| ())?;
            if states.is_dead(actor) || !states.spell_idle(&runtime, actor, command.game_session_id)
            {
                return Ok(Some(UseOutcome::rejected()));
            }
            let position = at.position();
            if callback.is_some_and(|callback| {
                position.floor != target_position.floor
                    || position.x.abs_diff(target_position.x) > u32::from(callback.use_range)
                    || position.y.abs_diff(target_position.y) > u32::from(callback.use_range)
            }) {
                return Ok(Some(outcome(UseDisposition::TooFar)));
            }
            if !self
                .states
                .world_gate
                .lock()
                .is_ok_and(|mut gate| gate.admit_use_with(actor, self.clock_origin.elapsed()))
            {
                return Ok(Some(outcome(UseDisposition::Exhausted)));
            }
            let Some(callback) = callback else {
                return Ok(Some(UseOutcome::rejected()));
            };
            if changes_carried_items(callback)
                && !self
                    .states
                    .player_trades
                    .physical_service(command.game_session_id)
            {
                return Ok(Some(UseOutcome::rejected()));
            }
            let proof = GroundUseProof {
                actor,
                command: command_ref,
                fence: current,
                at,
                binding: Sha256::digest(encoded).into(),
                affected: Vec::new(),
            };
            let context = QuestWorldContext {
                map_revision,
                content_revision: format!(
                    "sha256:{}",
                    hex(&runtime.content_pin().server_artifact_digest())
                ),
                placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                reset_epoch: self.reset_epoch,
                actor_placement: actor.placement_identity(),
                movement_revision: at.revision(),
                party: Vec::new(),
            }
            .with_selected_ground_tool(
                source,
                encoded.to_vec(),
                target,
                [position.x, position.y, i32::from(position.floor)],
                [
                    target_position.x,
                    target_position.y,
                    i32::from(target_position.floor),
                ],
            )
            .ok_or(())?;
            let mut reservation = runtime
                .prepare_source_actor_reservation(&proof)
                .map_err(|_| ())?;
            runtime
                .reserve_source_actors(&mut reservation, &proof)
                .map_err(|_| ())?;
            (callback, context, GroundUseTurn(proof, reservation))
        };
        let mut guard = GroundUseGuard {
            owner: &self.runtime,
            cancelled: &self.states.quest_callbacks.ground_use_cleanup,
            turn: Some(turn),
        };
        let request = QuestTransitionRequest {
            transition_key: callback.key.clone(),
            cause: QuestCause::Use(command_ref),
        };
        let retained = self.states.command_ingress.pending(command_ref);
        if retained {
            let mut ground_uses = self
                .states
                .quest_callbacks
                .ground_uses
                .lock()
                .map_err(|_| ())?;
            let std::collections::hash_map::Entry::Vacant(entry) =
                ground_uses.entry(command.game_session_id)
            else {
                return Err(());
            };
            entry.insert(PendingGroundUse {
                character: current.character_id,
                request: request.clone(),
                intent: encoded.to_vec(),
            });
        }
        let result = revision
            .commit_world_callback_with_items_selected(
                self.root,
                self.character,
                self.holder,
                current,
                request,
                std::sync::Arc::clone(catalogue),
                &permit,
                context,
                self.active_generation
                    .and_then(|generation| generation.native_gameplay()),
                None,
                &self.premium_coordinator,
            )
            .await;
        {
            let mut runtime = self.runtime.lock().await;
            if let Some(turn) = guard.turn.take() {
                turn.release(&mut runtime);
            }
        }
        drop(revision);
        drop(permit);
        let result = result.map_err(|_| ())?;
        let committed = matches!(
            result,
            QuestTransitionOutcome::Committed(_) | QuestTransitionOutcome::AlreadyCommitted(_)
        );
        if matches!(result, QuestTransitionOutcome::Committed(_))
            && let Some(effect) = callback.effects.target_magic
            && let Some(position) = native(target_position)
        {
            map.facts.emit_movement_effect(position, effect);
        }
        if matches!(result, QuestTransitionOutcome::Committed(_))
            && let Some(message) = &callback.success_message
        {
            self.states
                .chat
                .push_quest_feedback(command.game_session_id, message);
        }
        if committed {
            self.refresh_quest_session(command.game_session_id, false)
                .await;
        }
        #[cfg(any(test, feature = "test-support"))]
        if committed
            && self
                .states
                .quest_callbacks
                .interrupt_ground_use_after_commit
                .swap(false, std::sync::atomic::Ordering::AcqRel)
        {
            return Err(());
        }
        let disposition = if committed {
            UseDisposition::Committed
        } else {
            UseDisposition::Rejected
        };
        if retained {
            self.states
                .command_ingress
                .complete_item_use(command_ref, disposition)
                .ok_or(())?;
            self.states
                .quest_callbacks
                .ground_uses
                .lock()
                .map_err(|_| ())?
                .remove(&command.game_session_id);
        }
        Ok(Some(UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        }))
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn use_koshei_corpse(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        key: super::super::item_view::ItemKey,
    ) -> Option<UseOutcome> {
        use super::super::item_view::ItemKey;
        let find = |runtime: &ChannelRuntimeV1| match key {
            ItemKey::Instance(item) => runtime
                .visible_corpses()
                .into_iter()
                .find(|corpse| corpse.item.item_instance_id == item),
            ItemKey::Entity(identity) => runtime.bound_corpse(identity),
            _ => None,
        };
        let corpse = find(&*self.runtime.lock().await)?;
        if corpse.item.production_key != "oteryn:item.tibia.i7535" {
            return None;
        }
        let outcome = |disposition| {
            Some(UseOutcome {
                disposition,
                ..UseOutcome::rejected()
            })
        };
        let Some(fence) = command.item_fence else {
            return outcome(UseDisposition::Rejected);
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return outcome(UseDisposition::Rejected);
        };
        if !self
            .current_quest_fence(command.game_session_id)
            .await
            .is_ok_and(|current| current.is_some_and(|current| current.item_fence() == fence))
        {
            return outcome(UseDisposition::Rejected);
        }
        let observed = tokio::time::Instant::now();
        let Ok(Some(remaining)) = self
            .root
            .koshei_corpse_remaining(&permit, fence, corpse.item.item_instance_id)
            .await
        else {
            return outcome(UseDisposition::Rejected);
        };
        let Some(expires) = observed.checked_add(remaining) else {
            return outcome(UseDisposition::Rejected);
        };
        #[cfg(test)]
        {
            let probe = self
                .states
                .quest_callbacks
                .observed_corpse_expiry
                .lock()
                .ok()
                .and_then(|mut slot| slot.take());
            if let Some(probe) = probe {
                let _ = probe.observed.send(expires);
                let _ = probe.resume.await;
            }
        }
        let Ok((mut runtime, states)) =
            tokio::time::timeout_at(expires, self.lock_callback_guards()).await
        else {
            return outcome(UseDisposition::Rejected);
        };
        let now = self.clock_origin.elapsed();
        if !runtime
            .player_control_facts(actor, command.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
            || runtime.binding().scope_generation() != fence.scope_ownership_generation
            || find(&runtime).as_ref() != Some(&corpse)
            || states.is_dead(actor)
            || !states.spell_idle(&runtime, actor, command.game_session_id)
            || !runtime
                .bound_attacker_lease(actor, command.game_session_id)
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
            || runtime.binding().runtime_scope() != fence.runtime_scope
        {
            return outcome(UseDisposition::Rejected);
        }
        let Ok(snapshot) = runtime.read_actor_position(actor) else {
            return outcome(UseDisposition::Rejected);
        };
        let at = snapshot.position();
        if at.floor != corpse.position.floor
            || at.x.abs_diff(corpse.position.x) > 1
            || at.y.abs_diff(corpse.position.y) > 1
        {
            return outcome(UseDisposition::TooFar);
        }
        if !self
            .states
            .world_gate
            .lock()
            .is_ok_and(|mut gate| gate.admit(actor, now))
        {
            return outcome(UseDisposition::Exhausted);
        }
        outcome(
            if tokio::time::Instant::now() < expires
                && runtime
                    .wake_koshei_from_corpse(corpse.item.item_instance_id)
                    .unwrap_or(false)
            {
                UseDisposition::Committed
            } else {
                UseDisposition::Rejected
            },
        )
    }
}
