//! A sealed read proof for the existing Ground retirement owner.
//! Returning every participant ends use of a private scope; item retirement then
//! completes before the native runtime can be removed from its directory.
use super::*;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RetiringActivity {
    handoff_id: [u8; 16],
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
}

impl RetiringActivity {
    pub const fn handoff_id(&self) -> [u8; 16] {
        self.handoff_id
    }
    pub const fn scope(&self) -> RuntimeScopeRefV1 {
        self.scope
    }
    pub const fn generation(&self) -> ScopeOwnershipGeneration {
        self.generation
    }
}

impl ActivityHandoffOwner {
    /// Scan durable private Ground after a process loss. This is independent of
    /// the native directory, but still requires this node's current origin
    /// Channel. Every returned witness is rechecked by the same item owner.
    pub async fn departed_retirements_after(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        origin: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
        after: Option<[u8; 16]>,
    ) -> Result<(Vec<RetiringActivity>, Option<[u8; 16]>)> {
        let RuntimeScopeRefV1::Channel { world_id, channel_id } = origin else {
            return Err(DurabilityError::InvalidStoredState);
        };
        let recovery = authority.record_for(&self.root).map_err(|_| DurabilityError::Unavailable)?;
        let root = self.root.clone();
        let node = node.clone();
        self.root.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            if !super::departed_runtime::origin_current(&mut tx, &root, &node, origin, generation).await? {
                return Err(DurabilityError::Unavailable);
            }
            let rows = sqlx::query("SELECT uuid_send(g.handoff_id) AS handoff,uuid_send(g.instance_id) AS instance,g.ownership_generation::text AS generation \
                FROM game_activity_handoff_groups g JOIN game_runtime_scope_assignments a \
                  ON a.scope_key='\\x03'::bytea||uuid_send(g.world_id)||uuid_send(g.instance_id) \
                WHERE g.world_id=encode($1,'hex')::uuid AND g.state=2 \
                  AND ($3::bytea IS NULL OR g.handoff_id>encode($3,'hex')::uuid) \
                  AND EXISTS(SELECT 1 FROM game_activity_handoff_participants p WHERE p.handoff_id=g.handoff_id AND p.origin_channel_id=encode($2,'hex')::uuid) \
                  AND (a.state<>1 OR (a.ownership_generation=g.ownership_generation AND NOT game_node_lock_current_registration(a.holder_node_id,a.holder_registration_revision))) \
                  AND EXISTS(SELECT 1 FROM game_item_ground_locations l JOIN game_item_instances i USING(item_instance_id) \
                    WHERE l.scope_kind=2 AND l.world_id=g.world_id AND l.instance_id=g.instance_id AND i.lifecycle=1) \
                ORDER BY g.handoff_id LIMIT 15")
                .bind(world_id.as_bytes().as_slice()).bind(channel_id.as_bytes().as_slice()).bind(after.map(|id| id.to_vec()))
                .fetch_all(&mut *tx).await?;
            let next = if rows.len() == ACTIVITY_PARTICIPANTS_MAX {
                rows.last().map(|row| bytes16(row.try_get("handoff")?)).transpose()?
            } else { None };
            let mut activities = Vec::with_capacity(rows.len());
            for row in rows {
                let proof = RetiringActivity {
                    handoff_id: bytes16(row.try_get("handoff")?)?,
                    scope: RuntimeScopeRefV1::instance(world_id, bytes16(row.try_get("instance")?)?).map_err(|_| DurabilityError::InvalidStoredState)?,
                    generation: ScopeOwnershipGeneration::new(parse_u64(row.try_get("generation")?)?).map_err(|_| DurabilityError::InvalidStoredState)?,
                };
                if validate_retiring_activity_in_transaction(&mut tx, &node, &proof).await? { activities.push(proof); }
            }
            commit_semantic_transaction(tx, deadline).await?;
            Ok((activities, next))
        })).await
    }

    /// Historical receipts alone are insufficient: this reads the complete
    /// retained group and proves that no admitted session remains in its exact
    /// Instance. It neither creates nor revokes a control assignment.
    pub async fn retirement_ready(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
    ) -> Result<Option<RetiringActivity>> {
        let RuntimeScopeRefV1::Instance {
            world_id,
            instance_id,
        } = scope
        else {
            return Ok(None);
        };
        let recovery = authority
            .record_for(&self.root)
            .map_err(|_| DurabilityError::Unavailable)?;
        let node = node.clone();
        self.root.try_issue_semantic_pass()?.run(move|holder,deadline|Box::pin(async move{
            let mut tx=begin_semantic_transaction(holder,deadline).await?;
            assert_recovery_fence(&mut tx,&recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let id:Option<Vec<u8>>=sqlx::query_scalar("SELECT uuid_send(handoff_id) FROM game_activity_handoff_groups \
                WHERE world_id=encode($1,'hex')::uuid AND instance_id=encode($2,'hex')::uuid AND ownership_generation=$3::text::numeric AND state=2 FOR SHARE")
                .bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice()).bind(generation.get().to_string()).fetch_optional(&mut *tx).await?;
            let Some(id)=id else{return Ok(None);};
            let proof=RetiringActivity{handoff_id:bytes16(id)?,scope,generation};
            if !validate_retiring_activity_in_transaction(&mut tx,&node,&proof).await? {return Ok(None);}
            commit_semantic_transaction(tx,deadline).await?;
            Ok(Some(proof))
        })).await
    }
}

/// The Ground owner calls this again in every freeze/commit/replay transaction.
/// A still-assigned scope may be cleared only by its current node at this exact
/// generation. A revoked scope is clearable after all returns and the absence of
/// every live session in the Instance. A replacement assignment cannot authorize
/// this old witness to erase a newly installed generation's Ground.
pub(crate) async fn validate_retiring_activity_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    proof: &RetiringActivity,
) -> Result<bool> {
    let RuntimeScopeRefV1::Instance {
        world_id,
        instance_id,
    } = proof.scope
    else {
        return Ok(false);
    };
    if !prove_current_incarnation(tx, node).await? {
        return Ok(false);
    }
    let history: bool = sqlx::query_scalar("SELECT game_runtime_scope_assignment_history_valid()")
        .fetch_one(&mut **tx)
        .await?;
    if !history {
        return Ok(false);
    }
    // A crashed process may leave its assignment ACTIVE until control revokes
    // it. The origin's current owner can finish the returned scope only after
    // the independent registration proves that exact holder has ended. A
    // replacement ACTIVE generation is deliberately not this cleanup's scope.
    let origin_owned = current_retirement_origin(tx, node, proof).await?;
    let fact = node.fact();
    let valid:Option<bool>=sqlx::query_scalar("SELECT g.participant_count=(SELECT count(*) FROM game_activity_handoff_participants p WHERE p.handoff_id=g.handoff_id) \
        AND NOT EXISTS(SELECT 1 FROM game_activity_handoff_participants p \
            LEFT JOIN game_activity_runtime_returns r ON (r.handoff_id,r.character_id)=(p.handoff_id,p.character_id) \
            LEFT JOIN game_durability_reconnect_sessions s ON s.game_session_id=p.destination_game_session_id \
            WHERE p.handoff_id=g.handoff_id AND (r.return_id IS NULL OR r.source_game_session_id<>p.destination_game_session_id \
                OR s.session_state IS DISTINCT FROM 3 OR NOT COALESCE(game_activity_session_origin_valid(r.destination_game_session_id,p.character_id),false))) \
        AND NOT EXISTS(SELECT 1 FROM game_durability_reconnect_sessions s WHERE s.runtime_scope_kind=2 \
            AND s.runtime_scope_world_id=g.world_id AND s.runtime_scope_instance_id=g.instance_id AND s.session_state IN(1,2)) \
        AND (a.state<>1 OR (a.ownership_generation=g.ownership_generation \
            AND ((a.holder_node_id=encode($4,'hex')::uuid AND a.holder_registration_revision=$5::text::numeric) \
              OR ($7 AND NOT game_node_lock_current_registration(a.holder_node_id,a.holder_registration_revision))))) \
        FROM game_activity_handoff_groups g JOIN game_activity_runtime_entries e USING(handoff_id) \
        JOIN game_runtime_scope_assignments a ON a.scope_key='\\x03'::bytea || uuid_send(g.world_id) || uuid_send(g.instance_id) \
        WHERE g.handoff_id=encode($1,'hex')::uuid AND g.world_id=encode($2,'hex')::uuid AND g.instance_id=encode($3,'hex')::uuid \
          AND g.state=2 AND g.ownership_generation=$6::text::numeric AND a.source_revision>=e.assignment_source_revision FOR SHARE OF g,e,a")
        .bind(proof.handoff_id.as_slice()).bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice())
        .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string()).bind(proof.generation.get().to_string()).bind(origin_owned)
        .fetch_optional(&mut **tx).await?;
    Ok(valid == Some(true))
}

async fn current_retirement_origin(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    proof: &RetiringActivity,
) -> Result<bool> {
    let fact = node.fact();
    let rows = sqlx::query("SELECT DISTINCT uuid_send(p.origin_channel_id) AS channel,a.ownership_generation::text AS generation \
        FROM game_activity_handoff_participants p JOIN game_runtime_scope_assignments a \
          ON a.scope_key='\\x01'::bytea||uuid_send(p.world_id)||uuid_send(p.origin_channel_id) \
        WHERE p.handoff_id=encode($1,'hex')::uuid AND p.world_id=encode($2,'hex')::uuid \
          AND a.state=1 AND a.holder_node_id=encode($3,'hex')::uuid AND a.holder_registration_revision=$4::text::numeric")
        .bind(proof.handoff_id.as_slice()).bind(proof.scope.world_id().as_bytes().as_slice())
        .bind(fact.node_id().as_bytes().as_slice()).bind(fact.registration_revision().to_string())
        .fetch_all(&mut **tx).await?;
    for row in rows {
        let channel = crate::foundation::ChannelId::decode(&bytes16(row.try_get("channel")?)?).map_err(|_| DurabilityError::InvalidStoredState)?;
        let generation = ScopeOwnershipGeneration::new(parse_u64(row.try_get("generation")?)?).map_err(|_| DurabilityError::InvalidStoredState)?;
        if assert_runtime_scope_fence(tx, RuntimeScopeRefV1::channel(proof.scope.world_id(), channel), generation, node).await? {
            return Ok(true);
        }
    }
    Ok(false)
}

/// Reconstitute a retained cause only by independently checking all handoff and
/// current-owner facts. A decoded audit row alone never constructs this witness.
pub(crate) async fn read_retiring_activity_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    node: &NodeIncarnationProof,
    handoff_id: [u8; 16],
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
) -> Result<Option<RetiringActivity>> {
    let proof = RetiringActivity {
        handoff_id,
        scope,
        generation,
    };
    Ok(validate_retiring_activity_in_transaction(tx, node, &proof)
        .await?
        .then_some(proof))
}
