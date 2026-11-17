use super::*;
use crate::durability::character_progression::assert_runtime_scope_fence;
use crate::foundation::{RuntimeScopeRefV1, ScopeOwnershipGeneration};
use oteryn_gameplay_world_change::thornfire::{COMPLETE_PROGRESS, at_save};

/// Captures the request for this epoch in the reset finalization transaction.
pub(in crate::durability::world_event) async fn record_save(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<(), DurabilityError> {
    let progress: Option<i64> = sqlx::query_scalar(
        "SELECT elf_homes FROM game_thornfire_progress WHERE world_id=encode($1,'hex')::uuid AND reset_epoch<$2::text::numeric FOR UPDATE",
    )
    .bind(world.as_bytes().as_slice())
    .bind(epoch.to_string())
    .fetch_optional(&mut **tx)
    .await?;
    let (progress, requested) = at_save(progress.unwrap_or_default());
    sqlx::query("INSERT INTO game_thornfire_progress(world_id,reset_epoch,elf_homes,requested_at_save) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4) ON CONFLICT(world_id) DO UPDATE SET reset_epoch=EXCLUDED.reset_epoch,elf_homes=EXCLUDED.elf_homes,requested_at_save=EXCLUDED.requested_at_save WHERE game_thornfire_progress.reset_epoch<EXCLUDED.reset_epoch")
        .bind(world.as_bytes().as_slice())
        .bind(epoch.to_string())
        .bind(progress)
        .bind(requested)
        .execute(&mut **tx)
        .await?;
    Ok(())
}

impl DurabilityRoot {
    /// A native credited kill marks one exact spawn home. Replays and respawns from
    /// the same home are idempotent within the current collection period.
    pub async fn credit_thornfire_elf(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        epoch: u64,
        home_bit: i64,
    ) -> Result<(), DurabilityError> {
        if !matches!(scope, RuntimeScopeRefV1::Channel { .. })
            || !(1..=COMPLETE_PROGRESS).contains(&home_bit)
            || home_bit.count_ones() != 1
        {
            return Err(DurabilityError::InvalidStoredState);
        }
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !assert_runtime_scope_fence(&mut tx, scope, generation, &node).await? {
                return Ok(());
            }
            sqlx::query("INSERT INTO game_thornfire_progress(world_id,reset_epoch,elf_homes,requested_at_save)
                SELECT a.world_id,a.reset_epoch,$3,false FROM game_world_reset_active a
                WHERE a.world_id=encode($1,'hex')::uuid AND a.reset_epoch=$2::text::numeric
                AND NOT EXISTS (SELECT 1 FROM game_world_reset_jobs j WHERE j.world_id=a.world_id AND j.phase='RETIRING')
                ON CONFLICT(world_id) DO UPDATE SET elf_homes=game_thornfire_progress.elf_homes | EXCLUDED.elf_homes
                WHERE game_thornfire_progress.reset_epoch=EXCLUDED.reset_epoch")
                .bind(scope.world_id().as_bytes().as_slice())
                .bind(epoch.to_string())
                .bind(home_bit)
                .execute(&mut *tx)
                .await?;
            commit_semantic_transaction(tx, deadline).await
        })).await
    }

    /// The burning-phase owner can consume the decision captured for the active save.
    pub async fn thornfire_requested_at_save(
        &self,
        world: WorldId,
        epoch: u64,
    ) -> Result<bool, DurabilityError> {
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let requested: Option<bool> = sqlx::query_scalar("SELECT p.requested_at_save FROM game_thornfire_progress p JOIN game_world_reset_active a USING(world_id,reset_epoch) WHERE p.world_id=encode($1,'hex')::uuid AND p.reset_epoch=$2::text::numeric")
                .bind(world.as_bytes().as_slice())
                .bind(epoch.to_string())
                .fetch_optional(&mut *tx)
                .await?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(requested.unwrap_or_default())
        })).await
    }
}

/// The hash excludes Channel and Character. The first committed USE of this
/// World fire revision claims a winning draw, even if other Channels repeat it.
pub(crate) async fn record_wolf_draw(
    tx: &mut Transaction<'_, Postgres>,
    fence: super::super::CurrentCharacterItemFence,
    command: crate::foundation::CommandRef,
    scope: &crate::durability::terrain_tool::TerrainScope,
    mutation: &crate::durability::terrain_tool::TerrainMutation,
    event: &str,
) -> Result<(), DurabilityError> {
    let RuntimeScopeRefV1::Channel { world_id, channel_id } = fence.runtime_scope else {
        return Err(DurabilityError::InvalidStoredState);
    };
    let placement = mutation.placement.get(32..)
        .and_then(|bytes| <[u8; 8]>::try_from(bytes).ok())
        .map(i64::from_be_bytes)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let digest = Sha256::new()
        .chain_update(b"oteryn.world-change.thornfire.wolf.v1")
        .chain_update(world_id.as_bytes())
        .chain_update(scope.reset_epoch.to_be_bytes())
        .chain_update(placement.to_be_bytes())
        .chain_update(mutation.expected_revision.to_be_bytes())
        .finalize();
    let draw = |bytes: &[u8]| {
        <[u8; 8]>::try_from(bytes).map(u64::from_be_bytes)
            .map_err(|_| DurabilityError::InvalidStoredState)
    };
    if let Some((x, y, floor)) = oteryn_gameplay_world_change::thornfire::wolf_position(
        draw(&digest[..8])?, draw(&digest[8..16])?,
    ) {
        sqlx::query("INSERT INTO game_thornfire_wolf_spawns(world_id,reset_epoch,fire_placement,fire_revision,channel_id,scope_generation,game_session_id,command_id,spawn_id,x,y,floor)
            VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4,encode($5,'hex')::uuid,$6::text::numeric,encode($7,'hex')::uuid,$8::text::numeric,$9::uuid,$10,$11,$12)
            ON CONFLICT(world_id,reset_epoch,fire_placement,fire_revision) DO NOTHING")
            .bind(world_id.as_bytes().as_slice()).bind(scope.reset_epoch.to_string())
            .bind(placement).bind(mutation.expected_revision).bind(channel_id.as_bytes().as_slice())
            .bind(fence.scope_ownership_generation.get().to_string())
            .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
            .bind(event).bind(x).bind(y).bind(floor).execute(&mut **tx).await?;
    }
    Ok(())
}

impl DurabilityRoot {
    /// Reconcile the committed winning USE with the still-current Channel owner.
    /// A repeated projection uses the same runtime occurrence even after an unknown commit.
    #[allow(clippy::too_many_arguments, reason = "keep current scope ownership and original USE separate")]
    pub async fn project_thornfire_wolf<C: Send>(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: super::super::CurrentCharacterItemFence,
        command: crate::foundation::CommandRef,
        context: &mut C,
        place: fn(&mut C, [u8; 16], (i32, i32, i16)) -> Result<(), DurabilityError>,
    ) -> Result<(), DurabilityError> {
        let RuntimeScopeRefV1::Channel { world_id, channel_id } = fence.runtime_scope else {
            return Ok(());
        };
        let recovery = authority.record_for(self).map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run_with_context(context, move |holder, deadline, context| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !assert_runtime_scope_fence(&mut tx, fence.runtime_scope, fence.scope_ownership_generation, &node).await? {
                return Ok(());
            }
            let pending: Option<(Vec<u8>, i32, i32, i16)> = sqlx::query_as("SELECT uuid_send(s.spawn_id),s.x,s.y,s.floor FROM game_thornfire_wolf_spawns s JOIN game_world_reset_active a USING(world_id,reset_epoch)
                WHERE s.world_id=encode($1,'hex')::uuid AND s.channel_id=encode($2,'hex')::uuid AND s.scope_generation=$3::text::numeric
                  AND s.game_session_id=encode($4,'hex')::uuid AND s.command_id=$5::text::numeric AND NOT s.realized
                  AND NOT EXISTS (SELECT 1 FROM game_world_reset_jobs j WHERE j.world_id=s.world_id AND j.phase='RETIRING')
                FOR UPDATE OF s")
                .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
                .bind(fence.scope_ownership_generation.get().to_string())
                .bind(command.game_session_id().as_bytes().as_slice()).bind(command.command_id().get().to_string())
                .fetch_optional(&mut *tx).await?;
            if let Some((id, x, y, floor)) = pending {
                let occurrence = id.as_slice().try_into().map_err(|_| DurabilityError::InvalidStoredState)?;
                place(context, occurrence, (x, y, floor))?;
                sqlx::query("UPDATE game_thornfire_wolf_spawns SET realized=true WHERE game_session_id=encode($1,'hex')::uuid AND command_id=$2::text::numeric")
                    .bind(command.game_session_id().as_bytes().as_slice())
                    .bind(command.command_id().get().to_string()).execute(&mut *tx).await?;
            }
            commit_semantic_transaction(tx, deadline).await
        })).await
    }
}
