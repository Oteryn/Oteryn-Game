//! A bound authored map claim borrows the fenced quest exchange owner. The
//! registry digest, allocation and callback identity must agree before any write.
use super::ComposedFreshAdmission;
use super::bundle_chest::{item_ref, outcome};
use super::connection::{UseCommand, UseOutcome};
use super::item_ref_admission::NativeItemWeights;
use crate::content::world_reward_claims::{BoundPlacement, ServedClaim};
use crate::durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use crate::durability::reward_claim_mint::QuestRewardAdmission;
use crate::durability::spell_owner_commit::SpellLanePermit;
use crate::foundation::{CommandId, CommandRef, ExactActorRef};
use oteryn_gameplay_quests::quest::reward_callbacks::{self, RewardCallback};
use oteryn_protocol_oteryn::world_object::UseDisposition;

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|byte| format!("{byte:02x}")).collect()
}

fn callback(claim: &ServedClaim, bound: &BoundPlacement) -> Option<RewardCallback> {
    let selected = bound.reward_callback.as_deref()?;
    let callback = reward_callbacks::load()
        .ok()?
        .into_iter()
        .find(|callback| callback.key == selected)?;
    let binding = callback.world_claim.as_ref()?;
    if binding.key != claim.key.as_str()
        || binding.revision != claim.revision.as_str()
        || binding.authority_sha256 != hex(&claim.authority_digest)
        || callback.carried_item.is_some()
    {
        return None;
    }
    let reward = callback.give.as_ref()?;
    let (root, additional) = bound.placement.items.split_first()?;
    if additional.len() != reward.additional.len()
        || reward
            .additional
            .iter()
            .zip(additional)
            .any(|(expected, actual)| {
                expected.item != actual.item.key().as_str()
                    || u32::from(expected.count) != actual.count
                    || expected.fluid_subtype.is_some()
            })
    {
        return None;
    }
    if reward.item != root.item.key().as_str()
        || u32::from(reward.count) != root.count
        || reward.contents.len() != bound.contents.len()
        || reward
            .contents
            .iter()
            .zip(&bound.contents)
            .any(|(expected, actual)| {
                expected.item != actual.item.key().as_str()
                    || u32::from(expected.count) != actual.count
                    || expected.fluid_subtype.is_some()
                    || actual.key_number.is_some()
                    || actual.text.is_some()
            })
        || reward.text != bound.root_text
    {
        return None;
    }
    Some(callback)
}

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn settle_native_claim_premium(
        &self,
        claim: &ServedClaim,
        bound: &BoundPlacement,
        command: UseCommand,
    ) {
        if callback(claim, bound).is_some_and(|callback| {
            callback.requires_premium || callback.give.is_some_and(|reward| reward.premium)
        }) {
            let _ = self.session_premium_gate(command.game_session_id).await;
        }
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "one immutable map claim and the exact live actor/command share the existing physical permit"
    )]
    pub(super) async fn use_native_claim_reward(
        &self,
        actor: ExactActorRef,
        command: UseCommand,
        map: &crate::map::boot::BundleMap,
        claim: &ServedClaim,
        bound: &BoundPlacement,
        permit: &SpellLanePermit,
    ) -> UseOutcome {
        let Some(callback) = callback(claim, bound) else {
            return outcome(UseDisposition::Rejected);
        };
        let Some(native) = self
            .active_generation
            .and_then(|generation| generation.native_gameplay())
        else {
            return outcome(UseDisposition::Rejected);
        };
        let Some(catalogue) = self.quest_catalogue.as_ref() else {
            return outcome(UseDisposition::Rejected);
        };
        let Ok(Some(fence)) = self.current_quest_fence(command.game_session_id).await else {
            return outcome(UseDisposition::Rejected);
        };
        if command.item_fence != Some(fence.item_fence()) {
            return outcome(UseDisposition::Rejected);
        }
        let context = {
            let runtime = self.runtime.lock().await;
            let Ok(snapshot) = runtime.read_actor_position(actor) else {
                return outcome(UseDisposition::Rejected);
            };
            let position = snapshot.position();
            if position.x.abs_diff(i32::from(bound.cell.x)) > 1
                || position.y.abs_diff(i32::from(bound.cell.y)) > 1
                || i32::from(position.floor) != -i32::from(bound.cell.floor)
            {
                return outcome(UseDisposition::TooFar);
            }
            crate::durability::quest_world_callback::QuestWorldContext {
                map_revision: crate::map::overlay::map_revision(map.overlay.base()),
                content_revision: format!(
                    "sha256:{}",
                    hex(&runtime.content_pin().server_artifact_digest())
                ),
                placement_context: runtime.content_pin().frame_binding_digest().to_vec(),
                reset_epoch: self.reset_epoch,
                actor_placement: actor.placement_identity(),
                movement_revision: snapshot.revision(),
            }
        };
        let Ok(command_id) = CommandId::new(command.command_id) else {
            return outcome(UseDisposition::Rejected);
        };
        let weights = NativeItemWeights(native).with_wheel(self.premium_coordinator);
        // The exact Item facts are resolved by the same admitted generation as
        // the map binder. The quest owner rechecks capacity and costs atomically.
        if bound.placement.items.iter().any(|item| {
            super::item_ref_admission::generation_item_facts(native, &item_ref(&item.item)).is_err()
        }) {
            return outcome(UseDisposition::Rejected);
        }
        self.schedule_quest_refresh(command.game_session_id);
        let result = self
            .revision_sequencer
            .acquire(fence.character_id)
            .await
            .commit_world_callback_with_items(
                self.root,
                self.character,
                self.holder,
                fence,
                QuestTransitionRequest {
                    transition_key: callback.key,
                    cause: QuestCause::Use(CommandRef::new(command.game_session_id, command_id)),
                },
                std::sync::Arc::clone(catalogue),
                permit,
                context,
                Some(native),
                Some(QuestRewardAdmission {
                    native,
                    weights: &weights,
                    fee_payment: None,
                    actor_conditions: None,
                    carried_use: None,
                    creature_use: None,
                    peer_use: None,
                }),
                &self.premium_coordinator,
            )
            .await;
        #[cfg(test)]
        eprintln!("ENGCLAIM_NATIVE_RESULT {result:?}");
        if result.is_ok() {
            self.refresh_quest_session(command.game_session_id, false)
                .await;
        }
        outcome(
            if matches!(
                result,
                Ok(QuestTransitionOutcome::Committed(_)
                    | QuestTransitionOutcome::AlreadyCommitted(_))
            ) {
                UseDisposition::Committed
            } else {
                UseDisposition::Rejected
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::world_reward_claims::{RewardClaimBase, RewardClaimTile, UniqueEntry};
    struct Base;
    struct Tile;
    impl RewardClaimBase for Base {
        type Tile<'a> = Tile;
        fn tile(&self, x: u16, y: u16, floor: i8) -> Option<Tile> {
            (x == 32552 && y == 32200 && floor == -11).then_some(Tile)
        }
    }
    impl RewardClaimTile for Tile {
        fn depths(&self) -> &[u8] {
            &[0, 0]
        }
        fn unique(&self, _: u8) -> Option<UniqueEntry> {
            None
        }
        fn appearance(&self, ordinal: u8) -> Option<u16> {
            (ordinal == 1).then_some(4240)
        }
    }
    #[test]
    fn engclaim_m34_allocation_authority_rejects_changed_digest() -> Result<(), String> {
        let report =
            crate::content::world_reward_claims::bind_embedded_reward_claims(&Base, &|_| true)
                .map_err(|e| e.to_string())?;
        let mut claim = report
            .served
            .into_iter()
            .find(|c| c.key.as_str().ends_with("kingdom_of_kormarak.corpse"))
            .ok_or("corpse")?;
        let bound = claim.placements.first().ok_or("placement")?;
        assert_eq!(
            callback(&claim, bound)
                .ok_or("exact allocation")?
                .give
                .ok_or("items")?
                .additional
                .len(),
            3
        );
        claim.authority_digest[0] ^= 1;
        assert!(callback(&claim, &claim.placements[0]).is_none());
        Ok(())
    }
}
