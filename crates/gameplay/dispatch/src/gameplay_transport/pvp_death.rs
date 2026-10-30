//! Dispatch wiring for the existing death pipeline and its qualified Open-PvP input.
use super::ComposedFreshAdmission;
use crate::durability::character_build::BuildFormula;
use crate::durability::character_death::promotion::DeathPremiumOwner;
use crate::durability::character_death::{CharacterDeathRequest, CommittedCharacterDeath};
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn settle_source_player_death<const N: usize>(
        &self,
        fence: CurrentCharacterGameplayFence,
        request: CharacterDeathRequest<N>,
        formula: Option<&dyn BuildFormula>,
        premium: Option<&dyn DeathPremiumOwner>,
    ) -> Result<CommittedCharacterDeath, CharacterProgressionError> {
        let permit = self
            .spell_lane_permit()
            .await
            .ok_or(CharacterProgressionError::AuthorityRejected)?;
        let root = self
            .runtime
            .lock()
            .await
            .content_pin()
            .server_artifact_digest();
        let death = self
            .root
            .settle_runtime_player_death_with_items(
                &self.revision_sequencer,
                self.character,
                self.holder,
                fence,
                request,
                formula,
                premium,
                Some(root),
            )
            .await?;
        self.apply_player_death_items(&permit, fence, &death)
            .await?;
        Ok(death)
    }
}
