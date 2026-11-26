//! Native item commands own the Nomads ritual and publish its committed timer.
use super::{
    ComposedFreshAdmission,
    connection::{AdmittedSession, UseCommand},
};
use crate::durability::{
    nomads_ritual::{NomadsPhysical, NomadsRitualGesture, NomadsRitualReceipt, NomadsRitualTarget},
    quest_state::QuestTransitionOutcome,
    quest_world_callback::QuestWorldContext,
    spell_owner_commit::SpellLanePermit,
};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use crate::map::{
    boot::BundleMap,
    facts::QuestCallbackProjection,
    overlay::TilePos,
    view::{self, EntryFacts, MapEntryKey, MapFacts},
};
use oteryn_gameplay_quests::quest::world_callbacks::{Callback, Position, nomads_ritual};
use oteryn_protocol_oteryn::world_object::UseDisposition;

#[derive(Clone, Copy)]
pub(super) enum PendingGesture {
    Drop,
    Pour([u8; 16]),
}

pub(super) fn near_nomads(at: crate::foundation::MovementLocalPosition) -> bool {
    let entrance = nomads_ritual::ENTRANCE;
    at.floor == -i16::from(entrance.floor)
        && at.x.abs_diff(i32::from(entrance.x)) <= 16
        && at.y.abs_diff(i32::from(entrance.y)) <= 16
}

impl PendingGesture {
    pub(super) fn remember(self, owner: &ComposedFreshAdmission<'_, '_, '_>, command: UseCommand) {
        if let Ok(id) = CommandId::new(command.command_id) {
            owner
                .states
                .command_ingress
                .remember_nomads(CommandRef::new(command.game_session_id, id), self);
        }
    }
}

fn position(gesture: NomadsRitualGesture) -> Position {
    match gesture {
        NomadsRitualGesture::Drop => nomads_ritual::CRACK,
        NomadsRitualGesture::Pour => nomads_ritual::ENTRANCE,
    }
}

fn target(gesture: NomadsRitualGesture) -> Option<(TilePos, u64)> {
    let Position { x, y, floor } = position(gesture);
    Some((
        TilePos { x, y, floor },
        oteryn_world_bundle::bundle::placement_key(floor, x, y, 1)?,
    ))
}

fn live_entry(map: &BundleMap, tile: TilePos, key: u64) -> Option<EntryFacts> {
    view::compose_tile(&map.overlay, &map.facts.pinned(), tile)
        .ok()??
        .entries
        .into_iter()
        .find(|entry| entry.source.key() == MapEntryKey::Base(key))
        .map(|entry| entry.facts)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    fn prepare_nomads_ritual(
        &self,
        map: &BundleMap,
        gesture: NomadsRitualGesture,
    ) -> Option<&Callback> {
        let (tile, key) = target(gesture)?;
        let callback = self
            .callback_content()
            .iter()
            .find(|row| row.key == gesture.callback())?;
        (super::quest_callbacks::matches(map, callback, key)
            && callback.selector.appearance == Some(live_entry(map, tile, key)?.appearance_id)
            && map
                .facts
                .quest_callback_capacity(key, map.facts.object_revision(key), 0)
            && map
                .facts
                .quest_callback_replacement(key, callback.effects.transform?)
                .is_some())
        .then_some(callback)
    }

    /// The lane covers target admission, the joint item/Quest write, and publication.
    pub(super) async fn commit_nomads_ritual(
        &self,
        permit: &SpellLanePermit,
        actor: ExactActorRef,
        command: UseCommand,
        physical: NomadsPhysical<'_>,
    ) -> Result<Option<bool>, ()> {
        let map = self.bundle_map().ok_or(())?;
        let gesture = physical.gesture();
        if self.prepare_nomads_ritual(map, gesture).is_none() {
            // USE already selected the ritual before taking the lane. If the
            // shared state changed meanwhile, leave its flask untouched.
            return Ok((gesture == NomadsRitualGesture::Pour).then_some(false));
        }
        let Some(fence) = super::AdmissionRetryBudget::for_command(self.root)
            .retry_command_guard(async || {
                self.current_quest_fence_durable(command.game_session_id)
                    .await
            })
            .await
            .map_err(|_| ())?
        else {
            return Ok(Some(false));
        };
        if command.item_fence != Some(fence.item_fence()) {
            return Ok(Some(false));
        }
        let catalogue = self.quest_catalogue.as_ref().ok_or(())?;
        let reference = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        let mut slot = self.revision_sequencer.acquire(fence.character_id).await;
        let context = {
            let runtime = self.runtime.lock().await;
            let expected = runtime.read_actor_position(actor).map_err(|_| ())?;
            // The source is the drop destination or the selected water tile.
            // Neither gesture derives reach from the transformed mountain.
            let source = match gesture {
                NomadsRitualGesture::Drop => nomads_ritual::CRACK,
                NomadsRitualGesture::Pour => nomads_ritual::WATER,
            };
            let Some(at) = super::quest_callbacks::native(expected.position()) else {
                return Ok(Some(false));
            };
            if at.floor != source.floor
                || at.x.abs_diff(source.x) > 1
                || at.y.abs_diff(source.y) > 1
            {
                return Ok(Some(false));
            }
            QuestWorldContext {
                map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                content_revision: format!(
                    "sha256:{}",
                    super::quest_callbacks::hex(&runtime.content_pin().server_artifact_digest())
                ),
                placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                reset_epoch: self.reset_epoch,
                actor_placement: actor.placement_identity(),
                movement_revision: expected.revision(),
                party: Vec::new(),
            }
        };
        // Acquiring a fence or the revision slot can wait across a timer tick.
        if self.prepare_nomads_ritual(map, gesture).is_none() {
            return Ok(Some(false));
        }
        if gesture == NomadsRitualGesture::Pour
            && !self
                .root
                .check_character_world_callback(
                    self.character,
                    self.holder,
                    fence,
                    gesture.callback(),
                    std::sync::Arc::clone(catalogue),
                    permit,
                    context.clone(),
                )
                .await
                .map_err(|_| ())?
        {
            // Arito's personal gate controls the ritual, not ordinary fluid use.
            return Ok(None);
        }
        let outcome = slot
            .commit_nomads_ritual(
                self.root,
                self.character,
                self.holder,
                fence,
                reference,
                std::sync::Arc::clone(catalogue),
                permit,
                context,
                &physical,
            )
            .await;
        match outcome {
            Ok(QuestTransitionOutcome::Committed(committed)) => {
                self.apply_committed_quest_copy(command.game_session_id, true, &committed);
            }
            Ok(QuestTransitionOutcome::AlreadyCommitted(committed)) => {
                self.apply_committed_quest_copy(command.game_session_id, false, &committed);
            }
            Ok(QuestTransitionOutcome::Refused(_) | QuestTransitionOutcome::ObligationClosed) => {
                return Ok(Some(false));
            }
            Err(error) => {
                #[cfg(test)]
                eprintln!("Nomads ritual transaction failed: {error:?}");
                let _ = error;
                return Err(());
            }
        }
        #[cfg(test)]
        if interrupted_after_commit(command) {
            return Err(());
        }
        if gesture == NomadsRitualGesture::Drop {
            self.states.loop_ground.invalidate();
        }
        let receipt = self
            .read_nomads_receipt(fence.item_fence(), reference)
            .await?
            .ok_or(())?;
        self.publish_nomads_ritual(&receipt)?;
        Ok(Some(true))
    }

    async fn read_nomads_receipt(
        &self,
        fence: crate::durability::item_transfer::CurrentCharacterItemFence,
        reference: CommandRef,
    ) -> Result<Option<NomadsRitualReceipt>, ()> {
        super::AdmissionRetryBudget::for_command(self.root)
            .retry_command_guard(async || {
                self.root
                    .nomads_ritual_gesture(self.character, fence, reference)
                    .await
            })
            .await
            .map_err(|_| ())
    }

    fn publish_nomads_ritual(&self, receipt: &NomadsRitualReceipt) -> Result<(), ()> {
        let Some(map) = self.bundle_map() else {
            return Ok(());
        };
        if receipt.target.reset_epoch != self.reset_epoch
            || receipt.target.map_digest != map.overlay.base().digest()
        {
            return Ok(());
        }
        let (tile, key) = target(receipt.gesture).ok_or(())?;
        let callback = self
            .callback_content()
            .iter()
            .find(|row| row.key == receipt.gesture.callback())
            .ok_or(())?;
        let restore = live_entry(map, tile, key).ok_or(())?;
        if Some(restore.appearance_id) != callback.selector.appearance {
            return Ok(());
        }
        let after = map
            .facts
            .quest_callback_replacement(key, callback.effects.transform.ok_or(())?)
            .ok_or(())?;
        let remaining = receipt
            .deadline
            .saturating_duration_since(std::time::Instant::now());
        if remaining.is_zero() {
            return Ok(());
        }
        let due = self
            .owner_now()
            .get()
            .checked_add(u64::try_from(remaining.as_micros()).map_err(|_| ())?)
            .ok_or(())?;
        let projection = QuestCallbackProjection::TimedTransform { due, restore };
        map.facts
            .apply_quest_callback(
                key,
                map.facts.object_revision(key),
                after,
                &[],
                &[],
                false,
                Some(&projection),
            )
            .then_some(())
            .ok_or(())?;
        if receipt.gesture == NomadsRitualGesture::Drop {
            self.clear_nomads_ground(receipt.item);
        }
        Ok(())
    }

    fn clear_nomads_ground(&self, item: [u8; 16]) {
        self.states.loop_ground.invalidate();
        self.states
            .browse_fields
            .committed_move(super::item_view::ItemKey::Instance(item), None);
    }

    fn has_nomads_placement(&self) -> bool {
        self.bundle_map().is_some_and(|map| {
            target(NomadsRitualGesture::Drop)
                .is_some_and(|(tile, key)| live_entry(map, tile, key).is_some())
        })
    }

    pub(super) async fn sync_nomads_map(&self) -> Result<(), ()> {
        if self.has_nomads_placement() {
            let permit = self.spell_lane_permit().await.ok_or(())?;
            self.restore_nomads_rituals(&permit).await?;
        }
        Ok(())
    }

    /// A rebuilt map derives both timed appearances from committed world state.
    pub(super) async fn restore_nomads_rituals(
        &self,
        permit: &SpellLanePermit,
    ) -> Result<bool, ()> {
        let Some(map) = self.bundle_map().filter(|_| self.has_nomads_placement()) else {
            return Ok(false);
        };
        let target = NomadsRitualTarget {
            map_digest: map.overlay.base().digest(),
            reset_epoch: self.reset_epoch,
        };
        let receipts = super::AdmissionRetryBudget::for_command(self.root)
            .retry_command_guard(async || {
                self.root
                    .active_nomads_rituals(self.character, permit, permit.runtime_scope(), target)
                    .await
            })
            .await
            .map_err(|_| ())?;
        map.facts.expire_movement_items(self.owner_now().get());
        let mut crack = false;
        let mut entrance = false;
        for receipt in receipts {
            self.publish_nomads_ritual(&receipt)?;
            if receipt.deadline > std::time::Instant::now() {
                match receipt.gesture {
                    NomadsRitualGesture::Drop => crack = true,
                    NomadsRitualGesture::Pour => entrance = true,
                }
            }
        }
        Ok(crack && !entrance)
    }

    /// Exact command replay is independent of current item handles and splash lifetime.
    pub(super) async fn resume_nomads_ritual(
        &self,
        command: UseCommand,
        gesture: NomadsRitualGesture,
    ) -> Result<bool, ()> {
        let fence = command.item_fence.ok_or(())?;
        let reference = CommandRef::new(
            command.game_session_id,
            CommandId::new(command.command_id).map_err(|_| ())?,
        );
        let Some(receipt) = self.read_nomads_receipt(fence, reference).await? else {
            return Ok(false);
        };
        if receipt.gesture != gesture {
            return Err(());
        }
        let _permit = self.spell_lane_permit().await.ok_or(())?;
        self.publish_nomads_ritual(&receipt)?;
        match gesture {
            NomadsRitualGesture::Drop => self.clear_nomads_ground(receipt.item),
            NomadsRitualGesture::Pour => {
                self.finish_nomads_fluid(reference, receipt.item, UseDisposition::Committed)
                    .await
            }
        }
        Ok(true)
    }

    /// A committed item command can complete the original reservation before the
    /// reconnect gate. No new quest transition is run during recovery.
    pub(super) async fn recover_nomads_ingress(&self, lost: &AdmittedSession) -> bool {
        let ingress = &self.states.command_ingress;
        let (Some(fence), Some((id, pending))) = (
            lost.item_fence,
            ingress.disconnected_nomads(lost.game_session_id),
        ) else {
            return true;
        };
        let reference = CommandRef::new(lost.game_session_id, id);
        let Some(_permit) = self.spell_lane_permit().await else {
            return false;
        };
        let Ok(receipt) = self.read_nomads_receipt(fence, reference).await else {
            return false;
        };
        if let Some(receipt) = &receipt {
            if self.publish_nomads_ritual(receipt).is_err() {
                return false;
            }
            self.schedule_quest_refresh(lost.game_session_id);
            if receipt.gesture == NomadsRitualGesture::Drop {
                self.clear_nomads_ground(receipt.item);
            }
        }
        let budget = super::AdmissionRetryBudget::for_command(self.root);
        match pending {
            PendingGesture::Drop => {
                let moved = if receipt.is_some() {
                    true
                } else {
                    use crate::durability::item_loop_move::LoopMoveReplay;
                    match budget
                        .retry_command_guard(async || {
                            self.root
                                .read_item_loop_move_receipt(
                                    self.character,
                                    fence.character_id,
                                    reference,
                                )
                                .await
                        })
                        .await
                    {
                        Ok(LoopMoveReplay::Moved) => true,
                        Ok(LoopMoveReplay::Absent) => false,
                        _ => return false,
                    }
                };
                ingress.complete_nomads_drop(reference, moved).is_some()
            }
            PendingGesture::Pour(item) => {
                let committed = if receipt.is_some() {
                    true
                } else {
                    let Ok(committed) = budget
                        .retry_command_guard(async || {
                            self.root
                                .read_nomads_fluid_receipt(self.character, fence, reference, item)
                                .await
                        })
                        .await
                    else {
                        return false;
                    };
                    committed
                };
                let disposition = if committed {
                    UseDisposition::Committed
                } else {
                    UseDisposition::Rejected
                };
                self.finish_nomads_fluid(reference, item, disposition).await;
                ingress.complete_item_use(reference, disposition).is_some()
            }
        }
    }
}

#[cfg(test)]
thread_local! {
    static INTERRUPT_RITUAL: std::cell::Cell<Option<(crate::foundation::GameSessionId, u64)>> =
        const { std::cell::Cell::new(None) };
}

#[cfg(test)]
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) fn fixture_interrupt_nomads_ritual(
        &self,
        command: UseCommand,
    ) -> Result<(), &'static str> {
        INTERRUPT_RITUAL.with(|fault| {
            if fault.get().is_some() {
                return Err("ritual fault already armed");
            }
            fault.set(Some((command.game_session_id, command.command_id)));
            Ok(())
        })
    }
}

#[cfg(test)]
fn interrupted_after_commit(command: UseCommand) -> bool {
    INTERRUPT_RITUAL.with(|fault| {
        if fault.get() == Some((command.game_session_id, command.command_id)) {
            fault.set(None);
            true
        } else {
            false
        }
    })
}
