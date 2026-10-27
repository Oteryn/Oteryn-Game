//! Bounded, current-session-fenced read of the shared cooldown owner.
use super::super::admission_authority_guards::AdmissionGuardStore;
use super::super::character_authority::ReconciledCharacterAuthority;
use super::super::character_progression::assert_item_only_fence;
use super::super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::super::item_transfer::CurrentCharacterItemFence;
use super::*;
use crate::foundation::admission_authority_publication::AdmissionAuthorityGuardKeyV1;
use oteryn_protocol_oteryn::boss_encounter::{
    BossCooldownEntry, COOLDOWN_PAGE_SIZE, decode_cooldown_query, encode_cooldown_list,
};

impl super::super::DurabilityRoot {
    /// Read only this admitted Character's page under retained session identity use,
    /// current recovery, session, lease,
    /// assignment, node and live Character-root interpretation proofs. This is an item-only
    /// read: unrelated CharacterRevision advances do not invalidate it. The semantic-pass
    /// deadline bounds every database wait.
    pub async fn read_boss_cooldown_list(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        query: &[u8],
    ) -> std::result::Result<Option<Vec<u8>>, DurabilityError> {
        let Ok(after) = decode_cooldown_query(query) else {
            return Ok(None);
        };
        let after = after.to_owned();
        let recovery = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let guards = AdmissionGuardStore::from_root(self.clone());
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    // A materialized live session is insufficient after a partial
                    // restore loses issuance history. Prove retained identity use
                    // before the independent participant/root/runtime checks.
                    if !super::super::fresh_admission::committed_session_use(
                        &mut tx,
                        crate::foundation::CharacterId::decode(fence.character_id.as_bytes())
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        fence.game_session_id,
                    )
                    .await?
                    {
                        return Ok(None);
                    }
                    // A ready mirror and assignment history do not prove the
                    // readiness publisher's retained decision. Reuse its owner
                    // validator under the same relation locks and deadline.
                    let runtime_key = AdmissionAuthorityGuardKeyV1::Runtime(fence.runtime_scope);
                    match guards.load_locked(&mut tx, &runtime_key).await {
                        Ok(Some(_)) => {}
                        Ok(None) | Err(DurabilityError::InvalidStoredState) => return Ok(None),
                        Err(error) => return Err(error),
                    }
                    if assert_item_only_fence(&mut tx, &fence, &node)
                        .await?
                        .is_err()
                    {
                        return Ok(None);
                    }
                    // C collation is also the primary-key collation, matching Rust key ordering.
                    // One extra row establishes continuation without a count or an unbounded scan.
                    let rows: Vec<(String, i64)> = sqlx::query_as(
                        "SELECT boss_key,available_at FROM game_character_boss_cooldowns \
                 WHERE character_id=encode($1,'hex')::uuid AND boss_key > $2 COLLATE \"C\" \
                 ORDER BY boss_key LIMIT 65",
                    )
                    .bind(fence.character_id.as_bytes().as_slice())
                    .bind(after)
                    .fetch_all(&mut *tx)
                    .await?;
                    let now: i64 = sqlx::query_scalar(
                        "SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint",
                    )
                    .fetch_one(&mut *tx)
                    .await?;
                    let more = rows.len() > COOLDOWN_PAGE_SIZE;
                    let entries = rows
                        .into_iter()
                        .take(COOLDOWN_PAGE_SIZE)
                        .map(|(boss_key, at)| {
                            Ok(BossCooldownEntry {
                                boss_key,
                                available_at_unix_ms: u64::try_from(at)
                                    .map_err(|_| DurabilityError::InvalidStoredState)?,
                            })
                        })
                        .collect::<std::result::Result<Vec<_>, DurabilityError>>()?;
                    let payload = encode_cooldown_list(
                        u64::try_from(now).map_err(|_| DurabilityError::InvalidStoredState)?,
                        &entries,
                        more,
                    )
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Some(payload))
                })
            })
            .await
    }
}
