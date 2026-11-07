//! Bounded current-Instance lookup; a handoff UUID is never a placement grant.
use super::*;
use crate::foundation::GameSessionId;

#[derive(Debug, Clone)]
pub struct SoulpitScopeMember {
    pub character: CharacterId,
    pub slot: u8,
    pub destination_session: GameSessionId,
}

#[derive(Debug, Clone)]
pub struct SoulpitScopeBinding {
    pub handoff_id: [u8; 16],
    pub members: Vec<SoulpitScopeMember>,
}

impl DurabilityRoot {
    pub async fn read_soulpit_scope_binding(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        scope: RuntimeScopeRefV1,
        generation: ScopeOwnershipGeneration,
    ) -> Result<Option<SoulpitScopeBinding>, SoulpitWriteError> {
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
            let groups: Vec<Vec<u8>> = sqlx::query_scalar("SELECT uuid_send(g.handoff_id) FROM game_activity_handoff_groups g JOIN game_soulpit_encounters f USING(handoff_id) WHERE g.world_id=encode($1,'hex')::uuid AND g.instance_id=encode($2,'hex')::uuid AND g.activity_key=$3 AND g.state=2 AND g.ownership_generation<=$4::text::numeric AND EXISTS (SELECT 1 FROM game_activity_handoff_participants p WHERE p.handoff_id=g.handoff_id AND NOT EXISTS (SELECT 1 FROM game_activity_runtime_returns r WHERE r.handoff_id=p.handoff_id AND r.character_id=p.character_id)) ORDER BY g.handoff_id LIMIT 2")
                .bind(scope.world_id().as_bytes().as_slice()).bind(instance_id.as_slice())
                .bind(SOULPIT_ACTIVITY_KEY).bind(generation.get().to_string()).fetch_all(&mut *tx).await?;
            if groups.len() > 1 { return Ok(Err(DurabilityError::InvalidStoredState.into())); }
            let result = if let Some(handoff) = groups.first() {
                let handoff_id: [u8;16] = handoff.as_slice().try_into().map_err(|_| DurabilityError::InvalidStoredState)?;
                let rows = sqlx::query("SELECT uuid_send(s.character_id) AS character,s.staging_slot,uuid_send(p.destination_game_session_id) AS session FROM game_soulpit_participants s JOIN game_activity_handoff_participants p USING(handoff_id,character_id) WHERE s.handoff_id=encode($1,'hex')::uuid ORDER BY s.staging_slot LIMIT 6")
                    .bind(handoff_id.as_slice()).fetch_all(&mut *tx).await?;
                if rows.is_empty() || rows.len() > crate::domain::soulpit::MAX_PARTICIPANTS { return Ok(Err(DurabilityError::InvalidStoredState.into())); }
                let mut members = Vec::with_capacity(rows.len());
                for row in rows {
                    let character: Vec<u8> = row.try_get("character")?;
                    let session: Vec<u8> = row.try_get("session")?;
                    members.push(SoulpitScopeMember {
                        character: CharacterId::from_bytes(character.try_into().map_err(|_| DurabilityError::InvalidStoredState)?)
                            .map_err(|_| DurabilityError::InvalidStoredState)?,
                        slot: u8::try_from(row.try_get::<i16,_>("staging_slot")?).map_err(|_| DurabilityError::InvalidStoredState)?,
                        destination_session: GameSessionId::decode(&session).map_err(|_| DurabilityError::InvalidStoredState)?,
                    });
                }
                Some(SoulpitScopeBinding { handoff_id, members })
            } else { None };
            commit_semantic_transaction(tx, deadline).await?;
            Ok(Ok(result))
        })).await?
    }
}
