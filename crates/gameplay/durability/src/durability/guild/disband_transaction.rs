use super::*;

/// A current, locked physical transaction, not a retained disband receipt. Construction is
/// private and requires the actual Root-bound recovery capability and independently current
/// node/World assignment. Every consumer rechecks these facts in the SAME transaction.
pub struct GuildDisbandTransaction {
    pub(super) guild: GuildId,
    pub(super) world: crate::foundation::WorldId,
    pub(super) revision: u64,
    pub(super) xact_id: String,
    pub(super) recovery: crate::character_recovery_fence::CharacterRecoveryFenceV1,
    pub(super) node: NodeIncarnationProof,
}

impl GuildDisbandTransaction {
    pub fn guild_id(&self) -> GuildId {
        self.guild
    }

    pub fn world_id(&self) -> crate::foundation::WorldId {
        self.world
    }

    pub async fn assert_current(&self, tx: &mut Transaction<'_, Postgres>) -> Result<()> {
        let xid: String = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
            .fetch_one(&mut **tx)
            .await?;
        if xid != self.xact_id {
            return Err(GuildError::NotCurrent);
        }
        assert_recovery_fence(tx, &self.recovery).await?;
        lock_admission_relations(tx).await?;
        assert_guild_job_world(tx, &self.node, self.world).await?;
        let current: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND state=3 AND revision=$3::text::numeric)")
            .bind(self.guild_id().as_bytes().as_slice()).bind(self.world_id().as_bytes().as_slice())
            .bind(self.revision.to_string()).fetch_one(&mut **tx).await?;
        if !current {
            return Err(GuildError::NotCurrent);
        }
        Ok(())
    }
}

pub(super) async fn assert_guild_job_world(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    world: crate::foundation::WorldId,
) -> Result<()> {
    if !crate::durability::runtime_scope_assignment::prove_current_incarnation(tx, node).await? {
        return Err(GuildError::NotCurrent);
    }
    // Same genuine World assignment source used by the native Hall World-job owner.
    // A node incarnation alone never grants authority over the requested World.
    let fact = node.fact();
    let current = sqlx::query("SELECT 1 FROM game_runtime_scope_assignments WHERE world_id=encode($1,'hex')::uuid AND scope_kind=1 AND state=1 AND holder_node_id=encode($2,'hex')::uuid AND holder_registration_revision=$3::text::numeric ORDER BY scope_key LIMIT 1 FOR SHARE")
        .bind(world.as_bytes().as_slice()).bind(fact.node_id().as_bytes().as_slice())
        .bind(fact.registration_revision().to_string()).fetch_optional(&mut **tx).await?;
    if current.is_none() {
        return Err(GuildError::NotCurrent);
    }
    Ok(())
}

/// The actual production constructor is also used by sibling native qualification. It cannot
/// mint from a stored checkpoint, supplied xid, session-like DATA, or another Root's capability.
pub async fn lock_guild_disband_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    authority: &ReconciledCharacterAuthority<'_, '_>,
    node: &NodeIncarnationProof,
    request: &GuildDisbandJobRequest,
) -> Result<Option<GuildDisbandTransaction>> {
    let recovery = authority
        .record_for(root)
        .map_err(|_| GuildError::NotCurrent)?;
    lock_guild_disband_with_recovery(tx, recovery, node, request).await
}

pub(super) async fn lock_guild_disband_with_recovery(
    tx: &mut Transaction<'_, Postgres>,
    recovery: crate::character_recovery_fence::CharacterRecoveryFenceV1,
    node: &NodeIncarnationProof,
    request: &GuildDisbandJobRequest,
) -> Result<Option<GuildDisbandTransaction>> {
    assert_recovery_fence(tx, &recovery).await?;
    lock_admission_relations(tx).await?;
    assert_guild_job_world(tx, node, request.world_id).await?;
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended('guild-disband-start:'||encode($1,'hex'),0))")
        .bind(request.guild_id.as_bytes().as_slice()).execute(&mut **tx).await?;
    let row = sqlx::query("SELECT world_id::text,state,revision::text FROM game_guilds WHERE guild_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(request.guild_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else { return Ok(None) };
    if uuid_text(&row.try_get::<String, _>("world_id")?)? != *request.world_id.as_bytes()
        || row.try_get::<i16, _>("state")? != 3
    {
        return Ok(None);
    }
    let checkpoint: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_guild_job_checkpoints WHERE guild_id=encode($1,'hex')::uuid AND step=1 AND world_id=encode($2,'hex')::uuid AND revision<=$3::text::numeric)")
        .bind(request.guild_id.as_bytes().as_slice()).bind(request.world_id.as_bytes().as_slice())
        .bind(numeric_u64(&row,"revision")?.to_string()).fetch_one(&mut **tx).await?;
    if !checkpoint {
        return Err(invalid());
    }
    // The locked Guild serializes cleanup with commands and other workers. Each step locks
    // only its bounded row set; it never locks every member/invitation/account on every pass.
    let xact_id = sqlx::query_scalar("SELECT pg_current_xact_id()::text")
        .fetch_one(&mut **tx)
        .await?;
    Ok(Some(GuildDisbandTransaction {
        guild: request.guild_id,
        world: request.world_id,
        revision: numeric_u64(&row, "revision")?,
        xact_id,
        recovery,
        node: node.clone(),
    }))
}
