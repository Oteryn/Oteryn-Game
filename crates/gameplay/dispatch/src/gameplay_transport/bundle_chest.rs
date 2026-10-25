//! Source unique-id chests on the bundle World's real USE route. Content owns the
//! binding; DUR-03 owns the claim, reservation, replay and atomic backpack MINT.

use super::ComposedFreshAdmission;
use super::connection::{EarnedNotice, UseCommand, UseOutcome};
use super::item_ref_admission::{generation_item_facts, pinned_index};
use super::world_map::{MapViewSource, resolve_map_target};
use crate::content::world_reward_claims::bind_embedded_reward_claims;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::{RewardClaimMintError, RewardClaimMintRequest};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use crate::interaction_chest_use::{ChestUseError, chest_use_disposition, entry_chest};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

fn outcome(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        committed: None,
        earned: EarnedNotice::NoneEarned,
        gate_message_id: None,
        refusal_detail: None,
    }
}

fn item_ref(item: &crate::content::TypedDefinitionRef) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: item.key().as_str().into(),
        revision_ref: item.revision().as_str().into(),
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// `None` leaves an unbound object to the existing door owner. A source chest
    /// must bind by position, unique id and appearance, never by client-supplied ids.
    pub(super) async fn use_bundle_chest(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &crate::map::boot::BundleMap,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        let native = self.active_generation?.native_gameplay()?;
        // This is a bounded, immutable Content pass over the embedded claim catalogue.
        // Unsupported shapes and unadmitted rewards remain closed in the existing binder.
        let report = bind_embedded_reward_claims(map.overlay.base().as_ref(), &|item| {
            generation_item_facts(native, &item_ref(item)).is_ok_and(|facts| {
                facts.container_capacity.is_none()
                    && !matches!(
                        facts.stack,
                        crate::durability::item_transfer::ItemStackClass::Unknown
                    )
            })
        })
        .ok()?;
        let source = MapViewSource {
            overlay: &map.overlay,
            facts: &map.facts,
            content_generation: self.active_generation?.identity().client_artifact_digest(),
            reset_epoch: self.reset_epoch,
        };
        let resolved = resolve_map_target(&source, target, |key| {
            report.served.iter().find_map(|claim| {
                claim
                    .placements
                    .iter()
                    .find(|placement| placement.bundle_placement_key == key)
                    .map(|placement| (claim, placement))
            })
        })
        .ok()?;
        let (claim, bound) = resolved.canonical?;
        // Quest-transition admission remains at channel boot for canonical Content.
        // Do not bypass that admission for an unmounted transition in a source shard.
        if bound.placement.quest_transition.is_some() {
            return Some(outcome(UseDisposition::Rejected));
        }
        Some(
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                let Some(_permit) = self.spell_lane_permit().await else {
                    return outcome(UseDisposition::Rejected);
                };
                {
                    let mut runtime = self.runtime.lock().await;
                    let states = self.states.spell_states.lock().await;
                    if !runtime
                        .player_control_facts(actor, command.game_session_id)
                        .is_ok_and(|facts| facts.control_loss.is_none())
                        || states.is_dead(actor)
                        || states.has_pending_spell_commit(actor, command.game_session_id)
                        || pinned_index(&runtime, self.active_generation).is_none()
                    {
                        return outcome(UseDisposition::Rejected);
                    }
                    drop(states);
                    let Ok(position) = runtime.borrow_movement_position().read(actor) else {
                        return outcome(UseDisposition::Rejected);
                    };
                    if position.context() != runtime.pinned_movement_context() {
                        return outcome(UseDisposition::Rejected);
                    }
                    let position = position.position();
                    if position.x.abs_diff(i32::from(bound.cell.x)) > 1
                        || position.y.abs_diff(i32::from(bound.cell.y)) > 1
                        || i32::from(position.floor) != -i32::from(bound.cell.floor)
                    {
                        return outcome(UseDisposition::TooFar);
                    }
                    if !self
                        .states
                        .world_gate
                        .lock()
                        .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
                    {
                        return outcome(UseDisposition::Exhausted);
                    }
                }
                let (Some(fence), Ok(command_id), [reward]) = (
                    command.item_fence,
                    CommandId::new(command.command_id),
                    bound.placement.items.as_slice(),
                ) else {
                    return outcome(UseDisposition::Rejected);
                };
                let Ok(Some(bag)) = self
                    .root
                    .read_character_backpack(self.character, fence.character_id)
                    .await
                else {
                    return outcome(UseDisposition::Rejected);
                };
                let (Ok(item), Ok(backpack)) = (
                    generation_item_facts(native, &item_ref(&reward.item)),
                    generation_item_facts(native, &bag.backpack.definition),
                ) else {
                    return outcome(UseDisposition::Rejected);
                };
                let request = RewardClaimMintRequest {
                    command: CommandRef::new(command.game_session_id, command_id),
                    claim: TypedDefinitionRef {
                        family: "RewardClaim".into(),
                        production_key: claim.key.as_str().into(),
                        revision_ref: claim.revision.as_str().into(),
                    },
                    source_placement: bound.placement.placement.as_str().into(),
                    item,
                    quantity: reward.count,
                    backpack,
                    content_revision: format!(
                        "sha256:{}",
                        native
                            .source_digest()
                            .iter()
                            .map(|byte| format!("{byte:02x}"))
                            .collect::<String>()
                    ),
                    ruleset_revision: entry_chest::RULESET_REVISION.into(),
                    sim_revision: entry_chest::SIM_REVISION.into(),
                    achievement: None,
                    quest_transition: None,
                    key_number: bound.placement.key_number,
                };
                let result = async {
                    let mut candidate = self
                        .root
                        .freeze_reward_claim_mint(self.character, self.holder, fence, request)
                        .await?;
                    self.root
                        .commit_reward_claim_mint_noticed(
                            self.character,
                            self.holder,
                            fence,
                            &mut candidate,
                            Some(self.achievements.len()),
                        )
                        .await
                }
                .await;
                match result {
                    Ok((_, notice)) => UseOutcome {
                        earned: super::earned_notice(
                            self.achievements,
                            notice,
                            &mut super::operator_event,
                        ),
                        ..outcome(UseDisposition::Committed)
                    },
                    Err(error) => mint_refusal(error),
                }
            })
            .await
            .unwrap_or_else(|_| outcome(UseDisposition::Rejected)),
        )
    }
}

fn mint_refusal(error: RewardClaimMintError) -> UseOutcome {
    outcome(chest_use_disposition(&ChestUseError::Mint(error)))
}
