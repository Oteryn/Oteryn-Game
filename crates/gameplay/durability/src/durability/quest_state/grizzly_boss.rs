//! The existing quest receipt earns one durable Grizzly room visit. Native Channel
//! ownership controls cleanup; an offline player's return remains pending afterward.

use super::*;
use crate::foundation::{ChannelId, MovementLocalPosition, ScopeOwnershipGeneration, WorldId};
use super::super::boss_encounter::{BossClockError, assert_channel_owner};
use super::super::quest_world_callback::QuestWorldContext;
use super::super::spell_owner_commit::SpellLanePermit;
use oteryn_gameplay_quests::quest::grizzly_adams::{self as rules, GrizzlyAction};

mod storage;
mod vacancy;
use self::storage::{decode, select_columns};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrizzlyBossEntryContext {
    pub boss_id: u8,
    /// The actual pre-entry position, independently checked under the runtime lane.
    pub exit: MovementLocalPosition,
    pub world: QuestWorldContext,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GrizzlyBossEntryState { Active, ReturnPending }

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GrizzlyBossEntry {
    pub character_id: CharacterId,
    pub cause: CommandRef,
    pub boss_id: u8,
    pub world_id: WorldId,
    pub channel_id: ChannelId,
    pub scope_generation: ScopeOwnershipGeneration,
    pub character_lease_generation: u64,
    pub exit: MovementLocalPosition,
    pub entered_at_unix_ms: i64,
    pub deadline_unix_ms: i64,
    pub state: GrizzlyBossEntryState,
    pub expired: bool,
}

impl DurabilityRoot {
    #[allow(clippy::too_many_arguments)]
    pub async fn commit_character_grizzly_boss_entry(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        cause: QuestCause,
        catalogue: Arc<QuestStateCatalogue>,
        permit: &SpellLanePermit,
        context: GrizzlyBossEntryContext,
    ) -> Result<QuestTransitionOutcome> {
        let action = GrizzlyAction::EnterBoss(context.boss_id);
        self.commit_quest_inner::<_, 0>(authority, node, fence,
            QuestTransitionRequest { transition_key: action.transition_key(), cause },
            catalogue, None, Some(action), Some((permit, context)), None, &grizzly::NoGrizzlyPremium, None).await
    }

    pub async fn reconcile_grizzly_boss_entry(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
        cause: QuestCause,
        context: GrizzlyBossEntryContext,
    ) -> Result<Option<QuestTransitionOutcome>> {
        self.reconcile_grizzly_inner(authority, character, cause,
            GrizzlyAction::EnterBoss(context.boss_id), Some(context), None).await
    }

    /// Historical data only. Admission returns the player only if the current actor is
    /// still inside the independently qualified room; otherwise it acknowledges outside.
    pub async fn pending_grizzly_boss_entry(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        character: CharacterId,
    ) -> Result<Option<GrizzlyBossEntry>> {
        let recovery = authority.record_for(self).map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            let query = format!("{} WHERE character_id=encode($1,'hex')::uuid AND state IN ('ACTIVE','RETURN_PENDING')", select_columns());
            let row = sqlx::query(sqlx::AssertSqlSafe(query))
                .bind(character.as_bytes().as_slice()).fetch_optional(&mut *tx).await?;
            let value = row.as_ref().map(decode).transpose()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(value))
        })).await?
    }

    /// The current native Channel owner sees expired visits and visits from a prior
    /// owner generation. A process restart never extends a persisted deadline.
    pub async fn due_grizzly_boss_entries(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: WorldId,
        channel: ChannelId,
        generation: ScopeOwnershipGeneration,
    ) -> Result<Vec<GrizzlyBossEntry>> {
        let recovery = authority.record_for(self).map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            if let Err(error) = assert_channel_owner(&mut tx, &recovery, &node, world, channel, generation).await {
                return Ok(Err(owner_error(error)));
            }
            let query = format!("{} WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state='ACTIVE' AND (deadline_at<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint OR scope_generation<>$3::text::numeric) ORDER BY boss_id LIMIT 31", select_columns());
            let rows = sqlx::query(sqlx::AssertSqlSafe(query))
                .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
                .bind(generation.get().to_string()).fetch_all(&mut *tx).await?;
            if rows.len() > 30 { return Err(DurabilityError::InvalidStoredState); }
            let values = rows.iter().map(decode).collect::<StoredResult<Vec<_>>>()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(values))
        })).await?
    }

    /// Called after exact runtime monsters for this visit have been cleared. Current
    /// Channel authority releases the room even when its player is offline, retaining
    /// the return obligation. The original visit identity is never rewritten.
    #[allow(clippy::too_many_arguments)]
    pub async fn expire_grizzly_boss_entry(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: WorldId,
        channel: ChannelId,
        generation: ScopeOwnershipGeneration,
        entry: &GrizzlyBossEntry,
    ) -> Result<bool> {
        if entry.world_id != world || entry.channel_id != channel {
            return Err(CharacterProgressionError::InvalidInput);
        }
        let recovery = authority.record_for(self).map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        let entry = entry.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            if let Err(error) = assert_channel_owner(&mut tx, &recovery, &node, world, channel, generation).await {
                return Ok(Err(owner_error(error)));
            }
            let changed = sqlx::query("UPDATE game_character_grizzly_boss_entries SET state='RETURN_PENDING' WHERE character_id=encode($1,'hex')::uuid AND cause_id=encode($2,'hex')::uuid AND cause_ordinal=$3::text::numeric AND boss_id=$4 AND world_id=encode($5,'hex')::uuid AND channel_id=encode($6,'hex')::uuid AND state='ACTIVE' AND (deadline_at<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint OR scope_generation<>$7::text::numeric)")
                .bind(entry.character_id.as_bytes().as_slice()).bind(entry.cause.game_session_id().as_bytes().as_slice())
                .bind(entry.cause.command_id().get().to_string()).bind(i16::from(entry.boss_id))
                .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
                .bind(generation.get().to_string()).execute(&mut *tx).await?.rows_affected() == 1;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(changed))
        })).await?
    }

    /// The current Character owner acknowledges a completed return, or proves its
    /// actor already outside the room. Quest storage remains donor stage 2 until report.
    pub async fn close_grizzly_boss_entry(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterGameplayFence,
        cause: CommandRef,
        position: super::super::character_last_position::FinalLastPosition,
    ) -> Result<bool> {
        super::super::character_last_position::validate_fence(&fence, &position.record)?;
        if position.write_sequence == 0 { return Err(CharacterProgressionError::InvalidInput); }
        let recovery = authority.record_for(self).map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if let Err(error) = super::super::character_progression::assert_item_only_fence(&mut tx, &fence.item_fence(), &node).await? {
                return Ok(Err(error));
            }
            let open = sqlx::query("SELECT 1 FROM game_character_grizzly_boss_entries WHERE character_id=encode($1,'hex')::uuid AND cause_id=encode($2,'hex')::uuid AND cause_ordinal=$3::text::numeric AND state IN ('ACTIVE','RETURN_PENDING') FOR UPDATE")
                .bind(fence.character_id.as_bytes().as_slice()).bind(cause.game_session_id().as_bytes().as_slice())
                .bind(cause.command_id().get().to_string()).fetch_optional(&mut *tx).await?.is_some();
            if !open {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(Ok(false));
            }
            // Reuse CHAR-POSITION's sole ordered upsert. Persisting the verified outside
            // position and closing its return obligation are one physical transaction.
            use super::super::character_last_position::{self as positions, LastPositionKey, LastPositionWrite};
            let saved = positions::upsert_last_position(&mut tx, fence.character_id.as_bytes(),
                LastPositionKey { character_lease_generation: fence.character_lease_generation,
                    write_sequence: position.write_sequence }, &position.record).await?;
            let position_known = match saved {
                LastPositionWrite::Written => true,
                LastPositionWrite::PendingObligation => false,
                LastPositionWrite::Superseded => positions::load_last_position(&mut tx, fence.character_id)
                    .await?.is_some_and(|stored| stored.record.world_id == position.record.world_id
                        && stored.record.cell == position.record.cell
                        && stored.record.coordinate_frame_digest == position.record.coordinate_frame_digest
                        && stored.record.map_revision_digest == position.record.map_revision_digest
                        && stored.record.server_artifact_digest == position.record.server_artifact_digest),
            };
            if !position_known {
                commit_semantic_transaction(tx, deadline).await?;
                return Ok(Ok(false));
            }
            let changed = sqlx::query("UPDATE game_character_grizzly_boss_entries SET state='CLOSED' WHERE character_id=encode($1,'hex')::uuid AND cause_id=encode($2,'hex')::uuid AND cause_ordinal=$3::text::numeric AND state IN ('ACTIVE','RETURN_PENDING')")
                .bind(fence.character_id.as_bytes().as_slice()).bind(cause.game_session_id().as_bytes().as_slice())
                .bind(cause.command_id().get().to_string()).execute(&mut *tx).await?.rows_affected() == 1;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(changed))
        })).await?
    }
}

fn owner_error(error: BossClockError) -> CharacterProgressionError {
    match error {
        BossClockError::AuthorityRejected => CharacterProgressionError::AuthorityRejected,
        BossClockError::Unavailable(error) => CharacterProgressionError::Unavailable(error),
        _ => CharacterProgressionError::Unavailable(DurabilityError::InvalidStoredState),
    }
}

pub(super) fn bind_context(binding: &mut [u8; 33], context: &GrizzlyBossEntryContext) -> Result<()> {
    if rules::boss(context.boss_id).is_none() || !(0..=15).contains(&context.exit.floor)
        || !(0..=65535).contains(&context.exit.x) || !(0..=65535).contains(&context.exit.y)
        || context.world.placement_context.len() != 32 {
        return Err(CharacterProgressionError::InvalidInput);
    }
    let bytes = serde_json::to_vec(&context.world).map_err(|_| CharacterProgressionError::InvalidInput)?;
    let digest = Sha256::new().chain_update(*binding).chain_update([context.boss_id])
        .chain_update(context.exit.x.to_be_bytes()).chain_update(context.exit.y.to_be_bytes())
        .chain_update(context.exit.floor.to_be_bytes()).chain_update(bytes).finalize();
    binding[1..].copy_from_slice(&digest);
    Ok(())
}

pub(super) async fn admit(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    permit: &SpellLanePermit,
    fence: CurrentCharacterGameplayFence,
    context: &GrizzlyBossEntryContext,
) -> StoredResult<bool> {
    if !super::super::quest_world_callback::context_is_current(tx, permit, fence, &context.world).await? {
        return Ok(false);
    }
    let RuntimeScopeRefV1::Channel { world_id, channel_id } = fence.runtime_scope else { return Ok(false); };
    // Every quest writer already holds admission relations. Unique indexes also guard
    // these room/Character exclusions; an expired row awaits exact runtime cleanup.
    let busy: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_character_grizzly_boss_entries WHERE (character_id=encode($1,'hex')::uuid AND state IN ('ACTIVE','RETURN_PENDING')) OR (world_id=encode($2,'hex')::uuid AND channel_id=encode($3,'hex')::uuid AND boss_id=$4 AND state='ACTIVE'))")
        .bind(fence.character_id.as_bytes().as_slice()).bind(world_id.as_bytes().as_slice())
        .bind(channel_id.as_bytes().as_slice()).bind(i16::from(context.boss_id)).fetch_one(&mut **tx).await?;
    Ok(!busy)
}

pub(super) async fn earn(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    request: &QuestTransitionRequest,
    context: &GrizzlyBossEntryContext,
) -> StoredResult<()> {
    let RuntimeScopeRefV1::Channel { world_id, channel_id } = fence.runtime_scope else { return Err(DurabilityError::InvalidStoredState); };
    let (cause, ordinal) = request.cause.key();
    let boss = rules::boss(context.boss_id).ok_or(DurabilityError::InvalidStoredState)?;
    sqlx::query("INSERT INTO game_character_grizzly_boss_entries(character_id,cause_id,cause_ordinal,transition_key,boss_id,boss_track,world_id,channel_id,scope_generation,lease_generation,exit_x,exit_y,exit_floor,content_revision,frame_digest,entered_at,deadline_at) SELECT encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3::text::numeric,$4,$5,$6,encode($7,'hex')::uuid,encode($8,'hex')::uuid,$9::text::numeric,$10::text::numeric,$11,$12,$13,$14,$15,now_ms,now_ms+600000 FROM (SELECT floor(extract(epoch FROM clock_timestamp())*1000)::bigint AS now_ms) clock")
        .bind(fence.character_id.as_bytes().as_slice()).bind(cause.as_slice()).bind(ordinal.to_string())
        .bind(&request.transition_key).bind(i16::from(context.boss_id)).bind(rules::storage_track(boss.storage))
        .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice())
        .bind(fence.scope_ownership_generation.get().to_string()).bind(fence.character_lease_generation.to_string())
        .bind(context.exit.x).bind(context.exit.y).bind(context.exit.floor)
        .bind(&context.world.content_revision).bind(&context.world.placement_context).execute(&mut **tx).await?;
    Ok(())
}
