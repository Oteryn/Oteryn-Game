//! Captured world-effect queries and idempotent acknowledgement.
use super::*;

impl DurabilityRoot {
    pub async fn staff_world_effect_state(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        operation_id: &str,
    ) -> Result<Option<StaffWorldEffectState>, DurabilityError> {
        if !rules::hex(operation_id, 16) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let operation_id = operation_id.to_owned();
        self.with_staff_tools_fence(authority, node, fence, move |tx, fence, environment, _| Box::pin(async move {
            let row: Option<(bool, Option<bool>)> = sqlx::query_as("SELECT applied,success FROM game_staff_world_effects WHERE operation_id=$1::uuid AND issuer_character_id=encode($2,'hex')::uuid AND ($3::text IS NULL OR environment=$3) ORDER BY environment LIMIT 1")
                .bind(operation_id).bind(fence.character_id.as_bytes().as_slice()).bind(environment).fetch_optional(tx).await?;
            Ok(row.map(|(applied,success)| if !applied { StaffWorldEffectState::Pending } else if success==Some(true) { StaffWorldEffectState::Applied } else { StaffWorldEffectState::Refused }))
        })).await
    }

    pub async fn staff_world_effect_target(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        operation_id: &str,
    ) -> Result<Option<StaffWorldEffectTarget>, DurabilityError> {
        if !rules::hex(operation_id, 16) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let operation_id = operation_id.to_owned();
        self.with_staff_tools_fence(authority, node, fence, move |tx, fence, environment, _| Box::pin(async move {
            let target: Option<(String,String)> = sqlx::query_as("SELECT e.target_game_session_id::text,e.target_connection_generation::text FROM game_staff_world_effects e JOIN game_durability_reconnect_sessions s ON s.game_session_id=e.target_game_session_id JOIN game_staff_operations o ON o.environment=e.environment AND o.operation_id=e.operation_id WHERE e.operation_id=$1::uuid AND e.issuer_character_id=encode($2,'hex')::uuid AND ($3::text IS NULL OR e.environment=$3) AND s.session_state=2 AND s.current_generation=e.target_connection_generation AND s.scope_ownership_generation=e.target_scope_ownership_generation AND s.runtime_scope_world_id=o.world_id AND s.runtime_scope_channel_id=o.channel_id ORDER BY e.environment LIMIT 1")
                .bind(operation_id).bind(fence.character_id.as_bytes().as_slice()).bind(environment).fetch_optional(tx).await?;
            target.map(|(session,generation)| {
                let bytes=crate::durability::character_authority::uuid_text(&session)?;
                let session=GameSessionId::decode(&bytes).map_err(|_| DurabilityError::InvalidStoredState)?;
                let connection_generation=generation.parse::<u64>().map_err(|_|DurabilityError::InvalidStoredState)?;
                Ok(StaffWorldEffectTarget { session,connection_generation })
            }).transpose()
        })).await
    }

    pub async fn ack_staff_world_effect(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        fence: CurrentCharacterItemFence,
        operation_id: &str,
        applied: bool,
    ) -> Result<(), DurabilityError> {
        if !rules::hex(operation_id, 16) {
            return Err(DurabilityError::InvalidStoredState);
        }
        let operation_id = operation_id.to_owned();
        self.with_staff_tools_fence(authority, node, fence, move |tx, fence, environment, _| Box::pin(async move {
            let row = sqlx::query("UPDATE game_staff_world_effects SET applied=true,success=$4 WHERE operation_id=$1::uuid AND issuer_character_id=encode($2,'hex')::uuid AND ($3::text IS NULL OR environment=$3) AND NOT applied RETURNING environment,action,target_game_session_id::text,target_character_id::text")
                .bind(&operation_id).bind(fence.character_id.as_bytes().as_slice()).bind(environment).bind(applied).fetch_optional(&mut *tx).await?;
            if let Some(row)=row.filter(|_| applied) {
                let action: serde_json::Value=row.try_get("action")?;
                let kind=action["kind"].as_str().unwrap_or_default();
                if matches!(kind,"GHOST"|"INVISIBLE") {
                    let environment: String=row.try_get("environment")?;
                    let session: Option<String>=row.try_get("target_game_session_id")?;
                    let session=session.ok_or(DurabilityError::InvalidStoredState)?;
                    let character: String=row.try_get("target_character_id")?;
                    let enabled=action["enabled"].as_bool().ok_or(DurabilityError::InvalidStoredState)?;
                    sqlx::query("INSERT INTO game_staff_actor_flags(environment,game_session_id,character_id,ghost,invisible,operation_id) VALUES($1,$2::uuid,$3::uuid,$4,$5,$6::uuid) ON CONFLICT(environment,game_session_id) DO UPDATE SET ghost=CASE WHEN $7='GHOST' THEN EXCLUDED.ghost ELSE game_staff_actor_flags.ghost END,invisible=CASE WHEN $7='INVISIBLE' THEN EXCLUDED.invisible ELSE game_staff_actor_flags.invisible END,operation_id=EXCLUDED.operation_id")
                        .bind(environment).bind(session).bind(character).bind(kind=="GHOST" && enabled)
                        .bind(kind=="INVISIBLE" && enabled).bind(operation_id).bind(kind).execute(&mut *tx).await?;
                }
            }
            Ok(())
        })).await
    }
}
