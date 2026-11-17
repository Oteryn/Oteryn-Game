//! SEASONWIN's save-bound calendar, durable progress, and timed world rewards.
//! TibiaWiki Annual Autumn Vintage and Annual Autumn Vintage/Spoiler (fetched 2026-11-17).
//! TibiaWiki Demon's Lullaby and Demon's Lullaby/Spoiler (fetched 2026-11-17).
use super::db::{
    begin_semantic_transaction, commit_semantic_transaction, lock_admission_relations,
};
use super::{DurabilityError, DurabilityRoot};
use crate::foundation::WorldId;
use sqlx::{Postgres, Row, Transaction};

#[derive(Clone, Copy, Debug)]
pub enum SeasonalEvent {
    AutumnVintage,
    DemonsLullaby,
}
impl SeasonalEvent {
    fn key(self) -> &'static str {
        match self {
            Self::AutumnVintage => "autumn_vintage",
            Self::DemonsLullaby => "demons_lullaby",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SeasonalProgress {
    pub progress: i64,
    pub succeeded: bool,
    pub expires_ms: Option<i64>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SeasonalBuffs {
    pub experience_percent: u32,
    pub potion_percent: u32,
}
impl SeasonalBuffs {
    pub fn scale_experience(self, amount: i64) -> Result<i64, DurabilityError> {
        i64::try_from(i128::from(amount) * i128::from(self.experience_percent) / 100)
            .map_err(|_| DurabilityError::InvalidStoredState)
    }
    pub fn scale_potion(self, amount: u64) -> u64 {
        u64::try_from(u128::from(amount) * u128::from(self.potion_percent) / 100)
            .unwrap_or(u64::MAX)
    }
}

/// TibiaWiki Annual Autumn Vintage/Spoiler (fetched 2026-11-17):
/// 100 bunches form a completed load; partial loads are not reported as completed.
pub fn autumn_status(progress: Option<&SeasonalProgress>) -> String {
    let loads = progress.map_or(0, |state| state.progress / 100);
    if loads == 0 {
        "As of now, no berries have been processed...".into()
    } else {
        format!("As of now, {loads} loads have been processed. Keep it up!")
    }
}

pub(super) async fn capture(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<(), DurabilityError> {
    sqlx::query("SELECT game_seasonwin_capture(encode($1,'hex')::uuid,$2::text::numeric)")
        .bind(world.as_bytes().as_slice())
        .bind(epoch.to_string())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

pub(super) async fn buffs(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
) -> Result<SeasonalBuffs, DurabilityError> {
    // At most three admitted annual ranges can overlap; percentages are selected,
    // never accumulated on each query, login or save replay.
    let row: (i32,i32) = sqlx::query_as("SELECT coalesce(max(xp_percent),100),coalesce(max(potion_percent),100) FROM game_seasonwin_rounds WHERE world_id=encode($1,'hex')::uuid AND succeeded_at<=game_seasonwin_now() AND expires_at>game_seasonwin_now()")
        .bind(world.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    Ok(SeasonalBuffs {
        experience_percent: u32::try_from(row.0)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        potion_percent: u32::try_from(row.1).map_err(|_| DurabilityError::InvalidStoredState)?,
    })
}

pub(super) async fn announcements(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
) -> Result<Vec<String>, DurabilityError> {
    Ok(sqlx::query_scalar("SELECT DISTINCT CASE event WHEN 'autumn_vintage' THEN 'Annual Autumn Vintage' WHEN 'demons_lullaby' THEN 'Demon''s Lullaby' END || CASE WHEN succeeded_at IS NOT NULL AND expires_at>game_seasonwin_now() THEN ' reward is active.' ELSE ' is active.' END FROM game_seasonwin_rounds WHERE world_id=encode($1,'hex')::uuid AND ((opens_at<=game_seasonwin_now() AND closes_at>game_seasonwin_now()) OR (succeeded_at<=game_seasonwin_now() AND expires_at>game_seasonwin_now())) ORDER BY 1 LIMIT 3")
        .bind(world.as_bytes().as_slice()).fetch_all(&mut **tx).await?)
}

impl DurabilityRoot {
    /// Trusted server participation producers call this with their durable source
    /// receipt. No client command can directly supply progress. None refuses a
    /// closed or not-yet-save-admitted window. A repeated receipt returns its
    /// original outcome, even if the round subsequently succeeds or expires.
    pub async fn record_progress(
        &self,
        world: WorldId,
        epoch: u64,
        event: SeasonalEvent,
        amount: i64,
        receipt: [u8; 16],
    ) -> Result<Option<SeasonalProgress>, DurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder,deadline| Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            lock_admission_relations(&mut tx).await?;
            let row=sqlx::query("SELECT * FROM game_seasonwin_record(encode($1,'hex')::uuid,$2::text::numeric,$3,$4,encode($5,'hex')::uuid)")
                .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).bind(event.key()).bind(amount).bind(receipt.as_slice()).fetch_optional(&mut *tx).await?;
            let result=row.map(|r| Ok::<_,DurabilityError>(SeasonalProgress { progress:r.try_get("progress")?,succeeded:r.try_get("succeeded")?,expires_ms:r.try_get("expires_ms")? })).transpose()?;
            commit_semantic_transaction(tx,deadline).await?;
            Ok(result)
        })).await
    }

    /// Latest admitted round, retained for status dialogue after the reward ends.
    pub async fn seasonal_progress(
        &self,
        world: WorldId,
        event: SeasonalEvent,
    ) -> Result<Option<SeasonalProgress>, DurabilityError> {
        self.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move {
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            let row=sqlx::query("SELECT progress,succeeded_at IS NOT NULL AS succeeded,(extract(epoch FROM expires_at)*1000)::bigint AS expires_ms FROM game_seasonwin_rounds WHERE world_id=encode($1,'hex')::uuid AND event=$2 ORDER BY event_year DESC,slot DESC LIMIT 1")
                .bind(world.as_bytes().as_slice()).bind(event.key()).fetch_optional(&mut *tx).await?;
            let result=row.map(|r| Ok::<_,DurabilityError>(SeasonalProgress { progress:r.try_get("progress")?,succeeded:r.try_get("succeeded")?,expires_ms:r.try_get("expires_ms")? })).transpose()?;
            commit_semantic_transaction(tx,deadline).await?; Ok(result)
        })).await
    }

    pub async fn seasonal_buffs(&self, world: WorldId) -> Result<SeasonalBuffs, DurabilityError> {
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    let result = buffs(&mut tx, world).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(result)
                })
            })
            .await
    }
}
