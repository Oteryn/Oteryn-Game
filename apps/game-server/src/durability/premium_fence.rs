//! PREM-1a: the durable Premium consumer fence (migration 0029; PREMIUM-DELIVERY-0 §6,
//! PROD-ENTITLEMENTS-01 consumer contract §6).
//!
//! [`DurabilityRoot::accept_premium_evidence`] consumes one validated snapshot in one
//! transaction under the account fence row lock:
//!
//! - an `authority_revision` above the account high water is stored as evidence and advances
//!   the account fence and, for an entitlement, its (account, entitlement) fence;
//! - a revision already stored with the same fingerprint is an idempotent replay (at the high
//!   water) or stale (below it); with another fingerprint it is producer equivocation and marks
//!   the account conflicting, stickily, even below the high water (§6.2 rule 4);
//! - a revision below the high water that was never stored is stale;
//! - a higher authority revision that lowers an entitlement's `lifecycle_revision`, or repeats
//!   it with other lifecycle facts, is a conflict too.
//!
//! Every outcome returns the durable view after the transaction. The caller authorizes benefit
//! only from that view (fence before authorize, §6.3); this module holds no Premium policy.

use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::{DurabilityError, DurabilityRoot};
use sha2::{Digest, Sha256};
use sqlx::Row;

type Tx<'a> = sqlx::Transaction<'a, sqlx::Postgres>;
const FINGERPRINT_VERSION: u8 = 1;

/// The producer lifecycle state of a snapshot (PREMIUM-DELIVERY-0 §4).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EntitlementState {
    Active = 1,
    NotYetEffective = 2,
    Expired = 3,
    Revoked = 4,
    None = 5,
}

impl EntitlementState {
    fn from_stored(value: i16) -> Result<Self, DurabilityError> {
        Ok(match value {
            1 => Self::Active,
            2 => Self::NotYetEffective,
            3 => Self::Expired,
            4 => Self::Revoked,
            5 => Self::None,
            _ => return Err(DurabilityError::InvalidStoredState),
        })
    }
}

/// One validated `oteryn.premium_snapshot.v1`, times in Unix microseconds. The request nonce is
/// not part of it: it binds the response to its request and is checked before this exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PremiumEvidence {
    pub account_id: [u8; 16],
    pub producer_revision: String,
    pub producer_profile: String,
    pub product_id: String,
    pub product_version: u32,
    pub entitlement_id: Option<String>,
    pub state: EntitlementState,
    pub lifecycle_revision: u64,
    pub authority_revision: u64,
    pub effective_from_us: i64,
    pub effective_until_us: i64,
    pub authority_issued_at_us: i64,
    pub authority_valid_until_us: i64,
    pub refresh_after_us: i64,
}

impl PremiumEvidence {
    /// The entitlement's lifecycle facts: what may change only with a new `lifecycle_revision`.
    fn lifecycle_fingerprint(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update([FINGERPRINT_VERSION, self.state as u8]);
        for text in [
            &self.product_id,
            self.entitlement_id.as_deref().unwrap_or(""),
        ] {
            hash.update((text.len() as u64).to_be_bytes());
            hash.update(text.as_bytes());
        }
        hash.update(self.product_version.to_be_bytes());
        hash.update(self.lifecycle_revision.to_be_bytes());
        hash.update(self.effective_from_us.to_be_bytes());
        hash.update(self.effective_until_us.to_be_bytes());
        hash.finalize().into()
    }

    /// Every semantic field of one ordered authority. `producer_revision` (build provenance)
    /// and `refresh_after` (scheduling only) are excluded.
    fn fingerprint(&self) -> [u8; 32] {
        let mut hash = Sha256::new();
        hash.update([FINGERPRINT_VERSION]);
        hash.update(self.account_id);
        hash.update(self.lifecycle_fingerprint());
        hash.update((self.producer_profile.len() as u64).to_be_bytes());
        hash.update(self.producer_profile.as_bytes());
        hash.update(self.authority_revision.to_be_bytes());
        hash.update(self.authority_issued_at_us.to_be_bytes());
        hash.update(self.authority_valid_until_us.to_be_bytes());
        hash.finalize().into()
    }
}

/// The durable fence of one account: its latest accepted evidence and whether producer
/// equivocation was ever detected for it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PremiumFenceView {
    pub latest: PremiumEvidence,
    pub conflicting: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PremiumFenceOutcome {
    /// The evidence advanced the fence.
    Accepted(PremiumFenceView),
    /// The evidence is exactly the current high water.
    Replayed(PremiumFenceView),
    /// The evidence is below the high water and contradicts nothing.
    Stale(PremiumFenceView),
    /// The evidence contradicts accepted evidence; the account is marked conflicting.
    Conflict(PremiumFenceView),
}

impl DurabilityRoot {
    /// Fence one validated snapshot (module docs). Nothing is written unless the transaction
    /// commits; an error leaves the fence as it was or with an unknown commit outcome, and the
    /// caller must not authorize benefit from the evidence.
    pub async fn accept_premium_evidence(
        &self,
        evidence: &PremiumEvidence,
    ) -> Result<PremiumFenceOutcome, DurabilityError> {
        let evidence = evidence.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let outcome = accept(&mut tx, &evidence).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(outcome)
                })
            })
            .await
    }

    /// The durable fence of one account, or `None` before its first accepted snapshot.
    pub async fn load_premium_fence(
        &self,
        account_id: [u8; 16],
    ) -> Result<Option<PremiumFenceView>, DurabilityError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let view = match account_fence(&mut tx, account_id, false).await? {
                        Some((high, conflict)) => {
                            Some(view(&mut tx, account_id, high, conflict).await?)
                        }
                        None => None,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(view)
                })
            })
            .await
    }
}

async fn accept(
    tx: &mut Tx<'_>,
    evidence: &PremiumEvidence,
) -> Result<PremiumFenceOutcome, DurabilityError> {
    let account = evidence.account_id;
    let revision = evidence.authority_revision;
    let Some((high, conflict)) = account_fence(tx, account, true).await? else {
        let step = entitlement_step(tx, evidence).await?;
        insert_evidence(tx, evidence).await?;
        write_entitlement(tx, evidence, step).await?;
        sqlx::query(
            "INSERT INTO game_premium_account_fence(account_id, authority_revision) \
             VALUES (encode($1,'hex')::uuid, $2::numeric)",
        )
        .bind(account.as_slice())
        .bind(revision.to_string())
        .execute(&mut **tx)
        .await?;
        return Ok(PremiumFenceOutcome::Accepted(
            view(tx, account, revision, false).await?,
        ));
    };
    if let Some(stored) = stored_fingerprint(tx, account, revision).await? {
        if stored != evidence.fingerprint() {
            return mark_conflict(tx, account, high, conflict, revision).await;
        }
        let current = view(tx, account, high, conflict).await?;
        return Ok(if revision == high {
            PremiumFenceOutcome::Replayed(current)
        } else {
            PremiumFenceOutcome::Stale(current)
        });
    }
    if revision < high {
        return Ok(PremiumFenceOutcome::Stale(
            view(tx, account, high, conflict).await?,
        ));
    }
    let step = entitlement_step(tx, evidence).await?;
    if step == EntitlementStep::Regress {
        return mark_conflict(tx, account, high, conflict, revision).await;
    }
    insert_evidence(tx, evidence).await?;
    write_entitlement(tx, evidence, step).await?;
    sqlx::query(
        "UPDATE game_premium_account_fence SET authority_revision = $2::numeric \
          WHERE account_id = encode($1,'hex')::uuid",
    )
    .bind(account.as_slice())
    .bind(revision.to_string())
    .execute(&mut **tx)
    .await?;
    Ok(PremiumFenceOutcome::Accepted(
        view(tx, account, revision, conflict).await?,
    ))
}

/// The account high water and conflict marker, row-locked when `lock`.
async fn account_fence(
    tx: &mut Tx<'_>,
    account: [u8; 16],
    lock: bool,
) -> Result<Option<(u64, bool)>, DurabilityError> {
    let sql = if lock {
        "SELECT authority_revision::text AS high, conflict_authority_revision IS NOT NULL AS conflict \
           FROM game_premium_account_fence WHERE account_id = encode($1,'hex')::uuid FOR UPDATE"
    } else {
        "SELECT authority_revision::text AS high, conflict_authority_revision IS NOT NULL AS conflict \
           FROM game_premium_account_fence WHERE account_id = encode($1,'hex')::uuid"
    };
    let Some(row) = sqlx::query(sql)
        .bind(account.as_slice())
        .fetch_optional(&mut **tx)
        .await?
    else {
        return Ok(None);
    };
    Ok(Some((numeric(&row, "high")?, row.try_get("conflict")?)))
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum EntitlementStep {
    /// `NONE`: the snapshot names no entitlement.
    Skip,
    Open,
    Advance,
    /// The evidence lowers the entitlement's lifecycle revision or repeats it with other
    /// lifecycle facts.
    Regress,
}

/// Check the evidence against its entitlement fence, row-locked.
async fn entitlement_step(
    tx: &mut Tx<'_>,
    evidence: &PremiumEvidence,
) -> Result<EntitlementStep, DurabilityError> {
    let Some(entitlement) = &evidence.entitlement_id else {
        return Ok(EntitlementStep::Skip);
    };
    let Some(row) = sqlx::query(
        "SELECT lifecycle_revision::text AS lifecycle, lifecycle_fingerprint \
           FROM game_premium_entitlement_fence \
          WHERE account_id = encode($1,'hex')::uuid AND entitlement_id = $2 FOR UPDATE",
    )
    .bind(evidence.account_id.as_slice())
    .bind(entitlement)
    .fetch_optional(&mut **tx)
    .await?
    else {
        return Ok(EntitlementStep::Open);
    };
    let stored: u64 = numeric(&row, "lifecycle")?;
    let stored_fingerprint: Vec<u8> = row.try_get("lifecycle_fingerprint")?;
    Ok(
        if evidence.lifecycle_revision < stored
            || (evidence.lifecycle_revision == stored
                && stored_fingerprint.as_slice() != evidence.lifecycle_fingerprint().as_slice())
        {
            EntitlementStep::Regress
        } else {
            EntitlementStep::Advance
        },
    )
}

/// Open or advance the entitlement fence to the evidence, which is already stored.
async fn write_entitlement(
    tx: &mut Tx<'_>,
    evidence: &PremiumEvidence,
    step: EntitlementStep,
) -> Result<(), DurabilityError> {
    let sql = match step {
        EntitlementStep::Skip => return Ok(()),
        EntitlementStep::Regress => return Err(DurabilityError::InvalidStoredState),
        EntitlementStep::Open => {
            "INSERT INTO game_premium_entitlement_fence(account_id, entitlement_id, \
               lifecycle_revision, lifecycle_fingerprint, authority_revision) \
             VALUES (encode($1,'hex')::uuid, $2, $3::numeric, $4, $5::numeric)"
        }
        EntitlementStep::Advance => {
            "UPDATE game_premium_entitlement_fence SET lifecycle_revision = $3::numeric, \
               lifecycle_fingerprint = $4, authority_revision = $5::numeric \
             WHERE account_id = encode($1,'hex')::uuid AND entitlement_id = $2"
        }
    };
    sqlx::query(sql)
        .bind(evidence.account_id.as_slice())
        .bind(evidence.entitlement_id.as_deref())
        .bind(evidence.lifecycle_revision.to_string())
        .bind(evidence.lifecycle_fingerprint().as_slice())
        .bind(evidence.authority_revision.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

/// Record detected equivocation at `revision` (sticky: the first detection stays) and return
/// the fence, whose high water is unchanged.
async fn mark_conflict(
    tx: &mut Tx<'_>,
    account: [u8; 16],
    high: u64,
    already: bool,
    revision: u64,
) -> Result<PremiumFenceOutcome, DurabilityError> {
    if !already {
        sqlx::query(
            "UPDATE game_premium_account_fence SET conflict_authority_revision = $2::numeric \
              WHERE account_id = encode($1,'hex')::uuid",
        )
        .bind(account.as_slice())
        .bind(revision.to_string())
        .execute(&mut **tx)
        .await?;
    }
    Ok(PremiumFenceOutcome::Conflict(
        view(tx, account, high, true).await?,
    ))
}

async fn stored_fingerprint(
    tx: &mut Tx<'_>,
    account: [u8; 16],
    revision: u64,
) -> Result<Option<[u8; 32]>, DurabilityError> {
    let stored: Option<Vec<u8>> = sqlx::query_scalar(
        "SELECT fingerprint FROM game_premium_evidence \
          WHERE account_id = encode($1,'hex')::uuid AND authority_revision = $2::numeric",
    )
    .bind(account.as_slice())
    .bind(revision.to_string())
    .fetch_optional(&mut **tx)
    .await?;
    stored
        .map(|bytes| <[u8; 32]>::try_from(bytes).map_err(|_| DurabilityError::InvalidStoredState))
        .transpose()
}

async fn insert_evidence(tx: &mut Tx<'_>, e: &PremiumEvidence) -> Result<(), DurabilityError> {
    sqlx::query(
        "INSERT INTO game_premium_evidence(account_id, authority_revision, fingerprint, \
           producer_revision, producer_profile, product_id, product_version, entitlement_id, \
           entitlement_state, lifecycle_revision, effective_from_us, effective_until_us, \
           authority_issued_at_us, authority_valid_until_us, refresh_after_us) \
         VALUES (encode($1,'hex')::uuid, $2::numeric, $3, $4, $5, $6, $7, $8, $9, \
           $10::numeric, $11, $12, $13, $14, $15)",
    )
    .bind(e.account_id.as_slice())
    .bind(e.authority_revision.to_string())
    .bind(e.fingerprint().as_slice())
    .bind(&e.producer_revision)
    .bind(&e.producer_profile)
    .bind(&e.product_id)
    .bind(i64::from(e.product_version))
    .bind(e.entitlement_id.as_deref())
    .bind(e.state as i16)
    .bind(e.lifecycle_revision.to_string())
    .bind(e.effective_from_us)
    .bind(e.effective_until_us)
    .bind(e.authority_issued_at_us)
    .bind(e.authority_valid_until_us)
    .bind(e.refresh_after_us)
    .execute(&mut **tx)
    .await?;
    Ok(())
}

async fn view(
    tx: &mut Tx<'_>,
    account: [u8; 16],
    revision: u64,
    conflicting: bool,
) -> Result<PremiumFenceView, DurabilityError> {
    let row = sqlx::query(
        "SELECT fingerprint, producer_revision, producer_profile, product_id, product_version, \
                entitlement_id, entitlement_state, lifecycle_revision::text AS lifecycle, \
                effective_from_us, effective_until_us, authority_issued_at_us, \
                authority_valid_until_us, refresh_after_us \
           FROM game_premium_evidence \
          WHERE account_id = encode($1,'hex')::uuid AND authority_revision = $2::numeric",
    )
    .bind(account.as_slice())
    .bind(revision.to_string())
    .fetch_one(&mut **tx)
    .await?;
    let latest = PremiumEvidence {
        account_id: account,
        producer_revision: row.try_get("producer_revision")?,
        producer_profile: row.try_get("producer_profile")?,
        product_id: row.try_get("product_id")?,
        product_version: u32::try_from(row.try_get::<i64, _>("product_version")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        entitlement_id: row.try_get("entitlement_id")?,
        state: EntitlementState::from_stored(row.try_get("entitlement_state")?)?,
        lifecycle_revision: numeric(&row, "lifecycle")?,
        authority_revision: revision,
        effective_from_us: row.try_get("effective_from_us")?,
        effective_until_us: row.try_get("effective_until_us")?,
        authority_issued_at_us: row.try_get("authority_issued_at_us")?,
        authority_valid_until_us: row.try_get("authority_valid_until_us")?,
        refresh_after_us: row.try_get("refresh_after_us")?,
    };
    // A stored row whose content no longer matches its fingerprint is not evidence.
    if row.try_get::<Vec<u8>, _>("fingerprint")?.as_slice() != latest.fingerprint().as_slice() {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(PremiumFenceView {
        latest,
        conflicting,
    })
}

fn numeric(row: &sqlx::postgres::PgRow, column: &str) -> Result<u64, DurabilityError> {
    row.try_get::<String, _>(column)?
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)
}
