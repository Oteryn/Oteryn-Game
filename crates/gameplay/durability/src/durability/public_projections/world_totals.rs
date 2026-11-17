//! PLAT-WIRE-1 store passes: reconciled authority and existing durability deadlines.
use super::*;
use crate::native_admission_source::public_projections::world_totals::{
    BATCH, TotalsFacts, TotalsSnapshot,
};

const WORLDS: &str = "SELECT world_id FROM game_character_roots \
    UNION SELECT world_id FROM game_public_world_deaths \
    UNION SELECT world_id FROM game_public_world_totals_heads";
const NEXT: &str = "SELECT s.world_id::text AS world_id, s.projection_epoch, \
    s.projection_revision, s.source_observed_at, s.online_players, s.player_deaths \
    FROM game_public_world_totals_samples s JOIN game_public_projection_epoch e \
    ON e.epoch_scope=1 AND e.projection_epoch=s.projection_epoch WHERE NOT s.acked \
    ORDER BY s.source_observed_at, s.world_id, s.projection_revision LIMIT $1";

fn count(row: &sqlx::postgres::PgRow, key: &str) -> std::result::Result<u64, sqlx::Error> {
    u64::try_from(row.try_get::<i64, _>(key)?).map_err(|e| sqlx::Error::Decode(Box::new(e)))
}

impl DurabilityRoot {
    pub async fn sample_public_world_totals(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        after: &str,
    ) -> Result<Option<String>> {
        let recovery = authority.record_for(self)?;
        let after = after.to_owned();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let query = format!(
                        "SELECT world_id::text FROM ({WORLDS}) w \
                WHERE nullif($1,'') IS NULL OR world_id > nullif($1,'')::uuid \
                ORDER BY world_id LIMIT $2"
                    );
                    let worlds: Vec<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(query))
                        .bind(after)
                        .bind(i64::try_from(BATCH).unwrap_or(i64::MAX))
                        .fetch_all(&mut *tx)
                        .await?;
                    for world in &worlds {
                        sqlx::query("SELECT game_public_world_totals_sample($1::uuid)")
                            .bind(world)
                            .execute(&mut *tx)
                            .await?;
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(worlds.last().cloned()))
                })
            })
            .await?
    }

    pub async fn next_public_world_totals(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
    ) -> Result<Vec<TotalsSnapshot>> {
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    let rows = sqlx::query(NEXT)
                        .bind(i64::try_from(BATCH).unwrap_or(i64::MAX))
                        .fetch_all(&mut *tx)
                        .await?;
                    let mut samples = Vec::with_capacity(rows.len());
                    for row in &rows {
                        samples.push(TotalsSnapshot {
                            world_id: row.try_get("world_id")?,
                            projection_epoch: count(row, "projection_epoch")?,
                            projection_revision: count(row, "projection_revision")?,
                            source_observed_at: row.try_get("source_observed_at")?,
                            online_players: count(row, "online_players")?,
                            player_deaths: count(row, "player_deaths")?,
                        });
                    }
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(samples))
                })
            })
            .await?
    }

    pub async fn ack_public_world_totals(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        sample: &TotalsSnapshot,
    ) -> Result<()> {
        let recovery = authority.record_for(self)?;
        let world = sample.world_id.clone();
        let epoch = wide(sample.projection_epoch)?;
        let revision = wide(sample.projection_revision)?;
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    sqlx::query(
                        "UPDATE game_public_world_totals_samples SET acked=true \
                WHERE world_id=$1::uuid AND projection_epoch=$2 AND projection_revision=$3",
                    )
                    .bind(world)
                    .bind(epoch)
                    .bind(revision)
                    .execute(&mut *tx)
                    .await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(Ok(()))
                })
            })
            .await?
    }

    pub async fn public_world_totals_facts(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
    ) -> Result<Option<TotalsFacts>> {
        let recovery = authority.record_for(self)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            // The aggregate returns one bounded row. A missing current head stalls
            // coverage, even when the delivery queue is empty.
            let query = format!("WITH worlds AS ({WORLDS}) \
                SELECT e.projection_epoch, floor(extract(epoch FROM clock_timestamp()))::bigint AS now, \
                CASE WHEN count(w.world_id)=0 OR bool_or(h.world_id IS NULL \
                    OR h.projection_epoch<>e.projection_epoch OR h.projection_revision=0) THEN NULL \
                ELSE least(min(h.sampled_at), coalesce((SELECT min(s.source_observed_at)-1 \
                    FROM game_public_world_totals_samples s WHERE NOT s.acked \
                    AND s.projection_epoch=e.projection_epoch), min(h.sampled_at))) END AS through \
                FROM game_public_projection_epoch e LEFT JOIN worlds w ON true \
                LEFT JOIN game_public_world_totals_heads h USING(world_id) \
                WHERE e.epoch_scope=1 GROUP BY e.projection_epoch");
            let row = sqlx::query(sqlx::AssertSqlSafe(query)).fetch_one(&mut *tx).await?;
            let facts = row.try_get::<Option<i64>, _>("through")?.map(|through| {
                Ok::<_, sqlx::Error>(TotalsFacts {
                    projection_epoch: count(&row, "projection_epoch")?,
                    complete_through: through,
                    observed_at: row.try_get("now")?,
                })
            }).transpose()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(facts))
        })).await?
    }
}
