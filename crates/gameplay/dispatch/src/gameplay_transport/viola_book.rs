//! Current Book of Wisdom chest: exact committed selector and five-minute personal repeat.
use super::ComposedFreshAdmission;
use super::connection::{EarnedNotice, UseCommand, UseOutcome};
use super::item_ref_admission::{NativeItemWeights, generation_item_facts, pinned_index};
use super::world_map::{MapViewSource, resolve_map_target};
use crate::content::world_reward_claims::RewardClaimBase;
use crate::durability::item_mint::TypedDefinitionRef;
use crate::durability::reward_claim_mint::{RewardClaimMintError, RewardClaimMintRequest};
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use crate::map::view::MapFacts;
use oteryn_protocol_oteryn::world_object::{UseDisposition, WorldObjectTarget};

const POSITION: (u16, u16, i8) = (32614, 32419, -7);
const CLAIM: &str = "oteryn:reward-claim.tutorial.viola-book";
const SOURCE: &str = "oteryn:placement.tutorial.viola-book.32614-32419-7";

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
    pub(super) async fn use_viola_book(
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
        if (resolved.pos.x, resolved.pos.y, resolved.pos.floor) != POSITION {
            return None;
        }
        let base = map.overlay.base();
        let tile = RewardClaimBase::tile(base.as_ref(), POSITION.0, POSITION.1, POSITION.2)?;
        let id = tile
            .depths()
            .iter()
            .zip(tile.ids())
            .filter(|(depth, _)| **depth == 0)
            .nth(usize::from(resolved.ordinal))
            .map(|(_, id)| *id)?;
        let entry = map.facts.base_entry(resolved.pos, resolved.ordinal, id)?;
        if resolved.ordinal != 2
            || tile
                .unique(resolved.ordinal)
                .is_some_and(|entry| entry.unique != 0)
            || entry.appearance_id != 2472
        {
            return Some(outcome(UseDisposition::NothingToUse));
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
                    if position.x.abs_diff(i32::from(POSITION.0)) > 1
                        || position.y.abs_diff(i32::from(POSITION.1)) > 1
                        || i32::from(position.floor) != -i32::from(POSITION.2)
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
                    let key = "oteryn:item.tibia.i51977";
                    index.revision(key).and_then(|revision| {
                        generation_item_facts(
                            native,
                            &TypedDefinitionRef {
                                family: "Item".into(),
                                production_key: key.into(),
                                revision_ref: revision.into(),
                            },
                        )
                        .ok()
                    })
                };
                let Some(item) = requests else {
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
                let request = RewardClaimMintRequest {
                    additional_items: Vec::new(),
                    backpack_outputs: false,
                    command: CommandRef::new(command.game_session_id, id),
                    claim: TypedDefinitionRef {
                        family: "RewardClaim".into(),
                        production_key: CLAIM.into(),
                        revision_ref: "reward-claim-r1".into(),
                    },
                    source_placement: SOURCE.into(),
                    item,
                    quantity: 1,
                    contents: Vec::new(),
                    root_text: None,
                    root_fluid_subtype: None,
                    root_charges_remaining: None,
                    backpack,
                    content_revision,
                    ruleset_revision: crate::interaction_chest_use::entry_chest::RULESET_REVISION
                        .into(),
                    sim_revision: crate::interaction_chest_use::entry_chest::SIM_REVISION.into(),
                    achievement: None,
                    quest_transition: None,
                    key_number: None,
                };
                match self
                    .root
                    .claim_viola_book(
                        self.character,
                        self.holder,
                        fence,
                        request,
                        &NativeItemWeights(native).with_wheel(self.premium_coordinator),
                    )
                    .await
                {
                    Ok(_) => outcome(UseDisposition::Committed),
                    Err(error) => viola_book_refusal(error),
                }
            })
            .await
            .unwrap_or_else(|_| outcome(UseDisposition::Rejected)),
        )
    }
}
fn viola_book_refusal(error: RewardClaimMintError) -> UseOutcome {
    outcome(crate::interaction_chest_use::chest_use_disposition(
        &crate::interaction_chest_use::ChestUseError::Mint(error),
    ))
}
