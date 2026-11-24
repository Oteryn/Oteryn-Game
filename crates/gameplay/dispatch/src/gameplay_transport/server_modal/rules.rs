//! Server rules retain exact callback inputs; answers never supply gameplay authority.
use super::{BedUseError, ComposedFreshAdmission, ServerModalOutcome, actor_is_current, owner::Root};
use crate::durability::bed_sleep::{BedAccessEvidence, BedReservation, BedUseOutcome, BedUsePlacement};
use crate::durability::consume_transform::HouseActionContext;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use crate::foundation::{ChannelRuntimeV1, ExactActorRef, RuntimeScopeRefV1};
use crate::gameplay_transport::{
    AdmissionRetryBudget,
    connection::{AdmittedSession, UseCommand, UseOutcome},
    item_view::ItemKey,
    world_map::{
        DecoratedFacts, MapTargetRefusal, MapViewSource, resolve_map_handle, resolve_map_target,
    },
};
use crate::map::{overlay::TilePos, view::placement_of};
use oteryn_protocol_oteryn::{
    server_modal::{Answer, CAPABILITY_SERVER_MODAL_V1, Dialog, Reply},
    world_object::{CAPABILITY_WORLD_INTERACTION_V1, UseDisposition, UseTarget},
};

// Crystal 201189f items_definitions.hpp:99, RETURNVALUE_YOUNEEDPREMIUMACCOUNT.
const BED_PREMIUM_MESSAGE_ID: u8 = 54;

#[derive(Clone, PartialEq, Eq)]
pub(super) enum Selection {
    HirelingStore(super::store::Selection),
    Bed(BedSelection),
}

#[derive(Clone, PartialEq, Eq)]
pub(super) struct BedSelection {
    target: ItemKey,
    placement: BedUsePlacement,
    fence: CurrentCharacterItemFence,
    actor: ExactActorRef,
    access: BedAccessEvidence,
}

impl BedSelection {
    pub(super) fn key(&self) -> Option<(RuntimeScopeRefV1, TilePos)> {
        let (head, _) = placement_of(self.placement.head.placement_key)?;
        Some((self.fence.runtime_scope, head))
    }

    pub(super) fn reachable(&self, runtime: &ChannelRuntimeV1) -> bool {
        let session = self.fence.game_session_id;
        let ItemKey::MapBase { placement_key, .. } = self.target else {
            return false;
        };
        let Some((part, _)) = placement_of(placement_key) else {
            return false;
        };
        runtime.house_actor_state(self.actor, session).is_ok_and(|state| {
            state.scope == self.fence.runtime_scope
                && state.generation == self.fence.scope_ownership_generation
                && state.lease.generation() == self.fence.character_lease_generation
                && state.lease.character_id().as_bytes() == self.fence.character_id.as_bytes()
                && super::super::bed_sleep_production::bed_part_reachable(state.position, part)
        }) && runtime.house_player_control_facts(self.actor, session)
            .is_ok_and(|facts| facts.control_loss.is_none())
    }
}

struct PreparedBedUse {
    context: HouseActionContext,
    placement: BedUsePlacement,
    target: ItemKey,
}

/// Crystal 201189f game.cpp:261-272,10912-10990: native bed skill choice.
fn bed_dialog() -> Dialog {
    Dialog {
        id: 1,
        title: "Choose a Skill".into(),
        message: "Please choose a skill for offline training:".into(),
        choices: [
            (4, "Axe"),
            (2, "Club"),
            (5, "Distance"),
            (1, "Fist"),
            (6, "Magic Level"),
            (3, "Sword"),
        ]
        .into_iter()
        .map(|(skill, name)| {
            (
                skill,
                if skill == 6 {
                    format!("{name} and Shielding")
                } else {
                    format!("{name} Fighting and Shielding")
                },
            )
        })
        .collect(),
        buttons: vec![(1, "Okay".into()), (2, "Cancel".into())],
        default_button: Some(1),
        cancel_button: Some(2),
        image: None,
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn offer_bed_modal(
        &self,
        admitted: AdmittedSession,
        command_id: u64,
        target: UseTarget,
        resolved: Option<ItemKey>,
    ) -> Result<Option<UseOutcome>, BedUseError> {
        let budget = AdmissionRetryBudget::for_command(self.root);
        let resolved = tokio::time::timeout_at(
            budget.deadline,
            Box::pin(self.resolve_bed_use(admitted, target, resolved, &budget)),
        )
        .await;
        let PreparedBedUse {
            context,
            placement,
            target,
        } = match resolved {
            Ok(Ok(Some(prepared))) => prepared,
            Ok(Err(MapTargetRefusal::StateRevisionMismatch)) => return Err(BedUseError::Resync),
            Ok(Err(MapTargetRefusal::Use(disposition))) => {
                return Ok(Some(UseOutcome {
                    disposition,
                    ..UseOutcome::rejected()
                }));
            }
            Ok(Ok(None)) | Err(_) => return Ok(None),
        };
        // Crystal 201189f bed.cpp:102-123 applies Premium before USE.
        if !self
            .session_premium_gate(admitted.game_session_id)
            .await
            .premium_current()
        {
            return Ok(Some(UseOutcome {
                disposition: UseDisposition::Sealed,
                gate_message_id: Some(BED_PREMIUM_MESSAGE_ID),
                ..UseOutcome::rejected()
            }));
        }
        // Sleep and House movement already use this lane. Keep the vacancy
        // observation and the modal claim in the same turn as those writers.
        let Ok(Some(_lane)) =
            tokio::time::timeout_at(budget.deadline, self.spell_lane_permit()).await
        else {
            return Ok(Some(UseOutcome {
                disposition: UseDisposition::Exhausted,
                ..UseOutcome::rejected()
            }));
        };
        let fence = context.fence();
        // Crystal 201189f bed.cpp:162-175,221-259: occupied USE ends at refusal or wake.
        let decision =
            Box::pin(self.production_bed_use(admitted, command_id, context, placement, &budget))
                .await?;
        let disposition = match decision {
            BedUseOutcome::Occupied => UseDisposition::BedOccupied,
            BedUseOutcome::Woken => UseDisposition::BedWoken,
            BedUseOutcome::Denied => UseDisposition::Rejected,
            BedUseOutcome::Stale => UseDisposition::StaleState,
            BedUseOutcome::CheckoutUnavailable | BedUseOutcome::RolledBack => {
                UseDisposition::Exhausted
            }
            BedUseOutcome::Vacant(access) => {
                self.offer_vacant_bed(admitted, target, placement, fence, access).await
            }
        };
        Ok(Some(UseOutcome {
            disposition,
            ..UseOutcome::rejected()
        }))
    }

    async fn offer_vacant_bed(
        &self,
        admitted: AdmittedSession,
        target: ItemKey,
        placement: BedUsePlacement,
        fence: CurrentCharacterItemFence,
        access: BedAccessEvidence,
    ) -> UseDisposition {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_SERVER_MODAL_V1)
        {
            return UseDisposition::Rejected;
        }
        let runtime = self.runtime.lock().await;
        let Some(root) = self.states.server_messages.connection_root(
            admitted.game_session_id,
            admitted.continuity.connection_generation,
        ) else {
            return UseDisposition::Rejected;
        };
        let Some(actor) = admitted.runtime_actor else {
            return UseDisposition::Rejected;
        };
        let selection = BedSelection { target, placement, fence, actor, access };
        self.states.server_modals.release_unreachable_beds(&runtime, fence.runtime_scope);
        if actor_is_current(&runtime, admitted)
            && selection.reachable(&runtime)
            && self.states.server_modals.accepts_terminal_reply(root)
            && self
                .states
                .server_modals
                .offer_rule(root, bed_dialog(), Selection::Bed(selection))
                .is_some()
        {
            UseDisposition::BedChooseSkill
        } else {
            UseDisposition::Rejected
        }
    }

    async fn resolve_bed_use(
        &self,
        admitted: AdmittedSession,
        target: UseTarget,
        resolved: Option<ItemKey>,
        budget: &AdmissionRetryBudget<'_>,
    ) -> Result<Option<PreparedBedUse>, MapTargetRefusal> {
        if !admitted
            .continuity
            .selected_capabilities
            .contains(CAPABILITY_WORLD_INTERACTION_V1)
            || (matches!(&target, UseTarget::Item(_))
                && !matches!(resolved, Some(ItemKey::MapBase { .. })))
        {
            return Ok(None);
        }
        let (Some(actor), Some(fence), Some(map)) = (
            admitted.runtime_actor,
            admitted.item_fence,
            self.bundle_map(),
        ) else {
            return Ok(None);
        };
        let context = {
            let runtime = self.runtime.lock().await;
            if !actor_is_current(&runtime, admitted) {
                return Ok(None);
            }
            let Ok(context) = HouseActionContext::from_native_owner(
                &runtime,
                actor,
                admitted.game_session_id,
                fence,
            ) else {
                return Ok(None);
            };
            context
        };
        let bundle_digest = map.overlay.base().digest();
        let Some(changes) = self
            .house_base_decoration_facts_with_retry(&context, bundle_digest, Some(budget))
            .await
        else {
            return Ok(None);
        };
        let runtime = self.runtime.lock().await;
        if !actor_is_current(&runtime, admitted) {
            return Ok(None);
        }
        let base = map.facts.base_only();
        let facts = DecoratedFacts {
            base: &base,
            changes: &changes,
        };
        let source = MapViewSource {
            overlay: &map.overlay,
            facts: &facts,
            content_generation: runtime.content_pin().client_artifact_digest(),
            reset_epoch: self.reset_epoch,
        };
        let target = match target {
            UseTarget::Item(_) => {
                let Some(key) = resolved else {
                    return Ok(None);
                };
                resolve_map_handle(&source, key).map_err(MapTargetRefusal::Use)?;
                key
            }
            UseTarget::WorldObject(target) => {
                match resolve_map_target(&source, &target, |_| None::<()>) {
                    Ok(key) => ItemKey::MapBase {
                        bundle_digest,
                        placement_key: key.placement_key,
                        reset_epoch: self.reset_epoch,
                    },
                    Err(MapTargetRefusal::Use(UseDisposition::NothingToUse)) => return Ok(None),
                    Err(refusal) => return Err(refusal),
                }
            }
            _ => return Ok(None),
        };
        let Some(placement) = self
            .bed_use_placement(&runtime, admitted, target, &changes)
            .map_err(MapTargetRefusal::Use)? else {
            return Ok(None);
        };
        if !self
            .states
            .world_gate
            .lock()
            .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
        {
            return Err(MapTargetRefusal::Use(UseDisposition::Exhausted));
        }
        Ok(Some(PreparedBedUse {
            context,
            placement,
            target,
        }))
    }
}

pub(super) async fn answer(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    root: Root,
    command_id: u64,
    payload: &[u8],
) -> Option<Option<ServerModalOutcome>> {
    let answer = Answer::decode(payload).ok()?;
    // The bed dialog asks for no input, so an answer carrying a value never reaches it.
    let (Selection::Bed(selection), None, _) =
        owner.states.server_modals.rule_selection(root, answer)?
    else {
        return None;
    };
    Some(complete(owner, admitted, root, command_id, answer, selection).await)
}

async fn complete(
    owner: &ComposedFreshAdmission<'_, '_, '_>,
    admitted: AdmittedSession,
    root: Root,
    command_id: u64,
    answer: Answer,
    selection: BedSelection,
) -> Option<ServerModalOutcome> {
    if answer.button_id == 2 {
        return owner
            .states
            .server_modals
            .finish_rule(root, answer)
            .map(ServerModalOutcome::Reply);
    }
    if admitted.item_fence != Some(selection.fence)
        || !owner.states.server_modals.accepts_terminal_reply(root)
    {
        owner.states.server_modals.refused_rule(root, answer)?;
        return None;
    }
    let actor = admitted.runtime_actor?;
    let acknowledgement = Reply::AnsweredAndLoggedOut(answer).encode().ok()?;
    // The durable owner future is deliberately kept off the connection's stack,
    // as on the existing command-31 path.
    let outcome = Box::pin(owner.production_bed_skill(
        actor,
        UseCommand {
            game_session_id: admitted.game_session_id,
            command_id,
            item_fence: admitted.item_fence,
        },
        answer.choice_id?,
        selection.target,
        Some(BedReservation { placement: selection.placement, access: selection.access }),
    ))
    .await;
    match outcome {
        Ok(outcome) if outcome.disposition == UseDisposition::OfflineTraining => {
            // The native owner committed the final release. Send its exact modal ACK
            // and end the connection; do not attempt another current-session mutation.
            let _ = owner
                .states
                .server_modals
                .finish_terminal_rule(root, answer);
            Some(ServerModalOutcome::LoggedOut(acknowledgement))
        }
        Ok(outcome) if outcome.disposition == UseDisposition::StaleState => {
            owner.states.server_modals.release_bed(root);
            None
        }
        Ok(_) => {
            owner.states.server_modals.refused_rule(root, answer)?;
            None
        }
        Err(_) => Some(ServerModalOutcome::Unavailable),
    }
}

#[cfg(test)]
#[path = "rules_tests.rs"]
mod tests;
