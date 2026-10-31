//! C20b: drive receipt-selected one-item operations before any player respawn.
use super::ComposedFreshAdmission;
use crate::durability::character_death::{CommittedCharacterDeath, PlayerDeathOccurrence};
use crate::durability::character_progression::{
    CharacterProgressionError, CurrentCharacterGameplayFence,
};
use crate::durability::player_death_items::{
    PlayerDeathAmuletCause, PlayerDeathDropCause, PlayerDeathItemRequest,
};
use crate::durability::spell_owner_commit::SpellLanePermit;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn resume_pending_player_death_items(
        &self,
        permit: &SpellLanePermit,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<(), CharacterProgressionError> {
        if let Some(occurrence) = self
            .root
            .pending_player_death_item_occurrence(self.character, self.holder, fence)
            .await?
        {
            self.resume_player_death_items(permit, fence, occurrence)
                .await?;
        }
        Ok(())
    }

    pub(super) async fn resume_player_death_items(
        &self,
        permit: &SpellLanePermit,
        fence: CurrentCharacterGameplayFence,
        occurrence: PlayerDeathOccurrence,
    ) -> Result<(), CharacterProgressionError> {
        let death = self
            .root
            .reconcile_character_death(self.character, occurrence)
            .await?
            .ok_or(CharacterProgressionError::InvalidInput)?;
        // An old receipt has no item obligation. History is never redrawn.
        if death.items.is_none() {
            return Ok(());
        }
        self.apply_player_death_items(permit, fence, &death).await
    }

    pub(super) async fn apply_player_death_items(
        &self,
        permit: &SpellLanePermit,
        fence: CurrentCharacterGameplayFence,
        death: &CommittedCharacterDeath,
    ) -> Result<(), CharacterProgressionError> {
        if death.character_id != fence.character_id {
            return Err(CharacterProgressionError::AuthorityRejected);
        }
        let Some(items) = &death.items else {
            return Ok(());
        };
        for (item, burn) in items
            .lost
            .iter()
            .map(|id| (*id, false))
            .chain(items.amulet.iter().map(|id| (*id, true)))
        {
            let request = PlayerDeathItemRequest::selected(death, item, burn)
                .map_err(|_| CharacterProgressionError::InvalidInput)?;
            let result = if burn {
                self.root
                    .commit_player_death_amulet(
                        permit,
                        self.character,
                        self.holder,
                        fence.item_fence(),
                        PlayerDeathAmuletCause {
                            death_occurrence: *death.occurrence.as_bytes(),
                            item_instance: item,
                        },
                        request,
                    )
                    .await
            } else {
                self.root
                    .commit_player_death_drop(
                        permit,
                        self.character,
                        self.holder,
                        fence.item_fence(),
                        PlayerDeathDropCause {
                            death_occurrence: *death.occurrence.as_bytes(),
                            item_instance: item,
                        },
                        request,
                    )
                    .await
            };
            result.map_err(|error| {
                super::operator_event(&format!("player_death_item_workflow_failed reason={error}"));
                // Keep the immutable receipt and pending respawn. A retry resumes
                // the same causes; a terminal rejection is never skipped.
                CharacterProgressionError::Unavailable(
                    crate::durability::DurabilityError::Unavailable,
                )
            })?;
        }
        Ok(())
    }
}
