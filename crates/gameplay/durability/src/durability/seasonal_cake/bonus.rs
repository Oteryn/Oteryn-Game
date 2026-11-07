//! Crystal201189f `a_piece_of_cake/world_bonus.lua`: independent seven-day benefits.
use super::CakeWorldState;
use crate::durability::DurabilityRoot;
use crate::durability::boss_encounter::BossClockError;
use crate::durability::character_authority::{ReconciledCharacterAuthority, assert_recovery_fence};
use crate::durability::db::begin_semantic_transaction;
use crate::foundation::WorldId;
use sqlx::{Acquire, Postgres, Transaction};

fn deadline(value: serde_json::Value, now: i64) -> Option<i64> {
    let state: CakeWorldState = serde_json::from_value(value).ok()?;
    let (from, until) = (state.bonus_from?, state.bonus_until?);
    (from <= now && now < until).then_some(until)
}

async fn read(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    now: i64,
) -> Result<Option<i64>, sqlx::Error> {
    let state:Option<serde_json::Value>=sqlx::query_scalar("SELECT state FROM game_seasonal_cake_worlds WHERE world_id=encode($1,'hex')::uuid AND event_start<=$2 ORDER BY event_start DESC LIMIT 1")
        .bind(world.as_bytes().as_slice()).bind(now).fetch_optional(&mut **tx).await?;
    Ok(state.and_then(|value| deadline(value, now)))
}

/// Optional bonus enrichment cannot poison the Character's already-open XP transaction.
/// A savepoint contains malformed/restored bonus data and optional-read failures.
pub(crate) async fn xp_percent(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    premium: bool,
) -> i16 {
    if !premium {
        return 0;
    }
    let Ok(mut optional) = tx.begin().await else {
        return 0;
    };
    let result = async {
        let now: i64 = sqlx::query_scalar(
            "SELECT floor(extract(epoch FROM public.game_daily_effect_now()))::bigint",
        )
        .fetch_one(&mut *optional)
        .await?;
        read(&mut optional, world, now).await
    }
    .await;
    match result {
        Ok(until) => {
            if optional.commit().await.is_ok() && until.is_some() {
                50
            } else {
                0
            }
        }
        Err(_) => {
            let _ = optional.rollback().await;
            0
        }
    }
}

impl DurabilityRoot {
    /// Read-only retained bonus deadline, including after annual entity cleanup.
    /// Callers treat unavailable optional enrichment as no bonus, never a failed tick.
    pub async fn cake_world_bonus_until(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        world: WorldId,
        now: i64,
    ) -> Result<Option<i64>, BossClockError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| BossClockError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let until = read(&mut tx, world, now).await?;
                    Ok(Ok(until))
                })
            })
            .await?
    }
}
