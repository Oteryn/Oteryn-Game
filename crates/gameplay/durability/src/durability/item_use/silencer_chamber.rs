//! The carried chamber burn and any resulting raid firing share a transaction.
use super::*;
use oteryn_gameplay_encounter_rules::raid::source_silencer_plateau::{
    CHAMBER_KEY, COOLDOWN_MS, RAID_KEY, successful_use,
};

pub const CAUSE: u32 = 2961;

#[derive(Clone, Copy, Debug)]
pub struct PlateauWorld {
    pub reset_epoch: u64,
    pub bundle_digest: [u8; 32],
}

impl DurabilityRoot {
    pub async fn commit_silencer_chamber_use(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        request: ItemUseRequest,
        plateau: PlateauWorld,
    ) -> Result<ItemUseOutcome> {
        let RuntimeScopeRefV1::Channel { world_id, channel_id } = fence.runtime_scope else {
            return Err(ItemUseError::AuthorityRejected);
        };
        if request.kind != ItemUseKind::SilencerChamber
            || request.source_definition.production_key != CHAMBER_KEY
            || plateau.reset_epoch == 0
            || plateau.bundle_digest == [0; 32]
        {
            return Err(ItemUseError::InvalidInput);
        }
        request.validate()?;
        let recovery = authority.record_for(self).map_err(|_| ItemUseError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_type2_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            match replay(&mut tx, &fence, &request).await {
                Ok(Some(outcome)) => {
                    tx.commit(deadline).await?;
                    return Ok(Ok(outcome));
                }
                Ok(None) => {}
                Err(ItemUseError::Unavailable(error)) => return Err(error),
                Err(error) => return Ok(Err(error)),
            }
            match assert_item_only_fence(&mut tx, &fence, &node).await? {
                Ok(root) if root.content_revision == request.content_revision
                    && root.ruleset_revision == request.ruleset_revision => {}
                Ok(_) => return Ok(Err(ItemUseError::StaleState)),
                Err(CharacterProgressionError::Unavailable(error)) => return Err(error),
                Err(_) => return Ok(Err(ItemUseError::AuthorityRejected)),
            }

            sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('oteryn:raid-world:' || encode($1,'hex'),0))")
                .bind(world_id.as_bytes().as_slice()).execute(&mut **tx).await?;
            let current: bool = sqlx::query_scalar(
                "SELECT game_world_reset_map_epoch_current(encode($1,'hex')::uuid,$2::text::numeric)
                    AND EXISTS(SELECT 1 FROM game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch)
                    WHERE a.world_id=encode($1,'hex')::uuid AND e.target_digest=$3)")
                .bind(world_id.as_bytes().as_slice()).bind(plateau.reset_epoch.to_string())
                .bind(plateau.bundle_digest.as_slice()).fetch_one(&mut **tx).await?;
            if !current {
                return Ok(Err(ItemUseError::StaleState));
            }

            let now: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint")
                .fetch_one(&mut **tx).await?;
            sqlx::query("UPDATE game_world_raid_firings SET state=3 WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2 AND reset_epoch<>$3::text::numeric AND state=1")
                .bind(world_id.as_bytes().as_slice()).bind(RAID_KEY).bind(plateau.reset_epoch.to_string())
                .execute(&mut **tx).await?;
            let cooling: bool = sqlx::query_scalar(
                "SELECT EXISTS(SELECT 1 FROM game_world_raid_firings
                 WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2 AND reset_epoch=$3::text::numeric
                   AND (state=1 OR (state=2 AND deadline_ms>$4)))")
                .bind(world_id.as_bytes().as_slice()).bind(RAID_KEY).bind(plateau.reset_epoch.to_string())
                .bind(now).fetch_one(&mut **tx).await?;
            if cooling {
                return Ok(Err(ItemUseError::Cooldown));
            }

            let tuple = tx.tuple();
            let outcome = match write(&mut tx, &fence, &request, tuple).await {
                Ok(outcome @ ItemUseOutcome::Written { .. }) => outcome,
                Ok(ItemUseOutcome::AlreadyCommitted { .. }) => return Ok(Err(ItemUseError::ConflictingCommand)),
                Err(ItemUseError::Unavailable(error)) => return Err(error),
                Err(error) => return Ok(Err(error)),
            };
            let fired = successful_use(plateau.bundle_digest, *fence.game_session_id.as_bytes(), request.command_id)
                .map_err(|_| DurabilityError::InvalidStoredState)?;
            if fired {
                let key = crate::content::ProductionKey::new(RAID_KEY)
                    .map_err(|_| DurabilityError::InvalidStoredState)?;
                super::super::raid_schedule::initialize(&mut tx, &recovery, world_id, &key, plateau.reset_epoch, now)
                    .await.map_err(|_| DurabilityError::Unavailable)?;
                sqlx::query("INSERT INTO game_world_raid_schedule_sources(world_id,raid_key,source_kind)
                             VALUES(encode($1,'hex')::uuid,$2,4) ON CONFLICT DO NOTHING")
                    .bind(world_id.as_bytes().as_slice()).bind(RAID_KEY).execute(&mut **tx).await?;
                let until = now.checked_add(i64::from(COOLDOWN_MS)).ok_or(DurabilityError::InvalidStoredState)?;
                sqlx::query("INSERT INTO game_world_raid_firings(world_id,raid_key,ordinal,firing_id,reset_epoch,fired_at_ms,deadline_ms,bundle_digest,state)
                             SELECT world_id,raid_key,next_firing_ordinal,encode($3,'hex')::uuid,$4::text::numeric,$5,$6,$7,1
                             FROM game_world_raid_schedules WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2")
                    .bind(world_id.as_bytes().as_slice()).bind(RAID_KEY).bind(request.transaction_id.as_slice())
                    .bind(plateau.reset_epoch.to_string()).bind(now).bind(until).bind(plateau.bundle_digest.as_slice())
                    .execute(&mut **tx).await?;
                sqlx::query("UPDATE game_world_raid_schedules SET revision=revision+1,reset_epoch=$3::text::numeric,
                             next_firing_ordinal=next_firing_ordinal+1,next_check_at_ms=$4,last_fired_at_ms=$5
                             WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2")
                    .bind(world_id.as_bytes().as_slice()).bind(RAID_KEY).bind(plateau.reset_epoch.to_string())
                    .bind(until).bind(now).execute(&mut **tx).await?;
                sqlx::query("INSERT INTO game_world_raid_targets(firing_id,channel_id,owner_generation)
                             VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric)")
                    .bind(request.transaction_id.as_slice()).bind(channel_id.as_bytes().as_slice())
                    .bind(fence.scope_ownership_generation.get().to_string()).execute(&mut **tx).await?;
            }
            sqlx::query("INSERT INTO game_silencer_chamber_uses(transaction_id,firing_id)
                         VALUES(encode($1,'hex')::uuid,CASE WHEN $2 THEN encode($1,'hex')::uuid ELSE NULL END)")
                .bind(request.transaction_id.as_slice()).bind(fired).execute(&mut **tx).await?;
            tx.commit(deadline).await?;
            Ok(Ok(outcome))
        })).await?
    }
}
