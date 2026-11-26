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
    pub(super) fn prepare_nomads_ritual(
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
            && super::quest_callbacks::world_requirements_match(map, callback)
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
        let Some(fence) = self.current_quest_fence(command.game_session_id).await? else {
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
        if gesture == NomadsRitualGesture::Drop {
            self.states.loop_ground.invalidate();
        }
        #[cfg(test)]
        if interrupted_after_commit(command) {
            return Err(());
        }
        let observed_at = self.owner_now().get();
        let receipt = self
            .read_nomads_receipt(fence.item_fence(), reference)
            .await?
            .ok_or(())?;
        self.publish_nomads_ritual(&receipt, observed_at)?;
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

    fn publish_nomads_ritual(
        &self,
        receipt: &NomadsRitualReceipt,
        observed_at: u64,
    ) -> Result<(), ()> {
        let Some(map) = self.bundle_map() else {
            return Ok(());
        };
        if receipt.remaining_micros == 0
            || receipt.target.reset_epoch != self.reset_epoch
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
        let due = observed_at
            .checked_add(receipt.remaining_micros)
            .ok_or(())?;
        if due <= self.owner_now().get() {
            return Ok(());
        }
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
            .ok_or(())
    }

    /// A rebuilt map derives both timed appearances from committed world state.
    pub(super) async fn restore_nomads_rituals(
        &self,
        permit: &SpellLanePermit,
        scope: crate::foundation::RuntimeScopeRefV1,
    ) -> Result<(), ()> {
        let Some(map) = self.bundle_map() else {
            return Ok(());
        };
        let Some((tile, key)) = target(NomadsRitualGesture::Drop) else {
            return Ok(());
        };
        if live_entry(map, tile, key).is_none() {
            return Ok(());
        }
        let target = NomadsRitualTarget {
            map_digest: map.overlay.base().digest(),
            reset_epoch: self.reset_epoch,
        };
        let observed_at = self.owner_now().get();
        let receipts = self
            .root
            .active_nomads_rituals(self.character, permit, scope, target)
            .await
            .map_err(|_| ())?;
        for receipt in receipts {
            self.publish_nomads_ritual(&receipt, observed_at)?;
        }
        Ok(())
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
        let observed_at = self.owner_now().get();
        let Some(receipt) = self.read_nomads_receipt(fence, reference).await? else {
            return Ok(false);
        };
        if receipt.gesture != gesture {
            return Err(());
        }
        let _permit = self.spell_lane_permit().await.ok_or(())?;
        self.publish_nomads_ritual(&receipt, observed_at)?;
        if gesture == NomadsRitualGesture::Pour {
            self.finish_nomads_fluid(reference, receipt.item).await;
        }
        Ok(true)
    }

    /// A committed item command can complete the original reservation before the
    /// reconnect gate. No new quest transition is run during recovery.
    pub(super) async fn recover_nomads_ingress(&self, lost: &AdmittedSession) -> bool {
        let ingress = &self.states.command_ingress;
        let (Some(fence), Some(id)) = (
            lost.item_fence,
            ingress.disconnected_original(lost.game_session_id),
        ) else {
            return true;
        };
        let reference = CommandRef::new(lost.game_session_id, id);
        let observed_at = self.owner_now().get();
        let Ok(receipt) = self.read_nomads_receipt(fence, reference).await else {
            return false;
        };
        let Some(receipt) = receipt else {
            return match super::AdmissionRetryBudget::for_command(self.root)
                .retry_command_guard(async || {
                    self.root
                        .read_nomads_ordinary_drop(self.character, fence, reference)
                        .await
                })
                .await
            {
                Ok(false) => true,
                Ok(true) => ingress.complete_nomads_drop(reference).is_some(),
                Err(_) => false,
            };
        };
        let Some(_permit) = self.spell_lane_permit().await else {
            return false;
        };
        let gesture = receipt.gesture;
        if self.publish_nomads_ritual(&receipt, observed_at).is_err() {
            return false;
        }
        self.schedule_quest_refresh(lost.game_session_id);
        match gesture {
            NomadsRitualGesture::Drop => ingress.complete_nomads_drop(reference),
            NomadsRitualGesture::Pour => {
                self.finish_nomads_fluid(reference, receipt.item).await;
                ingress.complete_item_use(reference, UseDisposition::Committed)
            }
        }
        .is_some()
    }
}

#[cfg(test)]
thread_local! {
    static INTERRUPT_AFTER_COMMIT: std::cell::Cell<Option<(crate::foundation::GameSessionId, u64)>> =
        const { std::cell::Cell::new(None) };
}

#[cfg(test)]
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) fn fixture_interrupt_nomads_ritual(
        &self,
        command: UseCommand,
    ) -> Result<(), &'static str> {
        INTERRUPT_AFTER_COMMIT.with(|fault| {
            if fault.get().is_some() {
                return Err("ritual fault already armed");
            }
            fault.set(Some((command.game_session_id, command.command_id)));
            Ok(())
        })
    }
}

#[cfg(test)]
pub(super) fn interrupted_after_commit(command: UseCommand) -> bool {
    INTERRUPT_AFTER_COMMIT.with(|fault| {
        if fault.get() == Some((command.game_session_id, command.command_id)) {
            fault.set(None);
            true
        } else {
            false
        }
    })
}
