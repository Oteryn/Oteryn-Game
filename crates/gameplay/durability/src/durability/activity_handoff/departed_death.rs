//! Recovery proof for the existing death-item owner. A lost runtime does not
//! forgive selected item losses or move their Ground to the origin Channel.
use super::super::spell_owner_commit::SpellLanePermit;
use super::*;

/// A read-only proof of a retained death whose exact native Instance owner has
/// been lost. Each item transaction must independently revalidate it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DepartedActivityDeath {
    source: CurrentCharacterItemFence,
    occurrence: [u8; 16],
    handoff_id: [u8; 16],
    origin_scope: RuntimeScopeRefV1,
    origin_generation: ScopeOwnershipGeneration,
}

impl DepartedActivityDeath {
    pub const fn source(&self) -> CurrentCharacterItemFence {
        self.source
    }
    pub const fn occurrence(&self) -> [u8; 16] {
        self.occurrence
    }
    pub const fn handoff_id(&self) -> [u8; 16] {
        self.handoff_id
    }
    pub const fn origin_scope(&self) -> RuntimeScopeRefV1 {
        self.origin_scope
    }
    pub const fn origin_generation(&self) -> ScopeOwnershipGeneration {
        self.origin_generation
    }
}

impl ActivityHandoffOwner {
    /// This does not require items_complete: its only use is resuming the
    /// receipt's already selected immutable death causes before final return.
    pub async fn read_departed_death(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        source: CurrentCharacterItemFence,
    ) -> Result<Option<DepartedActivityDeath>> {
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let root = self.root.clone();
        let node = node.clone();
        self.root
            .try_issue_semantic_pass()?
            .run(move |holder, deadline| {
                Box::pin(async move {
                    let mut tx = begin_semantic_transaction(holder, deadline).await?;
                    assert_recovery_fence(&mut tx, &recovery).await?;
                    lock_admission_relations(&mut tx).await?;
                    let proof = read_in_transaction(&mut tx, &root, &node, source).await?;
                    commit_semantic_transaction(tx, deadline).await?;
                    Ok(proof)
                })
            })
            .await
    }

    /// Compose the existing per-item Type2 recovery with the same semantic
    /// return owner. The caller owns this exact Instance lane. Each partial
    /// item commit is replayable; no successor is issued until all are done.
    pub async fn recover_departed_death(
        &self,
        permit: &SpellLanePermit,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        request: ActivityRecoveryRequest,
    ) -> Result<ActivityRecoveryOutcome> {
        permit.check_scope(request.source.runtime_scope)?;
        if let Some(proof) = self
            .read_departed_death(authority, node, request.source)
            .await?
        {
            if proof.handoff_id != request.handoff_id {
                return Ok(ActivityReturnOutcome::Refused(
                    ActivityRefusal::AuthorityRejected,
                ));
            }
            self.root
                .resume_departed_activity_death_items(permit, authority, node, &proof)
                .await
                .map_err(|_| DurabilityError::Unavailable)?;
        }
        self.recover(authority, node, request).await
    }
}

/// Called under the existing Type2 transaction before replay or mutation.
/// It retains the Character root lock used by normal death-item writes.
pub(crate) async fn validate_departed_activity_death_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    proof: &DepartedActivityDeath,
) -> Result<bool> {
    lock_admission_relations(tx).await?;
    Ok(read_in_transaction(tx, root, node, proof.source)
        .await?
        .as_ref()
        == Some(proof))
}

async fn read_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    root: &DurabilityRoot,
    node: &NodeIncarnationProof,
    source: CurrentCharacterItemFence,
) -> Result<Option<DepartedActivityDeath>> {
    let RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = source.runtime_scope
    else {
        return Ok(None);
    };
    let row = sqlx::query("SELECT uuid_send(g.handoff_id) AS handoff,p.account_id::text,g.content_revision,uuid_send(p.origin_channel_id) AS origin \
        FROM game_activity_handoff_groups g JOIN game_activity_handoff_participants p USING(handoff_id) \
        JOIN game_activity_runtime_entries e USING(handoff_id) \
        WHERE g.world_id=encode($1,'hex')::uuid AND g.instance_id=encode($2,'hex')::uuid \
          AND g.ownership_generation=$3::text::numeric AND g.state=2 \
          AND p.character_id=encode($4,'hex')::uuid AND p.destination_game_session_id=encode($5,'hex')::uuid \
          AND NOT EXISTS(SELECT 1 FROM game_activity_runtime_returns r WHERE (r.handoff_id,r.character_id)=(p.handoff_id,p.character_id)) \
        FOR SHARE OF g,p,e")
        .bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice()).bind(source.scope_ownership_generation.get().to_string())
        .bind(source.character_id.as_bytes().as_slice()).bind(source.game_session_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(None);
    };
    sqlx::query("SELECT 1 FROM game_durability_reconnect_sessions WHERE game_session_id=encode($1,'hex')::uuid FOR UPDATE")
        .bind(source.game_session_id.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    sqlx::query(
        "SELECT 1 FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid FOR UPDATE",
    )
    .bind(source.character_id.as_bytes().as_slice())
    .fetch_optional(&mut **tx)
    .await?;
    if !super::returning::recovery_source(
        tx,
        root,
        node,
        source,
        &row.try_get::<String, _>("account_id")?,
        &row.try_get::<String, _>("content_revision")?,
    )
    .await?
    {
        return Ok(None);
    }
    let Some(occurrence) = pending_death_occurrence(tx, source).await? else {
        return Ok(None);
    };
    let channel_id = crate::foundation::ChannelId::decode(&bytes16(row.try_get("origin")?)?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let origin_scope = RuntimeScopeRefV1::channel(world_id, channel_id);
    let generation: Option<String> = sqlx::query_scalar("SELECT ownership_generation::text FROM game_runtime_scope_assignments WHERE scope_key=$1 AND state=1 FOR SHARE")
        .bind(assignment_scope_key(origin_scope).as_slice()).fetch_optional(&mut **tx).await?;
    let Some(generation) = generation else {
        return Ok(None);
    };
    let origin_generation = ScopeOwnershipGeneration::new(parse_u64(generation)?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let guards = AdmissionGuardStore::from_root(root.clone());
    if guards
        .load_locked(tx, &AdmissionAuthorityGuardKeyV1::Runtime(origin_scope))
        .await?
        .is_none()
        || !assert_runtime_scope_fence(tx, origin_scope, origin_generation, node).await?
    {
        return Ok(None);
    }
    Ok(Some(DepartedActivityDeath {
        source,
        occurrence,
        handoff_id: bytes16(row.try_get("handoff")?)?,
        origin_scope,
        origin_generation,
    }))
}

/// The occurrence is chosen by retained immutable death state, not by a caller.
/// Deliberately omits items_complete; the final return cell checker requires it.
pub(super) async fn pending_death_occurrence(
    tx: &mut Transaction<'_, Postgres>,
    source: CurrentCharacterItemFence,
) -> Result<Option<[u8; 16]>> {
    let RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = source.runtime_scope
    else {
        return Ok(None);
    };
    let row: Option<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(p.death_occurrence_id) FROM game_character_pending_respawns p \
        JOIN game_character_death_receipts d ON (d.death_occurrence_id,d.character_id,d.respawn_position)=(p.death_occurrence_id,p.character_id,p.respawn_position) \
        WHERE p.character_id=encode($1,'hex')::uuid AND d.death_scope_kind=2 AND d.death_world_id=encode($2,'hex')::uuid \
          AND d.death_instance_id=encode($3,'hex')::uuid AND d.death_scope_generation=$4::text::numeric \
          AND octet_length(p.respawn_position)=10 FOR SHARE OF p,d")
        .bind(source.character_id.as_bytes().as_slice()).bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice())
        .bind(source.scope_ownership_generation.get().to_string()).fetch_optional(&mut **tx).await?;
    row.map(bytes16).transpose()
}
