//! Fenced snapshots of the existing Wheel's Gift of Life cooldown.
//! No CharacterRevision is minted: this is a runtime timer snapshot, like item-owner state.
use super::*;
use super::super::item_transfer::CurrentCharacterItemFence;

const MAX_REMAINING_MICROS: u64 = 30 * 60 * 60 * 1_000_000;

impl DurabilityRoot {
    pub async fn read_character_wheel_gift(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
    ) -> Result<u64> {
        let recovery = authority.record_for(self).map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        Ok(self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                let remaining: Option<i64> = sqlx::query_scalar(
                    "SELECT remaining_micros FROM game_character_wheel_gift_state WHERE character_id=encode($1,'hex')::uuid")
                    .bind(character.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                let remaining = u64::try_from(remaining.unwrap_or(0)).map_err(|_| DurabilityError::InvalidStoredState)?;
                if remaining > MAX_REMAINING_MICROS { return Err(DurabilityError::InvalidStoredState); }
                commit_semantic_transaction(tx, deadline).await?;
                Ok(remaining)
            })
        }).await?)
    }

    /// Receipt first: an identical retry returns its retained answer even after a session ends.
    pub async fn save_character_wheel_gift(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        occurrence: WheelOccurrence,
        remaining_micros: u64,
    ) -> Result<()> {
        if remaining_micros > MAX_REMAINING_MICROS || fence.character_lease_generation == 0 {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority.record_for(self).map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let mut hash = Sha256::new();
        hash.update(b"wheel-gift-snapshot-v1");
        hash.update(fence.character_id.as_bytes());
        hash.update(remaining_micros.to_be_bytes());
        let binding: [u8; 32] = hash.finalize().into();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:wheel-gift:' || encode($1,'hex'),0))")
                    .bind(occurrence.as_bytes().as_slice()).execute(&mut *tx).await?;
                let recorded: Option<Vec<u8>> = sqlx::query_scalar(
                    "SELECT request_binding FROM game_character_wheel_gift_receipts WHERE character_id=encode($1,'hex')::uuid AND occurrence=encode($2,'hex')::uuid")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(occurrence.as_bytes().as_slice())
                    .fetch_optional(&mut *tx).await?;
                if let Some(recorded) = recorded {
                    if recorded != binding { return Ok(Err(CharacterProgressionError::ConflictingOccurrence)); }
                    commit_semantic_transaction(tx, deadline).await?;
                    return Ok(Ok(()));
                }
                if let Err(error) = super::super::character_progression::assert_item_only_fence(&mut tx, &fence, &node).await? {
                    return Ok(Err(error));
                }
                let previous: Option<i64> = sqlx::query_scalar(
                    "SELECT revision FROM game_character_wheel_gift_state WHERE character_id=encode($1,'hex')::uuid FOR UPDATE")
                    .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                let before = previous.unwrap_or(0);
                let after = before.checked_add(1).ok_or(DurabilityError::InvalidStoredState)?;
                let remaining = i64::try_from(remaining_micros).map_err(|_| DurabilityError::InvalidStoredState)?;
                sqlx::query("INSERT INTO game_character_wheel_gift_receipts(character_id,occurrence,request_binding,before_revision,after_revision,remaining_micros) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,$4,$5,$6)")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(occurrence.as_bytes().as_slice())
                    .bind(binding.as_slice()).bind(before).bind(after).bind(remaining).execute(&mut *tx).await?;
                sqlx::query("INSERT INTO game_character_wheel_gift_state(character_id,revision,remaining_micros,occurrence) VALUES(encode($1,'hex')::uuid,$2,$3,encode($4,'hex')::uuid) ON CONFLICT(character_id) DO UPDATE SET revision=excluded.revision,remaining_micros=excluded.remaining_micros,occurrence=excluded.occurrence")
                    .bind(fence.character_id.as_bytes().as_slice()).bind(after).bind(remaining)
                    .bind(occurrence.as_bytes().as_slice()).execute(&mut *tx).await?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(Ok(()))
            })
        }).await?
    }
}
