//! Current terminal identity for physical cleanup; never a session/placement grant.
use super::*;

#[derive(Debug, Clone, Copy)]
pub struct SoulpitTerminalCleanup {
    handoff_id: [u8; 16],
    scope: RuntimeScopeRefV1,
    generation: ScopeOwnershipGeneration,
}
impl SoulpitTerminalCleanup {
    pub fn handoff_id(&self) -> [u8; 16] {
        self.handoff_id
    }
    pub fn matches(&self, scope: RuntimeScopeRefV1, generation: ScopeOwnershipGeneration) -> bool {
        self.scope == scope && self.generation == generation
    }
}

impl DurabilityRoot {
    /// Read only an exact current Instance whose terminal fight has no member
    /// awaiting a shared return. Historical identity cannot admit or move one.
    pub async fn read_soulpit_terminal_cleanup(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
    ) -> Result<Option<SoulpitTerminalCleanup>, SoulpitWriteError> {
        let recovery = authority
            .record_for(self)
            .map_err(|_| SoulpitWriteError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            assert_recovery_fence(&mut tx, &recovery).await?;
            lock_admission_relations(&mut tx).await?;
            let RuntimeScopeRefV1::Instance { instance_id, .. } = scope else {
                return Ok(Err(SoulpitWriteError::AuthorityRejected));
            };
            if !assert_runtime_scope_fence(&mut tx, scope, generation, &node).await? {
                return Ok(Err(SoulpitWriteError::AuthorityRejected));
            }
            let rows = sqlx::query("SELECT uuid_send(g.handoff_id) AS handoff,f.checkpoint FROM game_activity_handoff_groups g JOIN game_soulpit_encounters f USING(handoff_id) \
                WHERE g.world_id=encode($1,'hex')::uuid AND g.instance_id=encode($2,'hex')::uuid AND g.activity_key=$3 AND g.state=2 \
                  AND g.ownership_generation<=$4::text::numeric AND f.terminal \
                  AND g.participant_count=(SELECT count(*) FROM game_activity_handoff_participants p WHERE p.handoff_id=g.handoff_id) \
                  AND EXISTS(SELECT 1 FROM game_activity_handoff_participants p WHERE p.handoff_id=g.handoff_id) \
                  AND NOT EXISTS(SELECT 1 FROM game_activity_handoff_participants p WHERE p.handoff_id=g.handoff_id \
                      AND NOT EXISTS(SELECT 1 FROM game_activity_runtime_returns r WHERE r.handoff_id=p.handoff_id AND r.character_id=p.character_id AND r.source_game_session_id=p.destination_game_session_id)) \
                ORDER BY g.handoff_id LIMIT 2")
                .bind(scope.world_id().as_bytes().as_slice()).bind(instance_id.as_slice())
                .bind(SOULPIT_ACTIVITY_KEY).bind(generation.get().to_string()).fetch_all(&mut *tx).await?;
            if rows.len() > 1 {
                return Ok(Err(DurabilityError::InvalidStoredState.into()));
            }
            let result = if let Some(row) = rows.first() {
                let encounter = decode_checkpoint(row.try_get("checkpoint")?)?;
                if !encounter.snapshot().phase.is_terminal() {
                    return Ok(Err(DurabilityError::InvalidStoredState.into()));
                }
                let handoff: Vec<u8> = row.try_get("handoff")?;
                Some(SoulpitTerminalCleanup {
                    handoff_id: handoff.try_into().map_err(|_|DurabilityError::InvalidStoredState)?,
                    scope,generation,
                })
            } else { None };
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(result))
        })).await?
    }
}
