//! Bounded current-participant taint reads for the native activity think owner.
use super::*;
use crate::foundation::{ConnectionGeneration, GameSessionId, ScopeOwnershipGeneration};
use super::super::super::spell_item_transaction::{
    SpellItemError, SpellItemScopeAuthority, check_scope_transaction,
};

pub async fn read_in_transaction(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemScopeAuthority,
    node: &NodeIncarnationProof,
    sessions: &[GameSessionId],
    digest: [u8; 32],
) -> std::result::Result<Vec<(GameSessionId, u8)>, SpellItemError> {
    check_scope_transaction(tx, authority).await?;
    let RuntimeScopeRefV1::Instance { world_id, instance_id } = authority.runtime_scope() else {
        return Err(SpellItemError::Rejected("room taint Instance scope"));
    };
    if sessions.len() > 5 || digest == [0; 32]
        || sessions.iter().enumerate().any(|(i, session)| sessions[..i].contains(session)) {
        return Err(SpellItemError::Rejected("room taint input"));
    }
    let mut output = Vec::with_capacity(sessions.len());
    for session in sessions {
        let row = sqlx::query(
            "SELECT uuid_send(p.character_id) AS character, \
             e.destination_server_artifact_digest AS digest,s.current_generation::text AS connection, \
             s.character_lease_generation::text AS lease,r.character_revision::text AS revision \
             FROM game_activity_handoff_groups g \
             JOIN game_activity_handoff_participants p USING(handoff_id,world_id) \
             JOIN game_activity_runtime_entries e USING(handoff_id) \
             JOIN game_durability_reconnect_sessions s ON s.game_session_id=p.destination_game_session_id AND s.character_id=p.character_id \
             JOIN game_character_roots r ON r.character_id=p.character_id \
             WHERE g.world_id=encode($1,'hex')::uuid AND g.instance_id=encode($2,'hex')::uuid \
             AND g.state=2 AND p.destination_game_session_id=encode($3,'hex')::uuid \
             FOR SHARE OF s,r",
        ).bind(world_id.as_bytes().as_slice()).bind(instance_id.as_slice())
            .bind(session.as_bytes().as_slice()).fetch_optional(&mut **tx).await?
            .ok_or(SpellItemError::Rejected("room taint participant"))?;
        if row.try_get::<Vec<u8>, _>("digest")? != digest {
            return Err(SpellItemError::Rejected("room taint participant pin"));
        }
        let character = CharacterId::from_bytes(row.try_get::<Vec<u8>, _>("character")?
            .try_into().map_err(|_| DurabilityError::InvalidStoredState)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let number = |value: String| value.parse::<u64>().map_err(|_| DurabilityError::InvalidStoredState);
        let fence = CurrentCharacterGameplayFence {
            character_id: character,
            game_session_id: *session,
            connection_generation: ConnectionGeneration::new(number(row.try_get("connection")?)?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            character_lease_generation: number(row.try_get("lease")?)?,
            runtime_scope: authority.runtime_scope(),
            scope_ownership_generation: ScopeOwnershipGeneration::new(authority.generation())
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            expected_character_revision: CharacterRevision::new(number(row.try_get("revision")?)?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        };
        assert_gameplay_fence(tx, &fence, node).await?
            .map_err(|_| SpellItemError::Rejected("room taint current participant"))?;
        // Child flags are progression, not an additional taint counter.
        // Active condition taints remain independently owned by native conditions.
        output.push((*session, read_state(tx, character).await?.regular_taints()));
    }
    Ok(output)
}
