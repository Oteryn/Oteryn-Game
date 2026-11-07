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
        let house = {
            let runtime = self.runtime.lock().await;
            runtime
                .house_player_for_session(fence.game_session_id)
                .ok()
                .flatten()
                .and_then(|(_, state)| {
                    crate::durability::house_scope_handoff::HouseId::new(
                        self.world_id,
                        &state.house,
                    )
                })
        };
        if let Some(house) = house {
            let death = self
                .root
                .settle_runtime_house_player_death_with_items(
                    &self.revision_sequencer,
                    self.character,
                    self.holder,
                    &house,
                    fence,
                    request,
                    formula,
                    premium,
                    Some(root),
                )
                .await?;
            self.apply_house_player_death_items(&permit, &house, fence, &death)
                .await?;
            return Ok(death);
        }
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
