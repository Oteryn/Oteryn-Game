//! P08 Proficiency Catalysts: one unit burnt by the item-use writer and one `catalyst` receipt
//! on the target weapon's proficiency track, in one fenced transaction that advances the
//! CharacterRevision. The receipt's occurrence is the burn's TransactionId; migration 0564
//! checks the pair, its single line and the progress bound at commit.
//! Crystal `zimbadev/crystalserver` `main`
//! `data/scripts/actions/items/proficiency_catalysts.lua:1-34`; English TibiaWiki
//! `Proficiency_Catalyst`, `Greater_Proficiency_Catalyst` and `Lesser_Proficiency_Catalyst`
//! (read 2026-10-30).
use super::*;
use crate::durability::character_proficiency::{
    ProficiencyCause, ProficiencyChangeRequest, ProficiencyCommitOutcome, ProficiencyDefinitions,
    ProficiencyWriteRefusal, apply_change, replay_change,
};
use crate::durability::character_progression::CurrentCharacterGameplayFence;

pub const CATALYST_CAUSE: u32 = 494;

/// The progress one catalyst adds, before the track's final-progress cap.
pub fn catalyst_progress(production_key: &str) -> Option<u32> {
    match production_key {
        // Lesser Proficiency Catalyst: Global only, absent from Crystal's script.
        "oteryn:item.tibia.i54266" => Some(5_000),
        "oteryn:item.tibia.i51588" => Some(25_000),
        "oteryn:item.tibia.i51589" => Some(100_000),
        _ => None,
    }
}

impl DurabilityRoot {
    /// One catalyst use. An exact replay of the occurrence answers from its receipt; every
    /// refusal writes nothing. Runtime callers reach it only through a
    /// [`RevisionSlot`](crate::durability::character_revision_sequencer::RevisionSlot).
    pub async fn commit_proficiency_catalyst(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        item: ItemUseRequest,
        change: ProficiencyChangeRequest,
        policy: std::sync::Arc<dyn ProficiencyDefinitions>,
    ) -> std::result::Result<ProficiencyCommitOutcome, CharacterProgressionError> {
        let [line] = change.lines() else {
            return Err(CharacterProgressionError::InvalidInput);
        };
        let gained = line.after().progress().saturating_sub(line.before().progress());
        if item.validate().is_err()
            || item.kind != ItemUseKind::ProficiencyCatalyst
            || change.cause() != ProficiencyCause::Catalyst
            || item.transaction_id != *change.occurrence().as_bytes()
            || catalyst_progress(&item.source_definition.production_key)
                .is_none_or(|amount| gained > u64::from(amount))
            || fence.character_lease_generation == 0
            || !matches!(fence.runtime_scope, RuntimeScopeRefV1::Channel { .. })
        {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let binding = change.command_binding(fence.character_id, fence.expected_character_revision);
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_type2_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if let Some(replayed) =
                        replay_change(&mut tx, &change, &binding, policy.as_ref()).await?
                    {
                        if matches!(replayed, Ok(ProficiencyCommitOutcome::AlreadyCommitted(_))) {
                            tx.commit(deadline).await?;
                        }
                        return Ok(replayed);
                    }
                    let outcome =
                        apply_change(&mut tx, &fence, &node, &change, policy.as_ref(), &binding)
                            .await?;
                    if !matches!(outcome, Ok(ProficiencyCommitOutcome::Committed(_))) {
                        return Ok(outcome);
                    }
                    let item_fence = fence.item_fence();
                    match assert_item_only_fence(&mut tx, &item_fence, &node).await? {
                        Ok(root)
                            if root.content_revision == item.content_revision
                                && root.ruleset_revision == item.ruleset_revision => {}
                        Ok(_) => {
                            return Ok(Ok(ProficiencyCommitOutcome::Refused(
                                ProficiencyWriteRefusal::PolicyMismatch,
                            )));
                        }
                        Err(error) => return Ok(Err(error)),
                    }
                    let tuple = tx.tuple();
                    match write(&mut tx, &item_fence, &item, tuple).await {
                        Ok(ItemUseOutcome::Written { .. }) => {}
                        Ok(ItemUseOutcome::AlreadyCommitted { .. })
                        | Err(ItemUseError::ConflictingCommand) => {
                            return Ok(Err(CharacterProgressionError::ConflictingOccurrence));
                        }
                        Err(ItemUseError::StaleState) => {
                            return Ok(Ok(ProficiencyCommitOutcome::Refused(
                                ProficiencyWriteRefusal::CatalystMoved,
                            )));
                        }
                        Err(ItemUseError::Unavailable(error)) => return Err(error),
                        Err(ItemUseError::InvalidInput) => {
                            return Ok(Err(CharacterProgressionError::InvalidInput));
                        }
                        Err(ItemUseError::AuthorityRejected) => {
                            return Ok(Err(CharacterProgressionError::AuthorityRejected));
                        }
                    }
                    tx.commit(deadline).await?;
                    Ok(outcome)
                })
            })
            .await?
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalyst_amounts_follow_global() {
        assert_eq!(catalyst_progress("oteryn:item.tibia.i54266"), Some(5_000));
        assert_eq!(catalyst_progress("oteryn:item.tibia.i51588"), Some(25_000));
        assert_eq!(catalyst_progress("oteryn:item.tibia.i51589"), Some(100_000));
        assert_eq!(catalyst_progress("oteryn:item.tibia.i3031"), None);
    }
}
