//! PREY-DIFF: one receipt-fenced measurement descendant of the existing kill queue.
use super::*;
use crate::combat::{CombatDeathRewardLootError, CommittedCorpseLoot};
use crate::durability::character_progression::CurrentCharacterGameplayFence;
use crate::durability::character_revision_sequencer::RevisionSlot;
use crate::gameplay_transport::kill_reward::PendingKillSettlement;
use sha2::{Digest, Sha256};

impl ComposedFreshAdmission<'_, '_, '_> {
    /// Called while the kill owner still holds its spell lane and Character revision
    /// slot. Every Prey selection drains that queue first, so a delayed receipt
    /// cannot be charged to a successor selection.
    pub(in crate::gameplay_transport) async fn credit_prey_kill(
        &self,
        entry: &PendingKillSettlement,
        loot: Option<&Result<CommittedCorpseLoot, CombatDeathRewardLootError>>,
        revision: &mut RevisionSlot,
        fence: CurrentCharacterGameplayFence,
    ) -> bool {
        let Some(mut checkpoint) = entry.prey_checkpoint.clone() else { return true; };
        let Some(catalogue) = self.prey_catalogue() else { return false; };
        let corpse = match loot {
            Some(Ok(committed)) => Some(&committed.corpse),
            Some(Err(CombatDeathRewardLootError::Mint(_, corpse))) => Some(corpse),
            Some(Err(_)) | None => None,
        };
        checkpoint.loot_gold = if let Some(corpse) = corpse {
            match self.root.read_prey_committed_loot_value(
                self.character, fence.character_id, corpse, &catalogue.rules().npc_sell_prices
            ).await {
                Ok(value) => value,
                Err(_) => return false,
            }
        } else { 0 };
        let Ok(expected_character_revision) = revision.cursor(self.root, self.character).await else {
            return false;
        };
        let fence = CurrentCharacterGameplayFence { expected_character_revision, ..fence };
        let outcome = self.root.commit_prey_command(
            self.character,
            self.holder,
            fence,
            kill_occurrence(entry.facts.occurrence),
            PreyMutation::Checkpoint(vec![checkpoint]),
            &catalogue,
            None,
            &Premium(self.premium_coordinator),
        ).await;
        revision.observe_loop_training();
        outcome.is_ok_and(|(reply, _)| reply.status == 0)
    }
}

fn kill_occurrence(death: [u8; 16]) -> [u8; 16] {
    let mut digest = Sha256::new();
    digest.update(b"oteryn:prey:kill:v1\0");
    digest.update(death);
    let mut occurrence = [0; 16];
    occurrence.copy_from_slice(&digest.finalize()[..16]);
    occurrence[6] = (occurrence[6] & 15) | 0x70;
    occurrence[8] = (occurrence[8] & 63) | 0x80;
    occurrence
}

#[cfg(test)]
mod tests {
    #[test]
    fn prey_kill_measurement_has_stable_separate_receipt_identity() {
        let first = super::kill_occurrence([1; 16]);
        assert_eq!(first, super::kill_occurrence([1; 16]));
        assert_ne!(first, super::kill_occurrence([2; 16]));
        assert_ne!(first, [1; 16]);
        assert_eq!(first[6] >> 4, 7);
    }
}
