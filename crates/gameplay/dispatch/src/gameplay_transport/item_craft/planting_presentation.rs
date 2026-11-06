//! Crystal201189f movements/seeds.lua:3-17 and ice_flower_seeds.lua:3-17:
//! reserve the existing cap23 journal before SQL, publish the committed pot POFF once.
use super::*;
use crate::durability::consume_transform::{ConsumeTransformRequest, ConsumeTransformResult};
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::CommandRef;

#[derive(Debug, Clone)]
pub(super) struct PlantingCueHold {
    actor: ExactActorRef,
    fence: CurrentCharacterItemFence,
    command: CommandRef,
    intent: Vec<u8>,
    use_request: Option<ConsumeTransformRequest>,
    drop_request: Option<crate::durability::item_loop_move::LoopMoveRequest>,
}

impl SoundPublisher {
    pub(in crate::gameplay_transport) fn planting_use_request(
        &self,
        actor: ExactActorRef,
        request: &ConsumeTransformRequest,
    ) -> Option<ConsumeTransformRequest> {
        self.planting
            .iter()
            .find(|hold| {
                hold.actor == actor
                    && hold.command == request.command
                    && hold.intent == request.intent
            })
            .and_then(|hold| hold.use_request.clone())
    }

    /// True means this is the original retained attempt, including unknown COMMIT.
    #[allow(
        clippy::too_many_arguments,
        reason = "separate native and durable command proofs"
    )]
    pub(in crate::gameplay_transport) fn prepare_planting_cue(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        fence: CurrentCharacterItemFence,
        command: CommandRef,
        intent: Vec<u8>,
        use_request: Option<ConsumeTransformRequest>,
        drop_request: Option<crate::durability::item_loop_move::LoopMoveRequest>,
    ) -> Result<bool, ()> {
        let binding = context(runtime).map_err(|_| ())?;
        if self.binding.as_ref() != Some(&binding) {
            self.sounds.clear();
            self.ambient.clear();
            self.planting.clear();
            self.binding = Some(binding);
        }
        self.planting.retain(|hold| {
            runtime
                .player_control_facts(hold.actor, hold.command.game_session_id())
                .is_ok()
        });
        if let Some(hold) = self
            .planting
            .iter()
            .find(|hold| hold.actor == actor && hold.command == command)
        {
            return if hold.fence == fence && hold.intent == intent {
                Ok(true)
            } else {
                Err(())
            };
        }
        if fence.game_session_id != command.game_session_id()
            || runtime.binding().runtime_scope() != fence.runtime_scope
            || runtime.binding().scope_generation() != fence.scope_ownership_generation
            || !runtime
                .bound_attacker_lease(actor, command.game_session_id())
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == fence.character_id.as_bytes()
                        && lease.generation() == fence.character_lease_generation
                })
            || self.planting.len() >= oteryn_protocol_oteryn::presentation::MAX_PRESENTATION_EVENTS
            || self
                .revision
                .checked_add(self.planting.len() as u64 + 1)
                .is_none()
        {
            return Err(());
        }
        self.sounds
            .try_reserve(oteryn_protocol_oteryn::presentation::MAX_PRESENTATION_EVENTS)
            .map_err(|_| ())?;
        self.planting.try_reserve(1).map_err(|_| ())?;
        self.planting.push(PlantingCueHold {
            actor,
            fence,
            command,
            intent,
            use_request,
            drop_request,
        });
        Ok(false)
    }

    pub(in crate::gameplay_transport) fn finish_planting_cue(
        &mut self,
        runtime: &ChannelRuntimeV1,
        actor: ExactActorRef,
        command: CommandRef,
        receipt: Option<&ConsumeTransformResult>,
        emit_replay: bool,
    ) -> Result<(), ()> {
        let Some(index) = self
            .planting
            .iter()
            .position(|hold| hold.actor == actor && hold.command == command)
        else {
            return Ok(());
        };
        let Some(receipt) = receipt.filter(|receipt| {
            receipt.disposition == oteryn_protocol_oteryn::world_object::UseDisposition::Committed
        }) else {
            self.planting.remove(index);
            return Ok(());
        };
        if !emit_replay
            || !receipt
                .recipe
                .as_deref()
                .is_some_and(|recipe| recipe.starts_with("oteryn:planting."))
        {
            self.planting.remove(index);
            return Ok(());
        }
        let hold = &self.planting[index];
        if context(runtime).ok().as_ref() != self.binding.as_ref()
            || !runtime
                .bound_attacker_lease(actor, command.game_session_id())
                .is_ok_and(|lease| {
                    lease.character_id().as_bytes() == hold.fence.character_id.as_bytes()
                        && lease.generation() == hold.fence.character_lease_generation
                })
        {
            return Err(());
        }
        let cue = receipt.effects["cues"]
            .as_array()
            .and_then(|cues| cues.first());
        if let Some(cue) = cue {
            if cue["binding"] != "appearance:effect/poff" || cue["effect_id"] != 3 {
                return Err(());
            }
            let ground: oteryn_gameplay_items::novelty_items::GroundEffect =
                serde_json::from_value(receipt.effects["ground"].clone()).map_err(|_| ())?;
            self.revision += 1;
            if self.sounds.len() == oteryn_protocol_oteryn::presentation::MAX_PRESENTATION_EVENTS {
                self.sounds.pop_front();
            }
            self.sounds.push_back(SoundEntry {
                revision: self.revision,
                actor,
                position: MovementLocalPosition {
                    x: ground.x,
                    y: ground.y,
                    floor: ground.floor,
                },
                cue: 3,
                secondary: 0,
                creature: false,
                effect: true,
                private: false,
            });
        }
        self.planting.remove(index);
        Ok(())
    }
}

impl crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn reconcile_pending_planting_cues(
        &self,
        actor: ExactActorRef,
        session: GameSessionId,
    ) -> bool {
        let pending: Vec<_> = self
            .states
            .spell_states
            .lock()
            .await
            .sounds
            .planting
            .iter()
            .filter(|hold| hold.actor == actor && hold.command.game_session_id() == session)
            .cloned()
            .collect();
        if pending.is_empty() {
            return true;
        }
        let Some(native) = self
            .active_generation
            .and_then(|active| active.native_gameplay())
        else {
            return false;
        };
        let Some(permit) = self.spell_lane_permit().await else {
            return false;
        };
        let content = crate::gameplay_transport::item_ref_admission::NativeItemWeights(native)
            .with_wheel(self.premium_coordinator);
        for hold in pending {
            let receipt = if let Some(request) = hold.use_request {
                self.root
                    .consume_transform_detailed(
                        &permit,
                        self.character,
                        self.holder,
                        hold.fence,
                        request,
                        native,
                        &content,
                        true,
                    )
                    .await
            } else if let Some(request) = hold.drop_request {
                self.root
                    .read_item_loop_planting_presentation(
                        &permit,
                        self.character,
                        self.holder,
                        hold.fence,
                        hold.command,
                        &request,
                        native,
                        &content,
                    )
                    .await
            } else {
                return false;
            };
            let Ok(receipt) = receipt else { return false };
            if receipt.as_ref().is_some_and(|receipt| {
                receipt.disposition
                    == oteryn_protocol_oteryn::world_object::UseDisposition::Committed
            }) && !self.pending_item_owner_current(actor, hold.fence).await
            {
                return false;
            }
            let runtime = self.runtime.lock().await;
            if self
                .states
                .spell_states
                .lock()
                .await
                .sounds
                .finish_planting_cue(&runtime, actor, hold.command, receipt.as_ref(), true)
                .is_err()
            {
                return false;
            }
        }
        true
    }
}
