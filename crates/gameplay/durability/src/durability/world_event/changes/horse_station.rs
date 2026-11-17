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

impl DurabilityRoot {
    /// Debit the rental and retain its character-specific lease in one transaction.
    pub async fn rent_horse(
        &self,
        authority: &crate::durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
        node: &crate::durability::runtime_scope_assignment::NodeIncarnationProof,
        fence: crate::durability::character_progression::CurrentCharacterGameplayFence,
        epoch: u64,
        fee: crate::durability::item_fee_burn::FeeBurnRequest,
    ) -> Result<Option<oteryn_gameplay_world_change::horse_station::HorseRental>, DurabilityError> {
        use crate::durability::db::begin_type2_transaction;
        use crate::durability::item_fee_burn::{FeeBurnCause, FeeBurnError, burn_fee_in_transaction};
        use oteryn_gameplay_world_change::horse_station::{HorseRental, RENTAL_PRICE_GOLD};
        use sha2::{Digest, Sha256};

        let FeeBurnCause::HorseRental { occurrence } = fee.cause else {
            return Err(DurabilityError::InvalidStoredState);
        };
        if fee.fee_gold_units != RENTAL_PRICE_GOLD {
            return Err(DurabilityError::InvalidStoredState);
        }
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::InvalidStoredState)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let world = fence.runtime_scope.world_id();
            let current: bool = sqlx::query_scalar("SELECT game_world_reset_map_epoch_current(encode($1,'hex')::uuid,$2::text::numeric)")
                .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_one(&mut **tx).await?;
            if !current || crate::durability::character_progression::assert_gameplay_fence(&mut tx, &fence, &node).await?.is_err() {
                return Ok(None);
            }
            let character = fence.character_id.as_bytes().as_slice();
            let replay: Option<(i16, i64)> = sqlx::query_as(
                "SELECT mount_id, expires_at_ms FROM game_horse_rental_receipts
                 WHERE occurrence_id=encode($1,'hex')::uuid AND character_id=encode($2,'hex')::uuid
                   AND fee_transaction_id=encode($3,'hex')::uuid"
            ).bind(occurrence.as_slice()).bind(character).bind(fee.transaction_id.as_slice()).fetch_optional(&mut **tx).await?;
            if let Some((mount, expires_at_ms)) = replay {
                return Ok(Some(HorseRental { mount_id: u16::try_from(mount).map_err(|_| DurabilityError::InvalidStoredState)?, expires_at_ms }));
            }
            sqlx::query("INSERT INTO game_horse_stations(world_id) VALUES(encode($1,'hex')::uuid) ON CONFLICT DO NOTHING")
                .bind(world.as_bytes().as_slice()).execute(&mut **tx).await?;
            let loose: Option<i64> = sqlx::query_scalar("SELECT loose_until_ms FROM game_horse_stations WHERE world_id=encode($1,'hex')::uuid FOR UPDATE")
                .bind(world.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            let phase = loose.map_or(HorseStation::Confined, HorseStation::LooseUntil);
            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&mut **tx).await?;
            let held: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_horse_rental_receipts WHERE character_id=encode($1,'hex')::uuid AND expires_at_ms>$2)")
                .bind(character).bind(now).fetch_one(&mut **tx).await?;
            let draw = Sha256::new().chain_update(b"oteryn:horse-rental:v1").chain_update(occurrence).finalize();
            let Some(lease) = phase.rental(now, None, RENTAL_PRICE_GOLD, usize::from(draw[0]) % 3).filter(|_| !held) else {
                return Ok(None);
            };
            match burn_fee_in_transaction(&mut tx, &fence, &fee).await {
                Ok(_) => {}
                Err(FeeBurnError::InsufficientFunds | FeeBurnError::ChangeDoesNotFit | FeeBurnError::CapacityExceeded) => return Ok(None),
                Err(FeeBurnError::Unavailable(error)) => return Err(error),
                Err(_) => return Err(DurabilityError::InvalidStoredState),
            }
            sqlx::query("INSERT INTO game_horse_rental_receipts(character_id,world_id,occurrence_id,fee_transaction_id,mount_id,rented_at_ms,expires_at_ms)
                VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,$5,$6,$7)")
                .bind(character).bind(world.as_bytes().as_slice()).bind(occurrence.as_slice()).bind(fee.transaction_id.as_slice())
                .bind(i16::try_from(lease.mount_id).map_err(|_| DurabilityError::InvalidStoredState)?)
                .bind(now).bind(lease.expires_at_ms).execute(&mut **tx).await?;
            if let HorseStation::LooseUntil(until) = phase.after_rental(now, draw[1] % 10) {
                sqlx::query("UPDATE game_horse_stations SET loose_until_ms=$2 WHERE world_id=encode($1,'hex')::uuid")
                    .bind(world.as_bytes().as_slice()).bind(until).execute(&mut **tx).await?;
            }
            tx.commit(deadline).await?;
            Ok(Some(lease))
        })).await
    }
}
