use super::*;
use oteryn_gameplay_world_change::horse_station::HorseStation;

/// Advance the shared phase using the immutable activation timestamp. The
/// reset transaction serializes this update with other World changes.
pub(super) async fn at_save(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    timestamp: i64,
) -> Result<(), DurabilityError> {
    let deadline: Option<i64> = sqlx::query_scalar(
        "INSERT INTO game_horse_stations(world_id) VALUES(encode($1,'hex')::uuid)
         ON CONFLICT(world_id) DO UPDATE SET world_id=EXCLUDED.world_id
         RETURNING loose_until_ms",
    )
    .bind(world.as_bytes().as_slice())
    .fetch_one(&mut **tx)
    .await?;
    if let Some(deadline) = deadline
        && HorseStation::LooseUntil(deadline).at_save(timestamp) == HorseStation::Confined
    {
        sqlx::query(
            "UPDATE game_horse_stations SET loose_until_ms=NULL WHERE world_id=encode($1,'hex')::uuid",
        )
        .bind(world.as_bytes().as_slice())
        .execute(&mut **tx)
        .await?;
    }
    Ok(())
}

impl DurabilityRoot {
    /// Settle a stable entry from the current Channel owner. The phase row lock
    /// serializes eligible entries from all Channels in the World.
    pub async fn settle_horse_lure(
        &self,
        authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
        node: &crate::durability::runtime_scope_assignment::NodeIncarnationProof,
        scope: crate::foundation::RuntimeScopeRefV1,
        generation: crate::foundation::ScopeOwnershipGeneration,
        epoch: u64,
        draw: u8,
    ) -> Result<bool, DurabilityError> {
        if epoch == 0 || draw >= 30 || !matches!(scope, crate::foundation::RuntimeScopeRefV1::Channel { .. }) {
            return Ok(false);
        }
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                let world = scope.world_id();
                let current: bool = sqlx::query_scalar(
                    "SELECT game_world_reset_map_epoch_current(encode($1,'hex')::uuid,$2::text::numeric)",
                ).bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_one(&mut *tx).await?;
                if !current || !crate::durability::character_progression::assert_runtime_scope_fence(
                    &mut tx, scope, generation, &node,
                ).await? {
                    return Ok(false);
                }
                let loose: Option<i64> = sqlx::query_scalar(
                    "SELECT loose_until_ms FROM game_horse_stations WHERE world_id=encode($1,'hex')::uuid AND loose_until_ms IS NOT NULL FOR UPDATE",
                ).bind(world.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
                let Some(loose) = loose else { return Ok(false) };
                let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                    .fetch_one(&mut *tx).await?;
                if now >= loose {
                    return Ok(false);
                }
                if HorseStation::LooseUntil(loose).after_lure(now, draw) == HorseStation::Confined {
                    sqlx::query("UPDATE game_horse_stations SET loose_until_ms=NULL WHERE world_id=encode($1,'hex')::uuid")
                        .bind(world.as_bytes().as_slice()).execute(&mut *tx).await?;
                }
                commit_semantic_transaction(tx, deadline).await?;
                Ok(true)
            }))
            .await
    }
}
