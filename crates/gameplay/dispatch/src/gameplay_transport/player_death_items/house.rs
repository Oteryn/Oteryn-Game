//! Receipt-selected House death items; ordinary selection is unchanged.
use super::*;

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(in crate::gameplay_transport) async fn apply_house_player_death_items(
        &self,
        _permit: &SpellLanePermit,
        house: &crate::durability::house_scope_handoff::HouseId,
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
                    .commit_house_player_death_amulet(
                        self.character,
                        self.holder,
                        house,
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
                    .commit_house_player_death_drop(
                        self.character,
                        self.holder,
                        house,
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
                crate::gameplay_transport::operator_event(&format!(
                    "player_death_item_workflow_failed reason={error}"
                ));
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
