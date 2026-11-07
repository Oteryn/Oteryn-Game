//! One path for fresh and reconciled native selector rewards.
use super::*;
use crate::durability::character_progression::CharacterProgressionError;
use crate::durability::character_revision_sequencer::RevisionSlot;
use oteryn_gameplay_quests::quest::QuestStateCatalogue;

impl ComposedFreshAdmission<'_, '_, '_> {
    #[allow(
        clippy::too_many_arguments,
        reason = "the existing selector context and sequenced transaction retain one admission"
    )]
    pub(super) async fn commit_bound_world_callback(
        &self,
        slot: &mut RevisionSlot,
        fence: CurrentCharacterGameplayFence,
        request: QuestTransitionRequest,
        catalogue: std::sync::Arc<QuestStateCatalogue>,
        permit: &SpellLanePermit,
        context: QuestWorldContext,
    ) -> Result<QuestTransitionOutcome, CharacterProgressionError> {
        let callback = self
            .callback_content()
            .iter()
            .find(|c| c.key == request.transition_key)
            .ok_or(CharacterProgressionError::InvalidInput)?;
        let native = self.active_generation.and_then(|g| g.native_gameplay());
        if callback.effects.item_reward.is_some() {
            let native = native.ok_or(CharacterProgressionError::InvalidInput)?;
            let weights = super::super::item_ref_admission::NativeItemWeights(native)
                .with_wheel(self.premium_coordinator);
            slot.commit_world_callback_reward(
                self.root,
                self.character,
                self.holder,
                fence,
                request,
                catalogue,
                permit,
                context,
                crate::durability::reward_claim_mint::QuestRewardAdmission {
                    native,
                    weights: &weights,
                },
            )
            .await
        } else {
            slot.commit_world_callback(
                self.root,
                self.character,
                self.holder,
                fence,
                request,
                catalogue,
                permit,
                context,
                native,
            )
            .await
        }
    }
}
