//! Source and authored chests on the bundle World's real USE route. Content owns the
//! binding; DUR-03 owns the claim, reservation, replay and atomic backpack MINT.

use super::ComposedFreshAdmission;
use super::connection::{EarnedNotice, UseCommand, UseOutcome};
use super::item_ref_admission::{NativeItemWeights, generation_item_facts, pinned_index};
use super::world_map::{MapViewSource, resolve_map_target};
use crate::content::world_reward_claims::{bind_embedded_reward_claims, bind_reward_claims};
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::{
    RewardClaimAchievement, RewardClaimMintContent, RewardClaimMintError, RewardClaimMintOutcome,
    RewardClaimMintRequest,
};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use crate::interaction_chest_use::{ChestUseError, chest_use_disposition, entry_chest};
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

#[derive(Debug, Clone, Copy)]
enum ChestRefusal {
    FenceUnavailable,
    StaleFence,
    AwardRoute,
    UnqualifiedClaim,
    QuestBinding,
    SpellLaneUnavailable,
    ActorUnavailable,
    Dead,
    ContentPin,
    PositionUnavailable,
    TooFar,
    Exhausted,
    ClaimInput,
    BackpackUnavailable,
    RewardItemFacts,
    InvalidFluid,
    Timeout,
}

fn refusal(command: UseCommand, reason: ChestRefusal, disposition: UseDisposition) -> UseOutcome {
    super::operator_event(&format!(
        "event=chest_command_refused command_id={} reason={reason:?}",
        command.command_id,
    ));
    outcome(disposition)
}

pub(super) fn outcome(disposition: UseDisposition) -> UseOutcome {
    UseOutcome {
        disposition,
        committed: None,
        earned: EarnedNotice::NoneEarned,
        reward_granted: None,
        gate_message_id: None,
        potion_utterance: None,
        refusal_detail: None,
    }
}

pub(super) fn item_ref(item: &crate::content::TypedDefinitionRef) -> TypedDefinitionRef {
    TypedDefinitionRef {
        family: "Item".into(),
        production_key: item.key().as_str().into(),
        revision_ref: item.revision().as_str().into(),
    }
}

impl ComposedFreshAdmission<'_, '_, '_> {
    /// `None` leaves an unbound object to the existing door owner. Content binds
    /// the exact position and appearance with a source UID or authored ordinal.
    pub(super) async fn use_bundle_chest(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &crate::map::boot::BundleMap,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        if let Some(result) = self.use_viola_book(actor, command, map, target).await {
            return Some(result);
        }
        if let Some(result) = self.use_falcon_chest(actor, command, map, target).await {
            return Some(result);
        }
        #[cfg(test)]
        if let Some(shards) = self.states.quest_callbacks.reward_claims_fixture.get() {
            return self
                .use_bundle_chest_from_shards(
                    actor,
                    command,
                    map,
                    target,
                    &shards.iter().map(String::as_str).collect::<Vec<_>>(),
                )
                .await;
        }
        self.use_bundle_chest_bound(actor, command, map, target, None)
            .await
    }

    /// Server-authored claim documents enter the same binding and durable USE
    /// path as the embedded catalogue. Shards are server content, never wire input.
    #[cfg(test)]
    pub(super) async fn use_bundle_chest_from_shards(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &crate::map::boot::BundleMap,
        target: &WorldObjectTarget,
        shards: &[&str],
    ) -> Option<UseOutcome> {
        self.use_bundle_chest_bound(actor, command, map, target, Some(shards))
            .await
    }

    /// `None` binds the embedded shards with their explicit native placement
    /// aliases; supplied fixture shards keep the strict binder, as before.
    async fn use_bundle_chest_bound(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &crate::map::boot::BundleMap,
        target: &WorldObjectTarget,
        shards: Option<&[&str]>,
    ) -> Option<UseOutcome> {
        let native = self.active_generation?.native_gameplay()?;
        // This is a bounded, immutable Content pass over the embedded claim catalogue.
        // Unsupported shapes and unadmitted rewards remain closed in the existing binder.
        let admitted = |item: &crate::content::TypedDefinitionRef| {
            generation_item_facts(native, &item_ref(item)).is_ok_and(|facts| {
                facts.container_capacity.is_none_or(|capacity| capacity > 0)
                    && !matches!(
                        facts.stack,
                        crate::durability::item_transfer::ItemStackClass::Unknown
                    )
            })
        };
        let report = match shards {
            Some(shards) => bind_reward_claims(map.overlay.base().as_ref(), shards, &admitted),
            None => bind_embedded_reward_claims(map.overlay.base().as_ref(), &admitted),
        }
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
        // Only bound reward claims take the durable fence. A busy holder
        // queues the claim within the command budget without intercepting doors.
        if let Some(expected) = command.item_fence {
            let retry = super::AdmissionRetryBudget::for_command(self.root);
            let current = tokio::time::timeout(
                std::time::Duration::from_secs(10),
                retry.retry_command_guard(async || {
                    self.current_quest_fence_durable(command.game_session_id)
                        .await
                }),
            )
            .await;
            let reason = match current {
                Ok(Ok(Some(current))) if current.item_fence() == expected => None,
                Ok(Ok(_)) => Some(ChestRefusal::StaleFence),
                Ok(Err(_)) => Some(ChestRefusal::FenceUnavailable),
                Err(_) => Some(ChestRefusal::Timeout),
            };
            if let Some(reason) = reason {
                return Some(refusal(command, reason, UseDisposition::Rejected));
            }
        }
        // The tower's authored final USE owns this same once claim and its progress.
        if claim.key.as_str()
            == oteryn_gameplay_quests::quest::world_callbacks::SERPENTINE_PEARL_CLAIM
        {
            return Some(refusal(
                command,
                ChestRefusal::AwardRoute,
                UseDisposition::Rejected,
            ));
        }
        if !super::native_claim_reward::admits_forgemaster_claim(
            map,
            claim,
            bound,
            self.quest_catalogue.as_deref(),
        ) {
            return Some(refusal(
                command,
                ChestRefusal::UnqualifiedClaim,
                UseDisposition::NotPossible,
            ));
        }
        let mut quest_transition = bound.placement.quest_transition.clone();
        match oteryn_gameplay_quests::quest::completion_sets::mint_transition_for_claim(
            claim.key.as_str(),
        ) {
            Ok(Some(key)) => {
                if bound.reward_callback.is_some()
                    || quest_transition
                        .as_ref()
                        .is_some_and(|current| current != key)
                {
                    return Some(refusal(
                        command,
                        ChestRefusal::QuestBinding,
                        UseDisposition::Rejected,
                    ));
                }
                quest_transition = Some(key.to_owned());
            }
            Ok(None) => {}
            Err(_) => {
                return Some(refusal(
                    command,
                    ChestRefusal::QuestBinding,
                    UseDisposition::Rejected,
                ));
            }
        }

        // Source shards bypass the entry-room Content check at boot. Resolve their
        // authored transition against this owner's loaded catalogue before any mint
        // reservation, just as canonical chests do at channel activation.
        if quest_transition.as_ref().is_some_and(|key| {
            self.quest_catalogue
                .as_ref()
                .and_then(|catalogue| catalogue.transition(key))
                .is_none()
        }) {
            return Some(refusal(
                command,
                ChestRefusal::QuestBinding,
                UseDisposition::Rejected,
            ));
        }
        if bound.reward_callback.is_some() {
            self.settle_native_claim_premium(claim, bound, command)
                .await;
        }
        Some(
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                let Some(permit) = self.spell_lane_permit().await else {
                    return refusal(
                        command,
                        ChestRefusal::SpellLaneUnavailable,
                        UseDisposition::Rejected,
                    );
                };
                {
                    let mut runtime = self.runtime.lock().await;
                    // Wait inside the existing chest deadline. An observation
                    // of player state cannot disqualify the chest by holding its lock.
                    let states = self.states.spell_states.lock().await;
                    if !runtime
                        .player_control_facts(actor, command.game_session_id)
                        .is_ok_and(|facts| facts.control_loss.is_none())
                    {
                        return refusal(
                            command,
                            ChestRefusal::ActorUnavailable,
                            UseDisposition::Rejected,
                        );
                    }
                    if states.is_dead(actor) {
                        return refusal(command, ChestRefusal::Dead, UseDisposition::Rejected);
                    }
                    if pinned_index(&runtime, self.active_generation).is_none() {
                        return refusal(
                            command,
                            ChestRefusal::ContentPin,
                            UseDisposition::Rejected,
                        );
                    }
                    drop(states);
                    let Ok(position) = runtime.borrow_movement_position().read(actor) else {
                        return refusal(
                            command,
                            ChestRefusal::PositionUnavailable,
                            UseDisposition::Rejected,
                        );
                    };
                    if position.context() != runtime.pinned_movement_context() {
                        return refusal(
                            command,
                            ChestRefusal::PositionUnavailable,
                            UseDisposition::Rejected,
                        );
                    }
                    let position = position.position();
                    if position.x.abs_diff(i32::from(bound.cell.x)) > 1
                        || position.y.abs_diff(i32::from(bound.cell.y)) > 1
                        || i32::from(position.floor) != -i32::from(bound.cell.floor)
                    {
                        return refusal(command, ChestRefusal::TooFar, UseDisposition::TooFar);
                    }
                    if !self
                        .states
                        .world_gate
                        .lock()
                        .is_ok_and(|mut gate| gate.admit(actor, self.clock_origin.elapsed()))
                    {
                        return refusal(
                            command,
                            ChestRefusal::Exhausted,
                            UseDisposition::Exhausted,
                        );
                    }
                }
                if bound.reward_callback.is_some() {
                    return self
                        .use_native_claim_reward(actor, command, map, claim, bound, &permit)
                        .await;
                }
                let (Some(fence), Ok(command_id), Some((reward, additional))) = (
                    command.item_fence,
                    CommandId::new(command.command_id),
                    bound.placement.items.split_first(),
                ) else {
                    return refusal(command, ChestRefusal::ClaimInput, UseDisposition::Rejected);
                };
                let Ok(Some(bag)) = self
                    .root
                    .read_character_backpack(self.character, fence.character_id)
                    .await
                else {
                    return refusal(
                        command,
                        ChestRefusal::BackpackUnavailable,
                        UseDisposition::Rejected,
                    );
                };
                let (Ok(item), Ok(backpack)) = (
                    generation_item_facts(native, &item_ref(&reward.item)),
                    generation_item_facts(native, &bag.backpack.definition),
                ) else {
                    return refusal(
                        command,
                        ChestRefusal::RewardItemFacts,
                        UseDisposition::Rejected,
                    );
                };
                // Match the quest reward owner's variable-vessel admission. A
                // content fluid modifier cannot turn an ordinary Item into a vessel.
                if bound.contents.iter().any(|child| {
                    child.fluid_subtype.is_some()
                        && !native
                            .item_policy(child.item.key().as_str(), child.item.revision().as_str())
                            .is_some_and(|policy| {
                                matches!(
                                    &policy.record().semantics.fluid,
                                    crate::content::ReferenceItemField::Known(
                                        crate::content::ReferenceItemFluid {
                                            fluid_type:
                                                crate::content::ReferenceItemField::NotApplicable
                                        }
                                    )
                                )
                            })
                }) {
                    return refusal(
                        command,
                        ChestRefusal::InvalidFluid,
                        UseDisposition::Rejected,
                    );
                }
                let contents = bound
                    .contents
                    .iter()
                    .map(|child| {
                        generation_item_facts(native, &item_ref(&child.item)).map(|item| {
                            RewardClaimMintContent {
                                item,
                                quantity: child.count,
                                key_number: child.key_number,
                                text: child.text.clone(),
                                fluid_subtype: child.fluid_subtype,
                                charges_remaining: child.charges_remaining,
                            }
                        })
                    })
                    .collect::<Result<Vec<_>, _>>();
                let Ok(contents) = contents else {
                    return refusal(
                        command,
                        ChestRefusal::RewardItemFacts,
                        UseDisposition::Rejected,
                    );
                };
                let additional_items = additional
                    .iter()
                    .map(|reward| {
                        generation_item_facts(native, &item_ref(&reward.item)).map(|item| {
                            RewardClaimMintContent {
                                fluid_subtype: None,
                                item,
                                quantity: reward.count,
                                key_number: None,
                                text: None,
                                charges_remaining: None,
                            }
                        })
                    })
                    .collect::<Result<Vec<_>, _>>();
                let Ok(additional_items) = additional_items else {
                    return refusal(
                        command,
                        ChestRefusal::RewardItemFacts,
                        UseDisposition::Rejected,
                    );
                };
                let request = RewardClaimMintRequest {
                    backpack_outputs: false,
                    additional_items,
                    command: CommandRef::new(command.game_session_id, command_id),
                    claim: TypedDefinitionRef {
                        family: "RewardClaim".into(),
                        production_key: claim.key.as_str().into(),
                        revision_ref: claim.revision.as_str().into(),
                    },
                    source_placement: bound.placement.placement.as_str().into(),
                    item,
                    quantity: reward.count,
                    contents,
                    root_text: bound.root_text.clone(),
                    root_fluid_subtype: bound.root_fluid_subtype,
                    root_charges_remaining: bound.root_charges_remaining,
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
                    achievement: bound.placement.achievement.as_ref().map(|key| {
                        RewardClaimAchievement {
                            key: key.clone(),
                            catalogue: self.achievements.lookup(key),
                        }
                    }),
                    quest_transition: quest_transition.clone(),
                    key_number: bound.placement.key_number,
                };
                // Arm recovery before the commit: the deadline may cancel an
                // already committed mint before its acknowledgement reaches us.
                if quest_transition.is_some() {
                    self.schedule_quest_refresh(command.game_session_id);
                }
                #[cfg(test)]
                hold_reward_mint_checkout_for_test(self.root).await;
                let result = self
                    .root
                    .mint_bundle_chest_reward(
                        self.character,
                        self.holder,
                        fence,
                        request,
                        self.achievements.len(),
                        &NativeItemWeights(native),
                    )
                    .await;
                drop(permit);
                // The mint owns the atomic obligation insert. The quest owner applies
                // it in its revision slot; admission/cadence resumes an unacknowledged
                // commit without minting the reward or advancing the track twice.
                if quest_transition.is_some() && result.is_ok() {
                    self.refresh_quest_session(command.game_session_id, false)
                        .await;
                }
                match result {
                    Ok((mint, notice)) => {
                        super::quest_collections::after_reward(self, actor, command).await;
                        UseOutcome {
                            earned: super::earned_notice(
                                self.achievements,
                                notice,
                                &mut super::operator_event,
                            ),
                            reward_granted: Some(reward_occurrence(mint)),
                            ..outcome(UseDisposition::Committed)
                        }
                    }
                    Err(error) => mint_refusal(command, error),
                }
            })
            .await
            .unwrap_or_else(|_| refusal(command, ChestRefusal::Timeout, UseDisposition::Rejected)),
        )
    }
}

pub(super) fn reward_occurrence(mint: RewardClaimMintOutcome) -> [u8; 16] {
    match mint {
        RewardClaimMintOutcome::Committed(receipt)
        | RewardClaimMintOutcome::AlreadyCommitted(receipt) => receipt.event_id,
    }
}

fn mint_refusal(command: UseCommand, error: RewardClaimMintError) -> UseOutcome {
    let reason = match &error {
        RewardClaimMintError::Refused(reason) => format!("{reason:?}"),
        RewardClaimMintError::InvalidInput => "MintInvalidInput".into(),
        RewardClaimMintError::CapacityExceeded => "MintCapacityExceeded".into(),
        RewardClaimMintError::AuthorityRejected => "MintAuthorityRejected".into(),
        RewardClaimMintError::ConflictingCause => "ConflictingCause".into(),
        RewardClaimMintError::ConflictingCandidate => "ConflictingCandidate".into(),
        RewardClaimMintError::UnknownAchievement => "UnknownAchievement".into(),
        RewardClaimMintError::Unavailable(crate::durability::DurabilityError::RootUnavailable) => {
            "MintHolderBusy".into()
        }
        RewardClaimMintError::Unavailable(_) => "MintUnavailable".into(),
    };
    super::operator_event(&format!(
        "event=chest_command_refused command_id={} reason={reason}",
        command.command_id,
    ));
    outcome(chest_use_disposition(&ChestUseError::Mint(error)))
}

#[cfg(test)]
tokio::task_local! {
    static REWARD_MINT_HOLDER: std::cell::Cell<(Option<std::time::Duration>, bool)>;
}

#[cfg(test)]
pub(super) async fn with_busy_reward_mint_holder<F: std::future::Future>(
    duration: std::time::Duration,
    operation: F,
) -> (F::Output, bool) {
    REWARD_MINT_HOLDER
        .scope(std::cell::Cell::new((Some(duration), false)), async {
            let result = operation.await;
            (result, REWARD_MINT_HOLDER.with(|state| state.get().1))
        })
        .await
}

#[cfg(test)]
async fn hold_reward_mint_checkout_for_test(root: &crate::durability::DurabilityRoot) {
    let Ok(Some(duration)) = REWARD_MINT_HOLDER.try_with(|state| {
        let (duration, held) = state.get();
        state.set((None, held));
        duration
    }) else {
        return;
    };
    let Ok(holder) = root.try_issue_semantic_pass() else {
        return;
    };
    REWARD_MINT_HOLDER.with(|state| state.set((None, true)));
    tokio::spawn(async move {
        holder
            .run(move |_, _| {
                Box::pin(async move {
                    tokio::time::sleep(duration).await;
                    Ok(())
                })
            })
            .await
    });
}
