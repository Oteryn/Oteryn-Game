//! Read the latest immutable death outcome under the current admitted session fence.
use crate::durability::db::commit_semantic_read;
use super::*;
use crate::durability::item_transfer::CurrentCharacterItemFence;
use oteryn_protocol_oteryn::death_protection::FinalBlowKind;
impl DurabilityRoot {
    /// The unique (Character, committed revision) index bounds this to one receipt.
    /// Reconnect reads the same occurrence even after pending respawn was consumed.
    pub async fn read_latest_character_death(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
    ) -> Result<Option<(CommittedCharacterDeath, Option<bool>, FinalBlowKind)>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if super::super::character_progression::assert_session_fence(&mut tx, &fence, &node).await?.is_err() {
                return Ok(Err(CharacterProgressionError::AuthorityRejected));
            }
            let occurrence: Option<(String, Option<bool>, i16)> = sqlx::query_as(
                "SELECT d.death_occurrence_id::text, c.pvp, COALESCE(c.final_blow_kind, 0::smallint)
                 FROM game_character_death_receipts d
                 LEFT JOIN game_public_death_classifications c USING (death_occurrence_id)
                 WHERE d.character_id=encode($1,'hex')::uuid
                 ORDER BY d.committed_character_revision DESC LIMIT 1"
            )
                .bind(fence.character_id.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let death = match occurrence {
                Some((occurrence, pvp, kind)) => {
                    let occurrence = PlayerDeathOccurrence::from_bytes(uuid_text(&occurrence)?).map_err(|_| DurabilityError::InvalidStoredState)?;
                    let kind = u8::try_from(kind).ok().and_then(FinalBlowKind::decode)
                        .ok_or(DurabilityError::InvalidStoredState)?;
                    load_receipt(&mut tx, occurrence).await?.as_ref().map(decode_receipt).transpose()?
                        .map(|receipt| (receipt, pvp, kind))
                },
                None => None,
            };
            commit_semantic_read(tx, deadline).await?;
            Ok(Ok(death))
        })).await?
    }
}
