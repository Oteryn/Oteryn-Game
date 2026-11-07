use super::*;
pub const GUILD_JOB_PASS_MAX: i64 = 100;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GuildDeadlineOutcome {
    Started,
    AlreadyStarted,
    NoLongerDue,
}

impl DurabilityRoot {
    /// Candidate discovery takes no Guild locks. The worker must recheck under the Guild lock.
    pub async fn guild_deadline_candidates(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: crate::foundation::WorldId,
    ) -> Result<Vec<GuildId>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        Ok(self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                if !crate::durability::runtime_scope_assignment::prove_current_incarnation(&mut tx, &node).await? {
                    return Err(DurabilityError::Unavailable);
                }
                let rows: Vec<String> = sqlx::query_scalar("SELECT guild_id::text FROM game_guilds WHERE world_id=encode($1,'hex')::uuid AND ((state=1 AND formation_deadline<floor(extract(epoch FROM transaction_timestamp())*1000)::bigint) OR (state=2 AND vice_deficit_since IS NOT NULL AND floor(extract(epoch FROM transaction_timestamp())*1000)::numeric-vice_deficit_since::numeric>1209600000)) ORDER BY guild_id LIMIT $2")
                    .bind(world.as_bytes().as_slice()).bind(GUILD_JOB_PASS_MAX).fetch_all(&mut *tx).await?;
                let ids = rows.into_iter().map(|id| {
                    GuildId::from_bytes(uuid_text(&id)?).map_err(|_| DurabilityError::InvalidStoredState)
                }).collect::<std::result::Result<Vec<_>, _>>()?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(ids)
            })
        }).await?)
    }

    /// Only accepted GUILD0 §3.4 start: freeze commands and retain the pending step durably.
    /// Bank payout and Hall disposition must finish before any future terminal step is admitted.
    /// World jobs use current recovery/admission, never a Character session or retained receipt
    /// as authority (GUILDS_V1 §3; GUILD0 §4.1). The node proof attests only current process.
    pub async fn start_guild_deadline_disband(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: crate::foundation::WorldId,
        guild: GuildId,
    ) -> Result<GuildDeadlineOutcome> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    if !crate::durability::runtime_scope_assignment::prove_current_incarnation(
                        &mut tx, &node,
                    )
                    .await?
                    {
                        return Err(DurabilityError::Unavailable);
                    }
                    let outcome = start_deadline_disband(&mut tx, &node, world, guild).await;
                    match outcome {
                        Ok(outcome) => {
                            commit_semantic_transaction(tx, deadline).await?;
                            Ok(Ok(outcome))
                        }
                        Err(GuildError::Unavailable(error)) => Err(error),
                        Err(error) => Ok(Err(error)),
                    }
                })
            })
            .await?
    }

    /// Reload pending starts after a process/session change. This is a bounded delivery query,
    /// not a completion API: no Bank/Hall closure or external work is inferred from a checkpoint.
    pub async fn pending_guild_disbands(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: crate::foundation::WorldId,
    ) -> Result<Vec<GuildId>> {
        self.pending_guild_disbands_after(authority, node, world, None)
            .await
    }

    /// Ordered scan cursor is delivery DATA only. Advance it to the last returned Guild and
    /// wrap on an empty page, so100 waiting Guilds cannot monopolize every scheduler pass.
    pub async fn pending_guild_disbands_after(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: crate::foundation::WorldId,
        after: Option<GuildId>,
    ) -> Result<Vec<GuildId>> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| GuildError::NotCurrent)?;
        let node = node.clone();
        Ok(self.try_issue_semantic_pass()?.run(move |holder, deadline| {
            Box::pin(async move {
                let mut tx = begin_semantic_transaction(holder, deadline).await?;
                assert_recovery_fence(&mut tx, &recovery).await?;
                lock_admission_relations(&mut tx).await?;
                if !crate::durability::runtime_scope_assignment::prove_current_incarnation(&mut tx, &node).await? {
                    return Err(DurabilityError::Unavailable);
                }
                let rows: Vec<String> = sqlx::query_scalar("SELECT j.guild_id::text FROM game_guild_job_checkpoints j JOIN game_guilds g USING(guild_id) WHERE j.world_id=encode($1,'hex')::uuid AND g.world_id=j.world_id AND g.state=3 AND ($2::bytea IS NULL OR j.guild_id>encode($2,'hex')::uuid) ORDER BY j.guild_id LIMIT $3")
                    .bind(world.as_bytes().as_slice()).bind(after.map(|id|id.as_bytes().to_vec())).bind(GUILD_JOB_PASS_MAX).fetch_all(&mut *tx).await?;
                let ids = rows.into_iter().map(|id| {
                    GuildId::from_bytes(uuid_text(&id)?).map_err(|_| DurabilityError::InvalidStoredState)
                }).collect::<std::result::Result<Vec<_>, _>>()?;
                commit_semantic_transaction(tx, deadline).await?;
                Ok(ids)
            })
        }).await?)
    }
}

pub(super) async fn start_deadline_disband(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    world: crate::foundation::WorldId,
    guild: GuildId,
) -> Result<GuildDeadlineOutcome> {
    // Occurrence first, Guild second. Separate candidate transactions preserve the full order.
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('guild-disband-start:'||encode($1,'hex'),0))")
        .bind(guild.as_bytes().as_slice()).execute(&mut **tx).await?;
    let Some(row) = sqlx::query("SELECT world_id::text,state,revision::text,formation_deadline,vice_deficit_since,floor(extract(epoch FROM transaction_timestamp())*1000)::bigint AS now FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(guild.as_bytes().as_slice()).fetch_optional(&mut **tx).await? else {
        return Ok(GuildDeadlineOutcome::NoLongerDue);
    };
    if uuid_text(&row.try_get::<String, _>("world_id")?)? != *world.as_bytes() {
        return Ok(GuildDeadlineOutcome::NoLongerDue);
    }
    let state = match row.try_get::<i16, _>("state")? {
        1 => GuildState::Forming,
        2 => GuildState::Active,
        3 => {
            let exists: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guild_job_checkpoints WHERE guild_id=encode($1,'hex')::uuid AND step=1)")
                .bind(guild.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
            return Ok(if exists {
                GuildDeadlineOutcome::AlreadyStarted
            } else {
                GuildDeadlineOutcome::NoLongerDue
            });
        }
        4 => return Ok(GuildDeadlineOutcome::NoLongerDue),
        _ => return Err(invalid()),
    };
    let job = if state == GuildState::Forming {
        oteryn_gameplay_social_rules::guild::GuildDeadlineJob::Formation
    } else {
        oteryn_gameplay_social_rules::guild::GuildDeadlineJob::ViceDeficit
    };
    let now: i64 = row.try_get("now")?;
    if !job.due(
        state,
        row.try_get("formation_deadline")?,
        row.try_get("vice_deficit_since")?,
        now,
    ) {
        return Ok(GuildDeadlineOutcome::NoLongerDue);
    }
    // A live process may inspect/replay a checkpoint; a new World mutation additionally
    // requires the current World assignment, as every cleanup step already does.
    assert_guild_job_world(tx, node, world).await?;
    let previous = numeric_u64(&row, "revision")?;
    let revision = previous.checked_add(1).ok_or_else(invalid)?;
    let audit = crate::durability::guild_audit::GuildJobAuditProposal {
        version: 1,
        guild_id: *guild.as_bytes(),
        world_id: *world.as_bytes(),
        step: 1,
        reason: job.code(),
        previous_revision: previous,
        revision,
        occurred_at_unix_ms: now,
    }
    .checked_bytes()
    .ok_or_else(invalid)?;
    sqlx::query("INSERT INTO game_guild_job_checkpoints(guild_id,step,world_id,reason,previous_revision,revision,occurred_at) VALUES(encode($1,'hex')::uuid,1,encode($2,'hex')::uuid,$3,$4::text::numeric,$5::text::numeric,$6)")
        .bind(guild.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(job.code())
        .bind(previous.to_string()).bind(revision.to_string()).bind(now).execute(&mut **tx).await?;
    sqlx::query("UPDATE game_guilds SET state=3,revision=$2::text::numeric WHERE guild_id=encode($1,'hex')::uuid")
        .bind(guild.as_bytes().as_slice()).bind(revision.to_string()).execute(&mut **tx).await?;
    sqlx::query("INSERT INTO game_guild_job_event_outbox(guild_id,step,payload,payload_sha256) VALUES(encode($1,'hex')::uuid,1,$2,sha256($2))")
        .bind(guild.as_bytes().as_slice()).bind(&audit).execute(&mut **tx).await?;
    Ok(GuildDeadlineOutcome::Started)
}
