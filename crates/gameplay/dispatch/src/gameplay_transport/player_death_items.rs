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
use sha2::{Digest, Sha256};

impl ComposedFreshAdmission<'_, '_, '_> {
    pub(super) async fn resume_pending_player_death_items(
        &self,
        permit: &SpellLanePermit,
        fence: CurrentCharacterGameplayFence,
    ) -> Result<(), CharacterProgressionError> {
        if let Some(pending) = self
            .root
            .pending_respawn(self.character, self.holder, fence)
            .await?
        {
            self.resume_player_death_items(permit, fence, pending.occurrence)
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
            let request = PlayerDeathItemRequest {
                transaction_id: operation_id(death.occurrence, item, b"transaction"),
                event_id: operation_id(death.occurrence, item, b"event"),
                occurred_at_unix_ms: items.occurred_at,
                content_revision: items.content_revision.clone(),
                ruleset_revision: items.ruleset_revision.clone(),
                sim_revision: items.simulation_revision.clone(),
                server_build_id: "player-death-items-v1".into(),
            };
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

/// Stable UUIDv7 identities survive retries and reconnects. The occurrence supplies
/// the timestamp; independently domain-separated hashes supply the remaining bits.
fn operation_id(occurrence: PlayerDeathOccurrence, item: [u8; 16], purpose: &[u8]) -> [u8; 16] {
    let mut hash = Sha256::new();
    hash.update(b"oteryn.death.item_operation.v1");
    hash.update(occurrence.as_bytes());
    hash.update(item);
    hash.update(purpose);
    let digest = hash.finalize();
    let mut id = *occurrence.as_bytes();
    id[6..].copy_from_slice(&digest[..10]);
    id[6] = (id[6] & 0x0f) | 0x70;
    id[8] = (id[8] & 0x3f) | 0x80;
    id
}
