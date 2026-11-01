//! The same one-item death writers after the departed Instance owner is proven lost.
//! The activity owner seals and revalidates that proof; this module owns no session
//! handoff and never substitutes a Channel for the original death Ground.
use super::*;
use crate::durability::activity_handoff::{
    DepartedActivityDeath, validate_departed_activity_death_in_transaction,
};
use crate::durability::character_death::PlayerDeathOccurrence;
use crate::durability::character_progression::CharacterProgressionError;

impl DurabilityRoot {
    /// Finish only the immutable drop/AoL selection of this exact departed death.
    /// Each original item cause retains its existing independent atomic Type2
    /// receipt and audit. A failure leaves the pending respawn and remaining items
    /// untouched, for the same owner to resume before it commits a return.
    pub async fn resume_departed_activity_death_items(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        proof: &DepartedActivityDeath,
    ) -> Result<()> {
        let source = proof.source();
        if !matches!(source.runtime_scope, RuntimeScopeRefV1::Instance { .. }) {
            return Err(PlayerDeathItemError::AuthorityRejected);
        }
        permit.check_scope(source.runtime_scope)?;
        let occurrence = PlayerDeathOccurrence::from_bytes(proof.occurrence())
            .map_err(|_| PlayerDeathItemError::InvalidInput)?;
        let death = self
            .reconcile_character_death(authority, occurrence)
            .await
            .map_err(|error| match error {
                CharacterProgressionError::Unavailable(error) => {
                    PlayerDeathItemError::Unavailable(error)
                }
                _ => PlayerDeathItemError::StaleState,
            })?
            .ok_or(PlayerDeathItemError::StaleState)?;
        if death.character_id != source.character_id
            || death.death_cell.runtime_scope != source.runtime_scope
        {
            return Err(PlayerDeathItemError::AuthorityRejected);
        }
        let Some(items) = &death.items else {
            // Historical no-selection receipts have no loss obligation. The
            // final activity recovery still revalidates proof and completion.
            return Ok(());
        };
        for (item, burn) in items
            .lost
            .iter()
            .map(|item| (*item, false))
            .chain(items.amulet.iter().map(|item| (*item, true)))
        {
            let request = PlayerDeathItemRequest::selected(&death, item, burn)?;
            let cause = PlayerDeathDropCause {
                death_occurrence: proof.occurrence(),
                item_instance: item,
            };
            self.commit_departed_death_item(permit, authority, node, proof, cause, request, burn)
                .await?;
        }
        Ok(())
    }

    #[allow(
        clippy::too_many_arguments,
        reason = "existing one-item writer plus the sealed departed-owner proof"
    )]
    async fn commit_departed_death_item(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        proof: &DepartedActivityDeath,
        cause: PlayerDeathDropCause,
        request: PlayerDeathItemRequest,
        burn: bool,
    ) -> Result<PlayerDeathItemOutcome> {
        let source = proof.source();
        permit.check_scope(source.runtime_scope)?;
        request.validate(&cause.death_occurrence, &cause.item_instance)?;
        if cause.death_occurrence != proof.occurrence() {
            return Err(PlayerDeathItemError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| PlayerDeathItemError::AuthorityRejected)?;
        let root = self.clone();
        let node = node.clone();
        let proof = proof.clone();
        let mut context = permit;
        self.try_issue_semantic_pass()?.run_with_context(&mut context, move |holder, deadline, context| {
            Box::pin(async move {
                let permit = *context;
                let mut tx = begin_type2_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                // Historical evidence cannot authorize new mutation. This validates
                // independently lost source ownership, retained full session history,
                // current origin ownership, and exact death/pending scope and generation.
                if !validate_departed_activity_death_in_transaction(&mut tx, &root, &node, &proof).await? {
                    return Ok(Err(PlayerDeathItemError::AuthorityRejected));
                }
                match replay(&mut tx, &source, cause, &request, burn).await {
                    Ok(Some(outcome)) => { tx.commit(deadline).await?; return Ok(Ok(outcome)); }
                    Ok(None) => {},
                    Err(PlayerDeathItemError::Unavailable(error)) => return Err(error),
                    Err(error) => return Ok(Err(error)),
                }
                // The same Character root lock serializes custody and content with
                // live death operations. Its revision is neither read nor changed.
                let stored = sqlx::query("SELECT content_revision,ruleset_revision FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
                    .bind(source.character_id.as_bytes().as_slice()).bind(source.runtime_scope.world_id().as_bytes().as_slice())
                    .fetch_optional(&mut **tx).await?;
                let Some(stored) = stored else { return Ok(Err(PlayerDeathItemError::AuthorityRejected)); };
                if stored.try_get::<String, _>("content_revision")? != request.content_revision
                    || stored.try_get::<String, _>("ruleset_revision")? != request.ruleset_revision {
                    return Ok(Err(PlayerDeathItemError::StaleState));
                }
                let tuple = tx.tuple();
                match write(&mut tx, permit, &source, cause, &request, burn, tuple).await {
                    Ok(outcome) => { tx.commit(deadline).await?; Ok(Ok(outcome)) }
                    Err(PlayerDeathItemError::Unavailable(error)) => Err(error),
                    Err(error) => Ok(Err(error)),
                }
            })
        }).await?
    }
}

#[cfg(test)]
mod tests {
    use super::super::selected_operation_id;

    #[test]
    fn recovered_death_item_ids_preserve_the_original_dispatch_bytes() {
        let mut occurrence = [1; 16];
        occurrence[6] = 0x71;
        occurrence[8] = 0x81;
        let mut item = [2; 16];
        item[6] = 0x72;
        item[8] = 0x82;
        // Captured from the original DEATH-3 dispatch algorithm before moving
        // its sole implementation into the common live/recovery request owner.
        assert_eq!(
            selected_operation_id(&occurrence, item, b"transaction"),
            [
                1, 1, 1, 1, 1, 1, 126, 138, 186, 84, 75, 187, 141, 108, 64, 2
            ]
        );
        assert_eq!(
            selected_operation_id(&occurrence, item, b"event"),
            [
                1, 1, 1, 1, 1, 1, 116, 197, 155, 107, 179, 71, 138, 236, 4, 22
            ]
        );
    }
}
