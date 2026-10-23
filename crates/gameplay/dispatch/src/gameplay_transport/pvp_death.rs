//! Dispatch wiring for the existing death pipeline and its qualified Open-PvP input.
use super::ComposedFreshAdmission;
use crate::durability::character_build::BuildFormula;
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
    ) -> Result<CommittedCharacterDeath, CharacterProgressionError> {
        self.root
            .settle_runtime_player_death(
                &self.revision_sequencer,
                self.character,
                self.holder,
                fence,
                request,
                formula,
            )
            .await
    }
}
