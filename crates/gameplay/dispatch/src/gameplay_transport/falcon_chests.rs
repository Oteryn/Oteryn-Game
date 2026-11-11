//! Literal native Falcon chest USE, resolved by exact source and pinned items.
use super::ComposedFreshAdmission;
use super::connection::{EarnedNotice, UseCommand, UseOutcome};
use super::item_ref_admission::{NativeItemWeights, generation_item_facts, pinned_index};
use super::world_map::{MapViewSource, resolve_map_target};
use crate::content::world_reward_claims::RewardClaimBase;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::{RewardClaimMintError, RewardClaimMintRequest};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_gameplay_quests::quest::falcon::{CHESTS, REWARDS};
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

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn use_falcon_chest(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &crate::map::boot::BundleMap,
        target: &WorldObjectTarget,
    ) -> Option<UseOutcome> {
        let generation = self.active_generation?;
        let native = generation.native_gameplay()?;
        let source = MapViewSource {
            overlay: &map.overlay,
            facts: &map.facts,
            content_generation: generation.identity().client_artifact_digest(),
            reset_epoch: self.reset_epoch,
        };
        let resolved = resolve_map_target(&source, target, |_| Some(())).ok()?;
        let chest = CHESTS
            .into_iter()
            .find(|chest| chest.position == (resolved.pos.x, resolved.pos.y, resolved.pos.floor))?;
        let base = map.overlay.base();
        let tile = RewardClaimBase::tile(
            base.as_ref(),
            resolved.pos.x,
            resolved.pos.y,
            resolved.pos.floor,
        )?;
        let entry = tile.unique(resolved.ordinal)?;
        if entry.unique != chest.unique || entry.appearance != Some(chest.appearance) {
            return None;
        }
        // Ambiguous duplicate source IDs never grant a second placement choice.
        let matching = (0..tile.depths().iter().filter(|depth| **depth == 0).count())
            .filter_map(|ordinal| {
                u8::try_from(ordinal)
                    .ok()
                    .and_then(|ordinal| tile.unique(ordinal))
            })
            .filter(|entry| entry.unique == chest.unique)
            .count();
        if matching != 1 {
            return Some(outcome(UseDisposition::Rejected));
        }
        Some(
            tokio::time::timeout(std::time::Duration::from_secs(10), async {
                let Some(_permit) = self.spell_lane_permit().await else {
                    return outcome(UseDisposition::Rejected);
                };
                let requests = {
                    let mut runtime = self.runtime.lock().await;
                    // Do not suspend while holding the runtime owner lock.
                    let Ok(states) = self.states.spell_states.try_lock() else {
                        return outcome(UseDisposition::Rejected);
                    };
                    if !runtime
                        .player_control_facts(actor, command.game_session_id)
                        .is_ok_and(|facts| facts.control_loss.is_none())
                        || states.is_dead(actor)
                        || !states.spell_idle(&runtime, actor, command.game_session_id)
                        || !self
                            .states
                            .player_trades
                            .physical(command.game_session_id, None)
                    {
                        return outcome(UseDisposition::Rejected);
                    }
                    let Some(index) = pinned_index(&runtime, self.active_generation) else {
                        return outcome(UseDisposition::Rejected);
                    };
                    let Ok(position) = runtime.borrow_movement_position().read(actor) else {
                        return outcome(UseDisposition::Rejected);
                    };
                    if position.context() != runtime.pinned_movement_context() {
                        return outcome(UseDisposition::Rejected);
                    }
                    let position = position.position();
                    if position.x.abs_diff(i32::from(chest.position.0)) > 1
                        || position.y.abs_diff(i32::from(chest.position.1)) > 1
                        || i32::from(position.floor) != -i32::from(chest.position.2)
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
                    REWARDS
                        .iter()
                        .map(|(id, count)| {
                            let key = format!("oteryn:item.tibia.i{id}");
                            let revision = index.revision(&key)?;
                            let item = generation_item_facts(
                                native,
                                &TypedDefinitionRef {
                                    family: "Item".into(),
                                    production_key: key,
                                    revision_ref: revision.into(),
                                },
                            )
                            .ok()?;
                            Some((item, *count))
                        })
                        .collect::<Option<Vec<_>>>()
                };
                let Some(rewards) = requests else {
                    return outcome(UseDisposition::Rejected);
                };
                let (Some(fence), Ok(id)) =
                    (command.item_fence, CommandId::new(command.command_id))
                else {
                    return outcome(UseDisposition::Rejected);
                };
                let Ok(Some(bag)) = self
                    .root
                    .read_character_backpack(self.character, fence.character_id)
                    .await
                else {
                    return outcome(UseDisposition::Rejected);
                };
                let Ok(backpack) = generation_item_facts(native, &bag.backpack.definition) else {
                    return outcome(UseDisposition::Rejected);
                };
                let content_revision = format!(
                    "sha256:{}",
                    native
                        .source_digest()
                        .iter()
                        .map(|b| format!("{b:02x}"))
                        .collect::<String>()
                );
                let requests = rewards
                    .into_iter()
                    .map(|(item, quantity)| RewardClaimMintRequest {
                        additional_items: Vec::new(),
                        backpack_outputs: false,
                        command: CommandRef::new(command.game_session_id, id),
                        claim: TypedDefinitionRef {
                            family: "RewardClaim".into(),
                            production_key: chest.key(),
                            revision_ref: "reward-claim-r1".into(),
                        },
                        source_placement: chest.source(),
                        item,
                        quantity,
                        contents: Vec::new(),
                        root_text: None,
                        root_fluid_subtype: None,
                        root_charges_remaining: None,
                        backpack: backpack.clone(),
                        content_revision: content_revision.clone(),
                        ruleset_revision:
                            crate::interaction_chest_use::entry_chest::RULESET_REVISION.into(),
                        sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION
                            .into(),
                        achievement: None,
                        quest_transition: None,
                        key_number: None,
                    })
                    .collect::<Vec<_>>();
                let Ok(requests) = requests.try_into() else {
                    return outcome(UseDisposition::Rejected);
                };
                match self
                    .root
                    .claim_falcon_chest(
                        self.character,
                        self.holder,
                        fence,
                        requests,
                        &NativeItemWeights(native).with_wheel(self.premium_coordinator),
                    )
                    .await
                {
                    Ok(_) => outcome(UseDisposition::Committed),
                    Err(error) => falcon_refusal(error),
                }
            })
            .await
            .unwrap_or_else(|_| outcome(UseDisposition::Rejected)),
        )
    }
}
fn falcon_refusal(error: RewardClaimMintError) -> UseOutcome {
    outcome(crate::interaction_chest_use::chest_use_disposition(
        &crate::interaction_chest_use::ChestUseError::Mint(error),
    ))
}
