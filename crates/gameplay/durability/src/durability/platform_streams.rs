//! N009/N014: one page of a Platform stream that Game pulled (`PLATFORM_SANCTION_LOYALTY_EXPORT_V1`),
//! projected in one transaction together with its cursor. Platform owns every sanction and every
//! loyalty value; Game only mirrors them for its own reads.
//!
//! The stream's cursor row is locked first and must still hold the cursor the page was requested
//! after, so two pullers never both advance it; the loser's page rolls back as `Superseded`. Every
//! record projects through its revisioned function (migrations 2800, 2803), so a replayed or
//! reordered record never moves a projection backwards.
pub mod enforcement;
pub mod enforcement_transport;
pub mod enforcement_wire;
pub mod profile_b_wire;
mod security_revocation;

use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::{DurabilityError, DurabilityRoot};

/// A Platform stream and its cursor row in `game_platform_stream_cursors`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PlatformStream {
    Sanctions = 1,
    Loyalty = 2,
    SecurityRevocations = 3,
}

impl PlatformStream {
    fn key(self) -> i16 {
        self as i16
    }
}

/// One sanction decision revision, already checked against the export shape.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SanctionDecision {
    pub sanction_stream_id: [u8; 16],
    pub decision_revision: i64,
    pub account_id: [u8; 16],
    pub character_id: Option<[u8; 16]>,
    pub kind: i16,
    pub reason_code: i32,
    pub reason_text: String,
    pub starts_at_ms: i64,
    pub ends_at_ms: Option<i64>,
    pub revoked: bool,
}

/// One account loyalty snapshot revision.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct LoyaltySnapshot {
    pub account_id: [u8; 16],
    pub spent_premium_days: i64,
    pub loyalty_points: i64,
    pub platform_revision: i64,
}

/// One immutable Platform account-security revocation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SecurityRevocation {
    pub revocation_id: [u8; 16],
    pub account_id: [u8; 16],
    pub native_security_generation: i64,
    pub cause: i16,
    pub effective_at_ms: i64,
}
/// Owner-approved maximum successful-read age for reconnect, in milliseconds.
pub const SECURITY_REVOCATION_FRESHNESS_MS: i64 = 30_000;
pub fn security_revocation_read_fresh(
    enabled: bool,
    last_read_ms: Option<i64>,
    now_ms: i64,
) -> bool {
    !enabled
        || last_read_ms.is_some_and(|last| {
            now_ms >= last && now_ms.saturating_sub(last) <= SECURITY_REVOCATION_FRESHNESS_MS
        })
}

/// One pulled page.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum StreamPage {
    Sanctions(Vec<SanctionDecision>),
    Loyalty(Vec<LoyaltySnapshot>),
    SecurityRevocations(Vec<SecurityRevocation>),
}

impl StreamPage {
    pub fn stream(&self) -> PlatformStream {
        match self {
            Self::Sanctions(_) => PlatformStream::Sanctions,
            Self::Loyalty(_) => PlatformStream::Loyalty,
            Self::SecurityRevocations(_) => PlatformStream::SecurityRevocations,
        }
    }
}

/// What one committed page did, by record.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct PageCounts {
    pub applied: u32,
    pub replayed: u32,
    pub stale: u32,
    /// Records naming an account without a Game Character, or a Character outside the account.
    pub skipped: u32,
}

impl PageCounts {
    fn count(&mut self, outcome: i16) -> Result<(), DurabilityError> {
        let slot = match outcome {
            1 => &mut self.applied,
            2 => &mut self.replayed,
            3 => &mut self.stale,
            _ => return Err(DurabilityError::InvalidStoredState),
        };
        *slot = slot.saturating_add(1);
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PageOutcome {
    /// The records are projected and the cursor moved to the page's next cursor.
    Advanced(PageCounts),
    /// Another puller moved the cursor first; nothing of this page was kept.
    Superseded,
}

const CURSOR_SQL: &str =
    "SELECT stream_cursor FROM game_platform_stream_cursors WHERE stream=$1 FOR UPDATE";
const ADVANCE_SQL: &str = "UPDATE game_platform_stream_cursors SET stream_cursor=COALESCE($2,stream_cursor),pages=pages+1,advanced_at=clock_timestamp() WHERE stream=$1";
const SUBJECT_SQL: &str =
    "SELECT game_platform_stream_subject_known(encode($1,'hex')::uuid,encode($2,'hex')::uuid)";
const SANCTION_SQL: &str = "SELECT game_account_sanction_project(encode($1,'hex')::uuid,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6,$7,$8,$9,$10)";
const LOYALTY_SQL: &str =
    "SELECT game_account_loyalty_project_points(encode($1,'hex')::uuid,$2,$3,$4)";

async fn known(
    tx: &mut sqlx::PgConnection,
    account: &[u8; 16],
    character: Option<&[u8; 16]>,
) -> Result<bool, sqlx::Error> {
    sqlx::query_scalar(SUBJECT_SQL)
        .bind(account.as_slice())
        .bind(character.map(|id| id.as_slice()))
        .fetch_one(tx)
        .await
}

/// Inherit the immutable admission cut and captured terminal security state under the session lock.
pub(crate) async fn session_security_lineage(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    session: crate::foundation::GameSessionId,
) -> Result<(Option<u64>, bool), DurabilityError> {
    security_revocation::lineage(tx, session).await
}

/// Evaluate against the database clock after the deciding transaction has acquired its locks.
pub(crate) async fn revocation_fresh_in_transaction(
    tx: &mut sqlx::PgConnection,
) -> Result<bool, DurabilityError> {
    let (last,now):(Option<i64>,i64)=sqlx::query_as("SELECT floor(extract(epoch FROM advanced_at)*1000)::bigint,floor(extract(epoch FROM clock_timestamp())*1000)::bigint FROM game_platform_stream_cursors WHERE stream=3")
        .fetch_one(tx).await?;
    Ok(security_revocation_read_fresh(true, last, now))
}

impl DurabilityRoot {
    /// Freshness uses the same millisecond database clock as the locked controller decision.
    pub async fn security_revocation_read_fresh_now(&self) -> Result<bool, DurabilityError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let fresh = revocation_fresh_in_transaction(&mut tx).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(fresh)
                })
            })
            .await
    }
    /// A captured revocation survives stream outages and disabling the puller.
    pub async fn security_session_revoked(
        &self,
        session: crate::foundation::GameSessionId,
    ) -> Result<bool, DurabilityError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let revoked = sqlx::query_scalar(
                        "SELECT game_security_session_revoked(encode($1,'hex')::uuid)",
                    )
                    .bind(session.as_bytes().as_slice())
                    .fetch_one(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(revoked)
                })
            })
            .await
    }
    /// Last committed successful page read, including empty pages. Failed reads leave it intact.
    pub async fn security_revocation_last_read_ms(&self) -> Result<Option<i64>, DurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            let last=sqlx::query_scalar("SELECT floor(extract(epoch FROM advanced_at)*1000)::bigint FROM game_platform_stream_cursors WHERE stream=3")
                .fetch_one(&mut *tx).await?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(last)
        })).await
    }
    /// The stored resume cursor of `stream`; `None` before its first page.
    pub async fn platform_stream_cursor(
        &self,
        stream: PlatformStream,
    ) -> Result<Option<String>, DurabilityError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let cursor: Option<String> = sqlx::query_scalar(
                        "SELECT stream_cursor FROM game_platform_stream_cursors WHERE stream=$1",
                    )
                    .bind(stream.key())
                    .fetch_one(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(cursor)
                })
            })
            .await
    }

    /// Project `page`, pulled after `after`, and move the cursor to `next` (`None` keeps it).
    pub async fn apply_platform_stream_page(
        &self,
        after: Option<String>,
        page: StreamPage,
        next: Option<String>,
    ) -> Result<PageOutcome, DurabilityError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    if page.stream() == PlatformStream::SecurityRevocations {
                        super::db::lock_admission_relations(&mut tx).await?;
                    }
                    let stream = page.stream().key();
                    let stored: Option<String> = sqlx::query_scalar(CURSOR_SQL)
                        .bind(stream)
                        .fetch_one(&mut *tx)
                        .await?;
                    if stored != after {
                        return Ok(PageOutcome::Superseded);
                    }
                    let mut counts = PageCounts::default();
                    match &page {
                        StreamPage::SecurityRevocations(items) => {
                            for item in items {
                                counts.count(security_revocation::apply(&mut tx, item).await?)?;
                            }
                        }
                        StreamPage::Sanctions(items) => {
                            for item in items {
                                if !known(&mut tx, &item.account_id, item.character_id.as_ref())
                                    .await?
                                {
                                    counts.skipped = counts.skipped.saturating_add(1);
                                    continue;
                                }
                                let outcome: i16 = sqlx::query_scalar(SANCTION_SQL)
                                    .bind(item.sanction_stream_id.as_slice())
                                    .bind(item.decision_revision)
                                    .bind(item.account_id.as_slice())
                                    .bind(item.character_id.as_ref().map(|id| id.as_slice()))
                                    .bind(item.kind)
                                    .bind(item.reason_code)
                                    .bind(&item.reason_text)
                                    .bind(item.starts_at_ms)
                                    .bind(item.ends_at_ms)
                                    .bind(item.revoked)
                                    .fetch_one(&mut *tx)
                                    .await?;
                                counts.count(outcome)?;
                            }
                        }
                        StreamPage::Loyalty(items) => {
                            for item in items {
                                if !known(&mut tx, &item.account_id, None).await? {
                                    counts.skipped = counts.skipped.saturating_add(1);
                                    continue;
                                }
                                let outcome: i16 = sqlx::query_scalar(LOYALTY_SQL)
                                    .bind(item.account_id.as_slice())
                                    .bind(item.spent_premium_days)
                                    .bind(item.loyalty_points)
                                    .bind(item.platform_revision)
                                    .fetch_one(&mut *tx)
                                    .await?;
                                counts.count(outcome)?;
                            }
                        }
                    }
                    sqlx::query(ADVANCE_SQL)
                        .bind(stream)
                        .bind(next.as_deref())
                        .execute(&mut *tx)
                        .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(PageOutcome::Advanced(counts))
                })
            })
            .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn page_counts_take_only_the_three_projection_outcomes() -> Result<(), &'static str> {
        let mut counts = PageCounts::default();
        for outcome in [1, 1, 2, 3] {
            counts.count(outcome).map_err(|_| "outcome")?;
        }
        assert_eq!(
            counts,
            PageCounts {
                applied: 2,
                replayed: 1,
                stale: 1,
                skipped: 0
            }
        );
        assert!(counts.count(0).is_err());
        assert!(counts.count(4).is_err());
        assert_eq!(
            StreamPage::Loyalty(Vec::new()).stream(),
            PlatformStream::Loyalty
        );
        assert_eq!(PlatformStream::Sanctions.key(), 1);
        Ok(())
    }
}
