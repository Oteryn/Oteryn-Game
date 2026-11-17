//! N009: the admitted Character's own sanction record from Game's read-only projection of
//! Platform-owned decisions (migration 2800, `PLATFORM_SANCTION_LOYALTY_EXPORT_V1`). The read
//! is fenced like any session read and never writes, enforces or derives a sanction.
use super::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use super::character_progression::assert_session_fence;
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::item_transfer::CurrentCharacterItemFence;
use super::runtime_scope_assignment::NodeIncarnationProof;
use super::{DurabilityError, DurabilityRoot};
use oteryn_protocol_oteryn::sanction_record as wire;
use sqlx::Row;

/// Newest first; one row past the wire bound sets `more`.
const OWN_RECORDS_SQL: &str = "SELECT kind,character_id IS NOT NULL AS character_scope,revoked,reason_code,starts_at_ms,ends_at_ms,reason_text FROM game_account_sanction_projections WHERE account_id=$1::uuid AND (character_id IS NULL OR character_id=encode($2,'hex')::uuid) ORDER BY starts_at_ms DESC,sanction_stream_id DESC LIMIT $3";

/// One stored projection row, as read.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StoredSanction {
    pub kind: i16,
    pub character_scope: bool,
    pub revoked: bool,
    pub reason_code: i32,
    pub starts_at_ms: i64,
    pub ends_at_ms: Option<i64>,
    pub reason_text: String,
}

/// The encoded reply for at most `MAX_RECORDS + 1` newest-first rows, or `None` when a stored
/// row violates the wire shape (the table checks make that a store fault).
pub fn encode_own_record(rows: Vec<StoredSanction>) -> Option<Vec<u8>> {
    let more = rows.len() > wire::MAX_RECORDS;
    let entries = rows
        .into_iter()
        .take(wire::MAX_RECORDS)
        .map(|row| {
            Some(wire::SanctionEntry {
                kind: wire::SanctionKind::from_code(u8::try_from(row.kind).ok()?)?,
                character_scope: row.character_scope,
                revoked: row.revoked,
                reason_code: u16::try_from(row.reason_code).ok()?,
                starts_at_ms: row.starts_at_ms,
                ends_at_ms: row.ends_at_ms,
                reason: row.reason_text,
            })
        })
        .collect::<Option<Vec<_>>>()?;
    wire::encode_reply(&wire::SanctionRecord { more, entries })
}

impl DurabilityRoot {
    /// `Ok(None)` when the session fence is no longer current.
    pub async fn read_own_sanction_record(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
    ) -> Result<Option<Vec<u8>>, DurabilityError> {
        let record = authority
            .record_for(self)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        let limit = i64::try_from(wire::MAX_RECORDS + 1)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &record).await?;
                    lock_admission_relations(&mut tx).await?;
                    let Ok(account) = assert_session_fence(&mut tx, &fence, &node).await? else {
                        return Ok(None);
                    };
                    let rows = sqlx::query(OWN_RECORDS_SQL)
                        .bind(&account)
                        .bind(fence.character_id.as_bytes().as_slice())
                        .bind(limit)
                        .fetch_all(&mut *tx)
                        .await?
                        .into_iter()
                        .map(|row| {
                            Ok(StoredSanction {
                                kind: row.try_get("kind")?,
                                character_scope: row.try_get("character_scope")?,
                                revoked: row.try_get("revoked")?,
                                reason_code: row.try_get("reason_code")?,
                                starts_at_ms: row.try_get("starts_at_ms")?,
                                ends_at_ms: row.try_get("ends_at_ms")?,
                                reason_text: row.try_get("reason_text")?,
                            })
                        })
                        .collect::<Result<Vec<_>, sqlx::Error>>()?;
                    let bytes =
                        encode_own_record(rows).ok_or(DurabilityError::InvalidStoredState)?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Some(bytes))
                })
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn stored(kind: i16, starts: i64, ends: Option<i64>) -> StoredSanction {
        StoredSanction {
            kind,
            character_scope: kind != 4,
            revoked: false,
            reason_code: 3,
            starts_at_ms: starts,
            ends_at_ms: ends,
            reason_text: "Spamming".into(),
        }
    }

    #[test]
    fn own_record_marks_more_past_the_bound_and_refuses_corrupt_rows() -> Result<(), &'static str> {
        let rows: Vec<_> = (0..=wire::MAX_RECORDS)
            .map(|index| stored(1, 1_000 - i64::try_from(index).unwrap_or(0), Some(2_000)))
            .collect();
        let bytes = encode_own_record(rows).ok_or("encode")?;
        let decoded = wire::decode_reply(&bytes).ok_or("decode")?;
        assert!(decoded.more);
        assert_eq!(decoded.entries.len(), wire::MAX_RECORDS);
        assert_eq!(decoded.entries[0].starts_at_ms, 1_000);
        let empty = encode_own_record(Vec::new()).ok_or("empty")?;
        assert_eq!(empty, [0, 0]);
        assert!(encode_own_record(vec![stored(7, 1, None)]).is_none());
        assert!(encode_own_record(vec![stored(4, 1, None), stored(1, 2, Some(3))]).is_none());
        let mut wide = stored(3, 1, None);
        wide.reason_code = 70_000;
        assert!(encode_own_record(vec![wide]).is_none());
        Ok(())
    }
}
