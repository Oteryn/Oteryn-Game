//! Q10=B: the current native Channel owner can retire an independently observed
//! empty room. The exact old visit remains a return obligation for an offline actor.
use super::*;

impl DurabilityRoot {
    /// At most one ACTIVE visit per source room, hence at most thirty per Channel.
    /// Data only: the runtime rechecks an exact visit before any physical cleanup.
    pub async fn active_grizzly_boss_entries(&self,
        authority: &ReconciledCharacterAuthority<'_, '_>, node: &NodeIncarnationProof,
        world: WorldId, channel: ChannelId, generation: ScopeOwnershipGeneration,
    ) -> Result<Vec<GrizzlyBossEntry>> {
        let recovery = authority.record_for(self).map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            if let Err(error) = assert_channel_owner(&mut tx, &recovery, &node, world, channel, generation).await {
                return Ok(Err(owner_error(error)));
            }
            let query = format!("{} WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND state='ACTIVE' ORDER BY boss_id LIMIT 31", select_columns());
            let rows = sqlx::query(sqlx::AssertSqlSafe(query)).bind(world.as_bytes().as_slice())
                .bind(channel.as_bytes().as_slice()).fetch_all(&mut *tx).await?;
            if rows.len() > 30 { return Err(DurabilityError::InvalidStoredState); }
            let entries = rows.iter().map(decode).collect::<StoredResult<Vec<_>>>()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(entries))
        })).await?
    }

    /// A stale timer or entry attempt may never clean a newer visit in the same room.
    /// Native current Channel ownership and the full immutable visit key are checked.
    #[allow(clippy::too_many_arguments)]
    pub async fn current_active_grizzly_boss_entry(&self,
        authority: &ReconciledCharacterAuthority<'_, '_>, node: &NodeIncarnationProof,
        world: WorldId, channel: ChannelId, generation: ScopeOwnershipGeneration,
        entry: &GrizzlyBossEntry,
    ) -> Result<Option<GrizzlyBossEntry>> {
        if entry.world_id != world || entry.channel_id != channel { return Err(CharacterProgressionError::InvalidInput); }
        let recovery = authority.record_for(self).map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone(); let entry = entry.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            if let Err(error) = assert_channel_owner(&mut tx, &recovery, &node, world, channel, generation).await {
                return Ok(Err(owner_error(error)));
            }
            let query = format!("{} WHERE character_id=encode($1,'hex')::uuid AND cause_id=encode($2,'hex')::uuid AND cause_ordinal=$3::text::numeric AND boss_id=$4 AND world_id=encode($5,'hex')::uuid AND channel_id=encode($6,'hex')::uuid AND state='ACTIVE'", select_columns());
            let row = sqlx::query(sqlx::AssertSqlSafe(query)).bind(entry.character_id.as_bytes().as_slice())
                .bind(entry.cause.game_session_id().as_bytes().as_slice()).bind(entry.cause.command_id().get().to_string())
                .bind(i16::from(entry.boss_id)).bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
                .fetch_optional(&mut *tx).await?;
            let current = row.as_ref().map(decode).transpose()?;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(current))
        })).await?
    }

    /// The runtime holds the real spell lane and an exact, empty room census through
    /// cleanup and this write. This is the existing visit's state change, not a new
    /// quest transition, timer or reward owner. Exact retirement replay is a no-op.
    #[allow(clippy::too_many_arguments)]
    pub async fn retire_vacated_grizzly_boss_entry(&self,
        authority: &ReconciledCharacterAuthority<'_, '_>, node: &NodeIncarnationProof,
        permit: &SpellLanePermit, world: WorldId, channel: ChannelId,
        generation: ScopeOwnershipGeneration, entry: &GrizzlyBossEntry,
    ) -> Result<bool> {
        if entry.world_id != world || entry.channel_id != channel {
            return Err(CharacterProgressionError::InvalidInput);
        }
        permit.check_channel(world, channel).map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let recovery = authority.record_for(self).map_err(|_| CharacterProgressionError::AuthorityRejected)?;
        let node = node.clone(); let entry = entry.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            if let Err(error) = assert_channel_owner(&mut tx, &recovery, &node, world, channel, generation).await {
                return Ok(Err(owner_error(error)));
            }
            let changed = sqlx::query("UPDATE game_character_grizzly_boss_entries SET state='RETURN_PENDING' WHERE character_id=encode($1,'hex')::uuid AND cause_id=encode($2,'hex')::uuid AND cause_ordinal=$3::text::numeric AND boss_id=$4 AND world_id=encode($5,'hex')::uuid AND channel_id=encode($6,'hex')::uuid AND state='ACTIVE'")
                .bind(entry.character_id.as_bytes().as_slice()).bind(entry.cause.game_session_id().as_bytes().as_slice())
                .bind(entry.cause.command_id().get().to_string()).bind(i16::from(entry.boss_id))
                .bind(world.as_bytes().as_slice()).bind(channel.as_bytes().as_slice())
                .execute(&mut *tx).await?.rows_affected() == 1;
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(changed))
        })).await?
    }
}
