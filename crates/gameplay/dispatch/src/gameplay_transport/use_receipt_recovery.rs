//! Reconcile a disconnected USE through its original ledger entry before opening a stream.
use super::connection::{AdmittedSession, UseCommand};
use super::item_ref_admission::NativeItemWeights;
use super::session_ingress::{TerrainQuestController, UseReceipt};
use super::{AdmissionRetryBudget, ComposedFreshAdmission};
use crate::durability::item_craft::{ItemCraftRequest, MapHarvestRequest};
use crate::foundation::{CommandId, CommandRef};
use oteryn_protocol_oteryn::world_object::{
    UseDisposition, decode_use_intent_target, encode_item_use_intent,
};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn finish_recovered_terrain_use(&self, admitted: &AdmittedSession) {
        let (Some(actor), Some(fence)) = (admitted.runtime_actor, admitted.item_fence) else {
            return;
        };
        let session = admitted.game_session_id;
        let ingress = &self.states.command_ingress;
        if ingress.recovered_terrain_quest(session).is_none() {
            return;
        }
        // Receipt recovery runs while control is lost. Keep the restored controller's
        // fence for the owner cadence if the immediate retry window expires.
        let controller = TerrainQuestController { actor, fence };
        let applied = AdmissionRetryBudget::for_command(self.root)
            .retry_until(async || {
                if self.retry_recovered_terrain_use(controller).await {
                    return true;
                }
                self.refresh_quest_session(session, false).await;
                false
            })
            .await;
        if !applied {
            ingress.set_terrain_quest_controller(controller);
        }
    }

    pub(super) async fn retry_recovered_terrain_use(
        &self,
        controller: TerrainQuestController,
    ) -> bool {
        let session = controller.fence.game_session_id;
        let ingress = &self.states.command_ingress;
        while let Some(ordinal) = ingress.recovered_terrain_quest(session) {
            let command = UseCommand {
                game_session_id: session,
                command_id: ordinal.get(),
                item_fence: Some(controller.fence),
            };
            if !super::quest_terrain_uses::after_tool(self, controller.actor, command).await {
                return false;
            }
            ingress.clear_recovered_terrain_quest(CommandRef::new(session, ordinal));
        }
        true
    }

    pub(super) fn mark_use_receipt(&self, command: UseCommand, receipt: Option<UseReceipt>) {
        if let Ok(id) = CommandId::new(command.command_id) {
            self.states
                .command_ingress
                .set_use_receipt(CommandRef::new(command.game_session_id, id), receipt);
        }
    }

    /// The lost entry carries the original scope. Receipt probes do not need
    /// a live item handle and cannot start a new write or grant control.
    pub(super) async fn recover_pending_use_receipt(&self, lost: &AdmittedSession) -> Option<()> {
        let ingress = &self.states.command_ingress;
        let Some((reference, wire, receipt)) = ingress.pending_use_receipt(lost.game_session_id)?
        else {
            return Some(());
        };
        let actor = lost.runtime_actor?;
        let fence = lost.item_fence?;
        let permit = self.spell_lane_permit().await?;
        if !ingress.pending_reconciliation(reference) {
            return Some(());
        }
        let intent =
            decode_use_intent_target(lost.continuity.selected_capabilities.as_slice(), &wire)
                .ok()?;
        let encoded = encode_item_use_intent(&intent).ok()?;
        let disposition = match receipt {
            UseReceipt::Drome => {
                self.recover_drome_use(actor, fence, reference, &encoded)
                    .await?
            }
            UseReceipt::Craft => {
                let native = self.active_generation?.native_gameplay()?;
                let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
                self.root
                    .craft_item(
                        &permit,
                        self.character,
                        self.holder,
                        fence,
                        ItemCraftRequest {
                            command: reference,
                            source: [0; 16],
                            tool: None,
                            ingredient: None,
                            station: None,
                            gathering: None,
                            enchanting: None,
                            intent: encoded,
                        },
                        native,
                        &weights,
                        true,
                    )
                    .await
                    .ok()?
                    .unwrap_or(UseDisposition::Rejected)
            }
            UseReceipt::Harvest => {
                let native = self.active_generation?.native_gameplay()?;
                self.root
                    .harvest_map_item(
                        &permit,
                        self.character,
                        self.holder,
                        fence,
                        MapHarvestRequest {
                            command: reference,
                            intent: encoded,
                            operand: None,
                        },
                        native,
                        &NativeItemWeights(native),
                    )
                    .await
                    .ok()?
                    .map_or(UseDisposition::Rejected, |result| result.disposition)
            }
            UseReceipt::Terrain => self
                .root
                .apply_terrain_tool(
                    &permit,
                    self.character,
                    self.holder,
                    fence,
                    reference,
                    encoded,
                    self.terrain_scope()?,
                    None,
                )
                .await
                .ok()?
                .unwrap_or(UseDisposition::Rejected),
            UseReceipt::PromotionScroll(key) => {
                let committed = AdmissionRetryBudget::for_command(self.root)
                    .retry_unavailable(async || {
                        self.root
                            .read_promotion_scroll_receipt(self.character, fence, reference, key)
                            .await
                            .map_err(|_| super::AdmissionRefusal::Unavailable)
                    })
                    .await
                    .ok()?;
                if committed {
                    UseDisposition::Committed
                } else {
                    UseDisposition::Rejected
                }
            }
            UseReceipt::Coin => {
                match self
                    .root
                    .read_coin_use_receipt(self.character, fence, reference)
                    .await
                    .ok()?
                {
                    Some(true) => UseDisposition::Committed,
                    Some(false) => UseDisposition::NotPossible,
                    None => UseDisposition::Rejected,
                }
            }
        };
        if matches!(receipt, UseReceipt::PromotionScroll(_)) {
            let _runtime = self.runtime.lock().await;
            self.states
                .spell_states
                .lock()
                .await
                .item_uses
                .settle_recovered_promotion_scroll(actor, reference, disposition);
        }
        ingress.complete_item_use(reference, disposition)?;
        if disposition == UseDisposition::Committed
            && matches!(
                receipt,
                UseReceipt::Terrain | UseReceipt::Craft | UseReceipt::Harvest
            )
        {
            if matches!(receipt, UseReceipt::Terrain) {
                self.states.level_refresh.mark([fence.character_id]);
            }
            if matches!(receipt, UseReceipt::Harvest) {
                self.states.loop_ground.invalidate();
            }
            drop(permit);
            self.refresh_terrain_tools().await;
        }
        Some(())
    }
}
