//! Crash-consistent monotonic Game consumer fence for authenticated Platform snapshots.
//! No cached allow is reconstructed after restart. Fresh committed pulls alone issue a read.
use super::spell_premium_abi::AuthenticatedSnapshot;
use super::{DurabilityError, DurabilityRoot};
use sqlx::{Postgres, Row, Transaction};
#[derive(Debug)]
pub(crate) struct CommittedPremiumRead {
    account: [u8; 16],
    snapshot: Option<AuthenticatedSnapshot>,
    conflict: bool,
    fresh: bool,
}
impl CommittedPremiumRead {
    pub(crate) fn account(&self) -> [u8; 16] {
        self.account
    }
    pub(crate) fn snapshot(&self) -> Option<&AuthenticatedSnapshot> {
        self.snapshot.as_ref()
    }
    pub(crate) fn conflict(&self) -> bool {
        self.conflict
    }
    pub(crate) fn fresh(&self) -> bool {
        self.fresh
    }
}
impl DurabilityRoot {
    /// A successful return follows the actual same-transaction high-water/history/audit commit.
    pub(crate) async fn consume_premium_snapshot(
        &self,
        snapshot: AuthenticatedSnapshot,
    ) -> Result<CommittedPremiumRead, DurabilityError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = super::db::begin_semantic_transaction(holder, deadline).await?;
                    let result = consume_in_transaction(&mut tx, snapshot).await?;
                    super::db::commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }
    /// A failed refresh never advances/rewinds evidence. Its owned result withholds benefits
    /// immediately; root account coordinator must replace the prior access snapshot on every failure.
    pub(crate) async fn premium_failed_pull(
        &self,
        account: [u8; 16],
    ) -> Result<CommittedPremiumRead, DurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder,deadline|Box::pin(async move{
   let mut tx=super::db::begin_semantic_transaction(holder,deadline).await?;
   let conflict:Option<bool>=sqlx::query_scalar("SELECT conflicting FROM game_spell_premium_accounts WHERE account_id=encode($1,'hex')::uuid FOR SHARE").bind(account.as_slice()).fetch_optional(&mut *tx).await?;
   super::db::commit_semantic_transaction(tx,deadline).await?;
   Ok(CommittedPremiumRead{account,snapshot:None,conflict:conflict.unwrap_or(false),fresh:false})
  })).await
    }
}
fn number(s: String) -> Result<u64, DurabilityError> {
    s.parse().map_err(|_| DurabilityError::Unavailable)
}
async fn consume_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    snapshot: AuthenticatedSnapshot,
) -> Result<CommittedPremiumRead, DurabilityError> {
    let account = snapshot.account();
    let revision = snapshot.authority_revision();
    let fingerprint = snapshot.fingerprint();
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:premium-account:'||encode($1,'hex'),0))").bind(account.as_slice()).execute(&mut **tx).await?;
    let current=sqlx::query("SELECT authority_revision::text,source_authority,fingerprint,conflicting FROM game_spell_premium_accounts WHERE account_id=encode($1,'hex')::uuid FOR UPDATE").bind(account.as_slice()).fetch_optional(&mut **tx).await?;
    let historical:Option<Vec<u8>>=sqlx::query_scalar("SELECT fingerprint FROM game_spell_premium_history WHERE account_id=encode($1,'hex')::uuid AND authority_revision=$2::text::numeric").bind(account.as_slice()).bind(revision.to_string()).fetch_optional(&mut **tx).await?;
    let mut reason = None;
    if historical
        .as_deref()
        .is_some_and(|old| old != fingerprint.as_slice())
    {
        reason = Some("AUTHENTICATED_EQUIVOCATION")
    }
    if !snapshot.supported() {
        reason = Some("UNSUPPORTED_SEMANTICS")
    }
    let high = current
        .as_ref()
        .map(|r| number(r.try_get("authority_revision")?))
        .transpose()?;
    if current.as_ref().is_some_and(|r| {
        r.try_get::<String, _>("source_authority").ok().as_deref() != Some(snapshot.source())
    }) {
        reason = Some("SOURCE_CHANGED")
    }
    if high == Some(revision)
        && (historical.is_none()
            || current.as_ref().is_none_or(|r| {
                r.try_get::<Vec<u8>, _>("fingerprint").ok().as_deref()
                    != Some(fingerprint.as_slice())
            }))
    {
        reason = Some("AUTHENTICATED_EQUIVOCATION");
    }
    if snapshot.entitlement_id().is_none() {
        let existed:bool=sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_spell_premium_entitlements WHERE account_id=encode($1,'hex')::uuid)")
            .bind(account.as_slice()).fetch_one(&mut **tx).await?;
        if existed {
            reason = Some("UNSUPPORTED_SEMANTICS");
        }
    }
    if let Some(id) = snapshot.entitlement_id() {
        let life:Option<String>=sqlx::query_scalar("SELECT lifecycle_revision::text FROM game_spell_premium_entitlements WHERE account_id=encode($1,'hex')::uuid AND entitlement_id=$2::uuid FOR UPDATE").bind(account.as_slice()).bind(id).fetch_optional(&mut **tx).await?;
        if life
            .map(number)
            .transpose()?
            .is_some_and(|old| snapshot.lifecycle_revision() < old)
        {
            reason = Some("LIFECYCLE_ROLLBACK")
        }
    }
    if let Some(reason) = reason {
        if current.is_none() {
            // Preserve the authenticated unsupported observation only as denied fence data.
            sqlx::query("INSERT INTO game_spell_premium_accounts(account_id,source_authority,authority_revision,fingerprint,evidence,conflicting) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric,$4,$5,TRUE)").bind(account.as_slice()).bind(snapshot.source()).bind(revision.to_string()).bind(fingerprint.as_slice()).bind(snapshot.evidence_bytes().map_err(|_|DurabilityError::Unavailable)?).execute(&mut **tx).await?;
            sqlx::query("INSERT INTO game_spell_premium_history(account_id,authority_revision,fingerprint) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3)").bind(account.as_slice()).bind(revision.to_string()).bind(fingerprint.as_slice()).execute(&mut **tx).await?;
        } else {
            sqlx::query("UPDATE game_spell_premium_accounts SET conflicting=TRUE WHERE account_id=encode($1,'hex')::uuid").bind(account.as_slice()).execute(&mut **tx).await?;
        }
        sqlx::query("INSERT INTO game_spell_premium_security_audit(event_id,account_id,authority_revision,reason) VALUES(game_character_uuid_v7(),encode($1,'hex')::uuid,$2::text::numeric,$3)").bind(account.as_slice()).bind(revision.to_string()).bind(reason).execute(&mut **tx).await?;
        return Ok(CommittedPremiumRead {
            account,
            snapshot: None,
            conflict: true,
            fresh: false,
        });
    }
    if high.is_some_and(|high| revision < high) {
        return Ok(CommittedPremiumRead {
            account,
            snapshot: None,
            conflict: current
                .as_ref()
                .is_some_and(|r| r.try_get::<bool, _>("conflicting").unwrap_or(true)),
            fresh: false,
        });
    }
    let conflict = current
        .as_ref()
        .is_some_and(|r| r.try_get::<bool, _>("conflicting").unwrap_or(true));
    if high != Some(revision) {
        // Preserve every revision for the >=30-day protocol window. No eviction or
        // pruning is inferred; exhausted bounded history refuses the refresh/benefit.
        let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_spell_premium_history WHERE account_id=encode($1,'hex')::uuid")
            .bind(account.as_slice()).fetch_one(&mut **tx).await?;
        if count >= 4096 {
            return Err(DurabilityError::Unavailable);
        }
        sqlx::query("INSERT INTO game_spell_premium_accounts(account_id,source_authority,authority_revision,fingerprint,evidence) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric,$4,$5) ON CONFLICT(account_id) DO UPDATE SET authority_revision=EXCLUDED.authority_revision,fingerprint=EXCLUDED.fingerprint,evidence=EXCLUDED.evidence").bind(account.as_slice()).bind(snapshot.source()).bind(revision.to_string()).bind(fingerprint.as_slice()).bind(snapshot.evidence_bytes().map_err(|_|DurabilityError::Unavailable)?).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO game_spell_premium_history(account_id,authority_revision,fingerprint) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3)").bind(account.as_slice()).bind(revision.to_string()).bind(fingerprint.as_slice()).execute(&mut **tx).await?;
        if let Some(id) = snapshot.entitlement_id() {
            sqlx::query("INSERT INTO game_spell_premium_entitlements(account_id,entitlement_id,lifecycle_revision,authority_revision) VALUES(encode($1,'hex')::uuid,$2::uuid,$3::text::numeric,$4::text::numeric) ON CONFLICT(account_id,entitlement_id) DO UPDATE SET lifecycle_revision=EXCLUDED.lifecycle_revision,authority_revision=EXCLUDED.authority_revision").bind(account.as_slice()).bind(id).bind(snapshot.lifecycle_revision().to_string()).bind(revision.to_string()).execute(&mut **tx).await?;
        }
    }
    // Bounded history is retained without eviction; no unsupported pruning is performed.
    Ok(CommittedPremiumRead {
        account,
        snapshot: Some(snapshot),
        conflict,
        fresh: true,
    })
}
