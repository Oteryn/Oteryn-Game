//! Durable World reset owner (ADR-0021 §4.7). A decoded job is history;
//! the privately constructed drain witness comes only from a current RETIRING row.
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::{ControlLossCauseV1, WorldId};
use sqlx::{Postgres, Row, Transaction};

type Result<T> = std::result::Result<T, DurabilityError>;

/// Sealed planned-reset signal. No caller-selected reset or loss flag can construct it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RetiringWorldReset {
    pub(super) world: WorldId,
    pub(super) epoch: u64,
    pub(super) binding: [u8; 32],
    pub(super) target: [u8; 32],
}

pub(super) mod census;
mod activation;
mod content_preflight;
mod ground_retirement;
mod active_epoch;
pub(super) mod ground_step;
#[cfg(test)]
mod migration_tests;

impl RetiringWorldReset {
    pub fn loss_cause(&self) -> ControlLossCauseV1 {
        ControlLossCauseV1::PlannedWorldReset {
            world_id: self.world,
            reset_epoch: self.epoch,
            retirement_binding: self.binding,
            target_digest: self.target,
        }
    }
}

impl DurabilityRoot {
    /// Read only the current job, under a semantic-pass deadline. A signal is
    /// revalidated at the loss transaction; retaining it cannot retain authority.
    pub async fn retiring_world_reset(&self, world: WorldId) -> Result<Option<RetiringWorldReset>> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let row = sqlx::query("SELECT reset_epoch::text, retirement_binding, target_digest FROM game_world_reset_jobs WHERE world_id=encode($1,'hex')::uuid AND phase='RETIRING' LIMIT 1")
                .bind(world.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let result = row.map(|row| -> Result<RetiringWorldReset> {
                let epoch = row.try_get::<String, _>(0)?.parse::<u64>()
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                let binding = row.try_get::<Vec<u8>, _>(1)?.try_into()
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                let target = row.try_get::<Vec<u8>, _>(2)?.try_into().map_err(|_| DurabilityError::InvalidStoredState)?;
                Ok(RetiringWorldReset { world, epoch, binding, target })
            }).transpose()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(result)
        })).await
    }
}

pub(super) async fn loss_binding_current(
    tx: &mut Transaction<'_, Postgres>,
    cause: ControlLossCauseV1,
) -> Result<bool> {
    match cause {
        ControlLossCauseV1::AuthoritativeUnexpectedLoss => Ok(true),
        ControlLossCauseV1::PlannedWorldReset {
            world_id,
            reset_epoch,
            retirement_binding,
            target_digest,
        } => Ok(sqlx::query_scalar(
            "SELECT game_world_reset_loss_binding(encode($1,'hex')::uuid,$2::text::numeric,$3,$4)",
        )
        .bind(world_id.as_bytes().as_slice())
        .bind(reset_epoch.to_string())
        .bind(retirement_binding.as_slice())
        .bind(target_digest.as_slice())
        .fetch_one(&mut **tx)
        .await?),
        _ => Ok(false),
    }
}

#[cfg(any(test, feature = "test-support"))]
#[path = "world_reset/retiring_fixture.rs"]
pub mod test_support;
