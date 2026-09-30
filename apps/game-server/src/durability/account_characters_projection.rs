//! Character Authority side of `ListCharactersForAccount` (migration 0024).
//!
//! Every read runs under the reconciled Character authority, so a restored or
//! unreconciled store never publishes.

use super::character_authority::{
    CharacterAuthorityError, ReconciledCharacterAuthority, assert_recovery_fence,
};
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::{DB_PASS_DEADLINE, DurabilityRoot};
use oteryn_game_server::native_admission_source::account_characters::{
    AccountSnapshot, Availability, CharacterSummary, MAX_CHARACTERS, ProjectionStore,
    WatermarkFacts,
};
use sqlx::Row;
use std::time::Duration;

type Result<T> = std::result::Result<T, CharacterAuthorityError>;

/// Maximum Character transaction duration (§5.1): every semantic transaction
/// runs under a PostgreSQL `transaction_timeout` of at most `DB_PASS_DEADLINE`;
/// one second more covers the commit and clock granularity.
pub const MAX_CHARACTER_TRANSACTION: Duration = Duration::from_secs(DB_PASS_DEADLINE.as_secs() + 1);

// One statement is one database snapshot: the epoch, the account's revision and
// its Characters are read together. One row more than the wire bound is read so
// an oversized list refuses instead of being truncated; every Character is
// listed, so the bound counts exactly the entries sent. `observed_at` is when
// the revision was assigned (migration 0028), never the read time, so a retry
// of the same (epoch, revision) is byte-identical.
const SNAPSHOT: &str = "\
WITH head AS (SELECT account_id FROM game_character_account_projection_outbox \
              ORDER BY created_at, account_id LIMIT 1) \
SELECT h.account_id::text AS account_id, e.projection_epoch, p.projection_revision, \
       p.revised_at / 1000 AS observed_at, \
       r.character_id::text AS character_id, r.world_id::text AS world_id, r.name, r.lifecycle \
  FROM head h \
  JOIN game_character_account_projections p USING (account_id) \
 CROSS JOIN game_character_account_projection_epoch e \
  LEFT JOIN LATERAL (SELECT character_id, world_id, name, lifecycle FROM game_character_roots \
                      WHERE account_id = h.account_id ORDER BY character_id LIMIT $1) r ON true \
 ORDER BY r.character_id";

/// `AVAILABLE` for the only admissible lifecycle; any other lifecycle is not
/// admissible, so it is listed as `UNAVAILABLE` (§2.1). A deleted Character has
/// no root and is not listed.
fn availability(lifecycle: i16) -> Availability {
    if lifecycle == 1 {
        Availability::Available
    } else {
        Availability::Unavailable
    }
}

fn positive(value: i64) -> Result<u64> {
    u64::try_from(value)
        .ok()
        .filter(|v| *v > 0)
        .ok_or(CharacterAuthorityError::Rejected)
}

impl DurabilityRoot {
    /// The current snapshot of the account with the oldest undelivered change.
    pub async fn next_account_characters_snapshot(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
    ) -> Result<Option<AccountSnapshot>> {
        let recovery = authority.record_for(self)?;
        let limit = i64::try_from(MAX_CHARACTERS + 1).unwrap_or(i64::MAX);
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let rows = sqlx::query(SNAPSHOT)
                        .bind(limit)
                        .fetch_all(&mut *tx)
                        .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    let Some(first) = rows.first() else {
                        return Ok(Ok(None));
                    };
                    let mut snapshot = AccountSnapshot {
                        account_id: first.try_get("account_id")?,
                        projection_epoch: match positive(first.try_get("projection_epoch")?) {
                            Ok(v) => v,
                            Err(e) => return Ok(Err(e)),
                        },
                        projection_revision: match positive(first.try_get("projection_revision")?) {
                            Ok(v) => v,
                            Err(e) => return Ok(Err(e)),
                        },
                        source_observed_at: first.try_get("observed_at")?,
                        characters: Vec::with_capacity(rows.len()),
                    };
                    for row in &rows {
                        let Some(character_id) =
                            row.try_get::<Option<String>, _>("character_id")?
                        else {
                            continue;
                        };
                        snapshot.characters.push(CharacterSummary {
                            character_id,
                            world_id: row.try_get("world_id")?,
                            name: row.try_get("name")?,
                            availability: availability(row.try_get("lifecycle")?),
                        });
                    }
                    Ok(Ok(Some(snapshot)))
                })
            })
            .await?
    }

    /// Clear the account's outbox rows the acknowledged snapshot covers: every
    /// row of that or an earlier epoch up to the sent revision. The revision is
    /// per account and never reused, so a row it covers is in the snapshot; a
    /// resync row of a later epoch stays until that epoch's snapshot arrives.
    pub async fn clear_account_characters(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        account_id: &str,
        projection_epoch: u64,
        projection_revision: u64,
    ) -> Result<()> {
        let recovery = authority.record_for(self)?;
        let (Ok(epoch), Ok(revision)) = (
            i64::try_from(projection_epoch),
            i64::try_from(projection_revision),
        ) else {
            return Err(CharacterAuthorityError::Rejected);
        };
        let account = account_id.to_owned();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            sqlx::query("DELETE FROM game_character_account_projection_outbox WHERE account_id = $1::uuid AND projection_epoch <= $2 AND projection_revision <= $3")
                .bind(account).bind(epoch).bind(revision).execute(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(()))
        })).await?
    }

    /// The §5.1 watermark facts on the database clock.
    pub async fn account_characters_watermark_facts(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
    ) -> Result<WatermarkFacts> {
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let row = sqlx::query("SELECT e.projection_epoch, (SELECT min(created_at) FROM game_character_account_projection_outbox) AS oldest, floor(extract(epoch FROM clock_timestamp()) * 1000)::bigint AS now_ms FROM game_character_account_projection_epoch e WHERE e.epoch_scope = 1")
                .fetch_one(&mut *tx).await?;
            commit_semantic_transaction(tx, deadline).await?;
            let projection_epoch = match positive(row.try_get("projection_epoch")?) {
                Ok(v) => v,
                Err(e) => return Ok(Err(e)),
            };
            Ok(Ok(WatermarkFacts {
                projection_epoch,
                oldest_undelivered_ms: row.try_get("oldest")?,
                now_ms: row.try_get("now_ms")?,
            }))
        })).await?
    }
}

/// The publisher's store over one reconciled Character authority.
pub struct AccountCharactersStore<'r, 'a, 'f, 's> {
    pub root: &'r DurabilityRoot,
    pub authority: &'a ReconciledCharacterAuthority<'f, 's>,
}

impl ProjectionStore for AccountCharactersStore<'_, '_, '_, '_> {
    async fn next_snapshot(&mut self) -> std::result::Result<Option<AccountSnapshot>, ()> {
        self.root
            .next_account_characters_snapshot(self.authority)
            .await
            .map_err(|_| ())
    }
    async fn clear(
        &mut self,
        account_id: &str,
        projection_epoch: u64,
        projection_revision: u64,
    ) -> std::result::Result<(), ()> {
        self.root
            .clear_account_characters(
                self.authority,
                account_id,
                projection_epoch,
                projection_revision,
            )
            .await
            .map_err(|_| ())
    }
    async fn watermark_facts(&mut self) -> std::result::Result<WatermarkFacts, ()> {
        self.root
            .account_characters_watermark_facts(self.authority)
            .await
            .map_err(|_| ())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_lifecycle_is_listed_and_only_lifecycle_one_is_available() {
        assert_eq!(availability(1), Availability::Available);
        for lifecycle in [i16::MIN, -1, 0, 2, 3, i16::MAX] {
            assert_eq!(availability(lifecycle), Availability::Unavailable);
        }
        assert!(SNAPSHOT.contains("p.revised_at / 1000 AS observed_at"));
        assert!(!SNAPSHOT.contains("clock_timestamp"));
    }
}
