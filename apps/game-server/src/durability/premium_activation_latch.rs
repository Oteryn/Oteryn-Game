//! PREM-WIRE-1: the durable Premium activation latch (PREMIUM-ACTIVATION-0 §1.2, migration 0085).
//!
//! One row at most: Premium has been delivered on this deployment. It is written once, by
//! [`DurabilityRoot::latch_premium_activation`], and never updated or deleted. The latch insert
//! takes [`PREMIUM_ACTIVATION_LOCK_KEY`] exclusively; [`DurabilityRoot::read_premium_activation`]
//! takes it shared as the first statement of its transaction and runs a pre-delivery bypass under
//! it, so a latch commit waits for every bypass that read an empty table, and every read after
//! the latch commit sees the row.

use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::{DurabilityError, DurabilityRoot};
use sqlx::Row;
use std::future::Future;
use std::pin::Pin;

/// The gate's transaction: open only before delivery, for a pre-delivery bypass.
pub type PremiumGateTx<'t> = sqlx::Transaction<'t, sqlx::Postgres>;

/// The one advisory lock key of the latch: `hashtextextended('oteryn:premium-activation', 0)`
/// would also do, but a constant keeps the key visible next to the code that takes it.
pub const PREMIUM_ACTIVATION_LOCK_KEY: i64 = 0x4f54_5052_454d_4143;

/// The latched activation: the activation record's id, its switch-over instant `S` and when the
/// latch was written, in Unix microseconds.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PremiumActivationLatch {
    pub activation_id: String,
    pub switch_over_us: i64,
    pub latched_at_us: i64,
}

/// What a shared read of the latch found.
#[derive(Debug, PartialEq, Eq)]
pub enum PremiumActivationRead<T> {
    /// A row exists: Premium has been delivered.
    Latched(PremiumActivationLatch),
    /// The table was empty and the pre-delivery bypass ran under the shared lock and committed.
    PreDelivery(T),
    /// The table was empty and no bypass was given; nothing was written.
    Empty,
}

/// A pre-delivery bypass: applied in the gate's transaction, which holds the shared lock and read
/// an empty table. Its writes commit with it, before any latch can.
pub trait PreDeliveryBypass: Send {
    type Output: Send;
    fn apply<'a, 't>(
        self,
        tx: &'a mut PremiumGateTx<'t>,
    ) -> Pin<Box<dyn Future<Output = Self::Output> + Send + 'a>>
    where
        Self: 'a;
}

impl DurabilityRoot {
    /// Take the shared lock, then read the latch, and end the transaction unwritten: `Latched`
    /// or `Empty`.
    pub async fn read_premium_activation<T: Send>(
        &self,
    ) -> Result<PremiumActivationRead<T>, DurabilityError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    lock_shared(&mut tx).await?;
                    let latch = read_latch(&mut tx).await?;
                    tx.rollback().await?;
                    Ok(latch.map_or(PremiumActivationRead::Empty, PremiumActivationRead::Latched))
                })
            })
            .await
    }

    /// Take the shared lock, then read the latch. With an empty table, take the pre-delivery
    /// bypass out of `bypass`, run it in the same transaction and commit it: its writes commit
    /// before any latch can. `bypass` is left in place when the table holds a row or the read
    /// fails before the table was read.
    pub async fn read_premium_activation_with<B: PreDeliveryBypass>(
        &self,
        bypass: &mut Option<B>,
    ) -> Result<PremiumActivationRead<B::Output>, DurabilityError> {
        self.try_issue_semantic_pass()?
            .run_with_context(bypass, |holder, deadline, bypass| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    lock_shared(&mut tx).await?;
                    if let Some(latch) = read_latch(&mut tx).await? {
                        tx.rollback().await?;
                        return Ok(PremiumActivationRead::Latched(latch));
                    }
                    let Some(bypass) = bypass.take() else {
                        tx.rollback().await?;
                        return Ok(PremiumActivationRead::Empty);
                    };
                    let applied = bypass.apply(&mut tx).await;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(PremiumActivationRead::PreDelivery(applied))
                })
            })
            .await
    }

    /// Latch `activation` at `latched_at_us`: take the lock exclusively (waiting at most 5 s for
    /// in-flight pre-delivery bypasses) and insert the one row. Returns the row that holds, which
    /// is another node's winner when the insert conflicts.
    pub async fn latch_premium_activation(
        &self,
        activation_id: &str,
        switch_over_us: i64,
        latched_at_us: i64,
    ) -> Result<PremiumActivationLatch, DurabilityError> {
        let activation_id = activation_id.to_owned();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    // Wait at most 5 s for in-flight pre-delivery bypasses.
                    sqlx::query("SET LOCAL lock_timeout = '5s'")
                        .execute(&mut *tx)
                        .await?;
                    sqlx::query("SELECT pg_advisory_xact_lock($1)")
                        .bind(PREMIUM_ACTIVATION_LOCK_KEY)
                        .execute(&mut *tx)
                        .await?;
                    let inserted = sqlx::query(
                        "INSERT INTO game_premium_activation_latch \
                         (id, activation_id, switch_over_us, latched_at_us) \
                         VALUES (1, $1, $2, $3) ON CONFLICT (id) DO NOTHING \
                         RETURNING activation_id, switch_over_us, latched_at_us",
                    )
                    .bind(&activation_id)
                    .bind(switch_over_us)
                    .bind(latched_at_us)
                    .fetch_optional(&mut *tx)
                    .await?;
                    let latch = match inserted {
                        Some(row) => latch_from(&row)?,
                        None => read_latch(&mut tx)
                            .await?
                            .ok_or(DurabilityError::InvalidStoredState)?,
                    };
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(latch)
                })
            })
            .await
    }
}

/// The gate's first statement after `BEGIN` (PREMIUM-ACTIVATION-0 §1.2, one global lock order).
async fn lock_shared(tx: &mut PremiumGateTx<'_>) -> Result<(), DurabilityError> {
    sqlx::query("SELECT pg_advisory_xact_lock_shared($1)")
        .bind(PREMIUM_ACTIVATION_LOCK_KEY)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

async fn read_latch(
    tx: &mut PremiumGateTx<'_>,
) -> Result<Option<PremiumActivationLatch>, DurabilityError> {
    sqlx::query(
        "SELECT activation_id, switch_over_us, latched_at_us \
         FROM game_premium_activation_latch WHERE id = 1",
    )
    .fetch_optional(&mut **tx)
    .await?
    .as_ref()
    .map(latch_from)
    .transpose()
}

fn latch_from(row: &sqlx::postgres::PgRow) -> Result<PremiumActivationLatch, DurabilityError> {
    Ok(PremiumActivationLatch {
        activation_id: row.try_get("activation_id")?,
        switch_over_us: row.try_get("switch_over_us")?,
        latched_at_us: row.try_get("latched_at_us")?,
    })
}
