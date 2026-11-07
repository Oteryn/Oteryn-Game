//! Current named-House source preflight, durable CAS and committed cue publication.
use super::*;
use crate::durability::consume_transform::HouseActionContext;
use crate::durability::consume_transform::{ConsumeTransformRequest, ConsumeTransformResult};
use oteryn_gameplay_items::{
    furniture,
    novelty_items::{GroundEffect, NoveltyEffects},
};
#[path = "house_item_cue_journal.rs"]
pub(in crate::gameplay_transport) mod cues;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_house_interactive(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        context: HouseActionContext,
        request: ConsumeTransformRequest,
        native: &crate::content::native_gameplay::NativeGameplayState,
    ) -> Result<Option<UseOutcome>, ()> {
        let source = self
            .root
            .read_house_interactive_source(self.holder, context.clone(), request.source)
            .await
            .map_err(|_| ())?;
        let Some(item) = source else { return Ok(None) };
        if !furniture::requires_house_preflight(&item.definition.production_key) {
            return Ok(None);
        }
        if furniture::is_carpet(&item.definition.production_key) && !item.placed {
            return Ok(Some(UseOutcome {
                disposition: UseDisposition::NotPossible,
                ..UseOutcome::rejected()
            }));
        }
        let position = item.position;
        let at = crate::foundation::MovementLocalPosition {
            x: position[0],
            y: position[1],
            floor: i16::try_from(position[2]).map_err(|_| ())?,
        };
        let mut planned = if furniture::side_rule(&item.definition.production_key).is_some() {
            furniture::plan_interactive(
                native,
                &item.definition,
                *command.game_session_id.as_bytes(),
                command.command_id,
                request.source,
                GroundEffect {
                    x: position[0],
                    y: position[1],
                    floor: at.floor,
                },
                false,
            )
        } else {
            furniture::plan_house(native, &item.definition)
        }
        .ok_or(())?;
        let blocked_doors = self.door.lock().await.blocking_cells().clone();
        {
            let runtime = self.runtime.lock().await;
            super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)
                .ok_or(())?;
            if furniture::is_carpet(&item.definition.production_key)
                && (blocked_doors.contains(&crate::content::LogicalCell {
                    x: position[0],
                    y: position[1],
                    z: position[2],
                }) || self.movement_cells.index().lookup(
                    self.movement_cells.scope(),
                    crate::content::LogicalCell {
                        x: position[0],
                        y: position[1],
                        z: position[2],
                    },
                ) != Ok(crate::content::CollisionClass::Walkable))
            {
                return Ok(Some(UseOutcome {
                    disposition: UseDisposition::NotPossible,
                    ..UseOutcome::rejected()
                }));
            }

            let fresh = HouseActionContext::from_native_owner(
                &runtime,
                actor,
                command.game_session_id,
                command.item_fence.ok_or(())?,
            )
            .map_err(|_| ())?;
            if fresh.house() != context.house() || fresh.position() != context.position() {
                return Ok(Some(UseOutcome {
                    disposition: UseDisposition::StaleState,
                    ..UseOutcome::rejected()
                }));
            }
            if item.definition.production_key == "oteryn:item.tibia.i26077"
                && (context.position() == position
                    || runtime
                        .house_cell_occupied_except(
                            actor,
                            command.game_session_id,
                            crate::foundation::LocalPosition {
                                x: position[0],
                                y: position[1],
                                floor: at.floor,
                            },
                        )
                        .map_err(|_| ())?)
            {
                return Ok(Some(UseOutcome {
                    disposition: UseDisposition::NotPossible,
                    ..UseOutcome::rejected()
                }));
            }
        }
        if furniture::speech(&item.definition.production_key).is_some() {
            planned.effects["ground"] = serde_json::to_value(GroundEffect {
                x: position[0],
                y: position[1],
                floor: at.floor,
            })
            .map_err(|_| ())?;
        }
        let effects = serde_json::from_value::<NoveltyEffects>(planned.effects.clone()).ok();
        let reservation = if let Some(effects) = &effects {
            Some(
                self.prepare_house_item_cues(actor, command, at, effects)
                    .await?,
            )
        } else {
            None
        };
        let [(after, 1)] = planned.outputs.as_slice() else {
            return Err(());
        };
        planned.effects["house_after"] =
            serde_json::json!({"key":after.production_key,"revision":after.revision_ref});
        let golden_display = furniture::is_golden_display(&item.definition.production_key);
        let result = self
            .root
            .consume_house_interactive(
                self.holder,
                context,
                request.clone(),
                native,
                item.definition,
                item.state_revision,
            )
            .await
            .map_err(|_| ())?;
        if golden_display
            && result.disposition == UseDisposition::NotPossible
            && result.effects == serde_json::json!({"golden_display_locked":true})
        {
            // This is a source-qualified read-only refusal, never a synthetic
            // economic commit. The command ingress owns immutable replay.
            drop(reservation);
            let mut refusal = effects.ok_or(())?;
            refusal.cues = vec![oteryn_gameplay_items::novelty_items::NoveltyCue {
                binding: "appearance:effect/donor_3".into(),
                effect_id: 3,
            }];
            refusal.speech = Some("You need Full Golden Outfit to use it.".into());
            refusal.private_speech = true;
            let reserved = self
                .prepare_house_item_cues(actor, command, at, &refusal)
                .await?;
            self.publish_house_item_cues(actor, command, reserved, &refusal)
                .await?;
            return Ok(Some(UseOutcome {
                disposition: result.disposition,
                ..UseOutcome::rejected()
            }));
        }
        if result.disposition == UseDisposition::Committed && result.fresh {
            // The Root owns the locked temporal proposal. Compare the exact
            // output here; finish below separately matches every typed effect.
            if result.effects.get("house_after") != planned.effects.get("house_after") {
                return Err(());
            }
            if let Some(reservation) = reservation {
                self.finish_house_item_cues(actor, command, reservation, &result)
                    .await?;
            }
        }
        Ok(Some(UseOutcome {
            disposition: result.disposition,
            ..UseOutcome::rejected()
        }))
    }
}

pub(in crate::gameplay_transport) struct HouseCueReservation {
    reservation: cues::Reservation,
    effects: NoveltyEffects,
    scope: crate::foundation::RuntimeScopeRefV1,
    generation: crate::foundation::ScopeOwnershipGeneration,
    binding: ([u8; 32], [u8; 32], u64),
}
impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn prepare_house_item_cues(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        position: crate::foundation::MovementLocalPosition,
        effects: &NoveltyEffects,
    ) -> Result<HouseCueReservation, ()> {
        if effects.require_player_target {
            return Err(());
        }
        let map = self.bundle_map().ok_or(())?;
        let runtime = self.runtime.lock().await;
        if !runtime
            .house_player_control_facts(actor, command.game_session_id)
            .is_ok_and(|facts| facts.control_loss.is_none())
        {
            return Err(());
        }
        let context = HouseActionContext::from_native_owner(
            &runtime,
            actor,
            command.game_session_id,
            command.item_fence.ok_or(())?,
        )
        .map_err(|_| ())?;
        super::super::item_ref_admission::pinned_index(&runtime, self.active_generation)
            .ok_or(())?;
        let binding = (
            runtime.content_pin().client_artifact_digest(),
            map.overlay.base().digest(),
            self.reset_epoch,
        );
        let fence = command.item_fence.ok_or(())?;
        let scope = context.house().runtime_scope().map_err(|_| ())?;
        let reservation = cues::Journal::reserve(
            &self.states.house_item_effects,
            scope,
            fence.scope_ownership_generation,
            actor,
            position,
            effects.cues.iter().map(|cue| cue.effect_id).collect(),
            binding,
        )
        .ok_or(())?;
        Ok(HouseCueReservation {
            reservation,
            effects: effects.clone(),
            scope,
            generation: fence.scope_ownership_generation,
            binding,
        })
    }
    pub(in crate::gameplay_transport) async fn finish_house_item_cues(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        prepared: HouseCueReservation,
        receipt: &ConsumeTransformResult,
    ) -> Result<(), ()> {
        if receipt.disposition != UseDisposition::Committed || !receipt.fresh {
            return Ok(());
        }
        let actual =
            serde_json::from_value::<NoveltyEffects>(receipt.effects.clone()).map_err(|_| ())?;
        if actual != prepared.effects {
            return Err(());
        }
        self.publish_house_item_cues(actor, command, prepared, &actual)
            .await
    }
    async fn publish_house_item_cues(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        prepared: HouseCueReservation,
        actual: &NoveltyEffects,
    ) -> Result<(), ()> {
        let map = self.bundle_map().ok_or(())?;
        let runtime = self.runtime.lock().await;
        let Ok(current) = HouseActionContext::from_native_owner(
            &runtime,
            actor,
            command.game_session_id,
            command.item_fence.ok_or(())?,
        ) else {
            return Ok(());
        };
        let binding = (
            runtime.content_pin().client_artifact_digest(),
            map.overlay.base().digest(),
            self.reset_epoch,
        );
        if current.house().runtime_scope().map_err(|_| ())? != prepared.scope
            || command.item_fence.ok_or(())?.scope_ownership_generation != prepared.generation
            || binding != prepared.binding
        {
            return Ok(());
        }
        prepared.reservation.commit();
        self.states.chat.publish_house_item_speech(
            &runtime,
            actor,
            command.game_session_id,
            actual,
        );
        Ok(())
    }
}
