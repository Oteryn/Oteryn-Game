//! Accepted journal transition for one retained spell occurrence. Historical
//! evidence is not current authority; callers keep their independently fenced
//! Item transaction and physical owner borrow while consuming this binding.
use super::{ACTIVE, COMMITTED, active_committed_binding_is_valid, load_session_for_update};
use crate::domain::CharacterId;
use crate::durability::{
    DurabilityError,
    character_progression::CurrentCharacterGameplayFence,
    item_transfer::CurrentCharacterItemFence,
    spell_item_transaction::{self, SpellItemAuthority},
};
use crate::foundation::{CommandRef, ConnectionGeneration, RuntimeScopeRefV1};
use sqlx::{Postgres, Row, Transaction};

#[derive(Debug)]
pub(crate) struct SpellReconnectTransition {
    predecessor: ConnectionGeneration,
    current: ConnectionGeneration,
    command: CommandRef,
    character: CharacterId,
    lease: u64,
    scope: RuntimeScopeRefV1,
    scope_generation: u64,
}
impl SpellReconnectTransition {
    #[allow(clippy::too_many_arguments)]
    pub(crate) fn matches(
        &self,
        predecessor: ConnectionGeneration,
        current: ConnectionGeneration,
        command: CommandRef,
        character: CharacterId,
        lease: u64,
        scope: RuntimeScopeRefV1,
        scope_generation: u64,
    ) -> bool {
        self.predecessor == predecessor
            && self.current == current
            && self.command == command
            && self.character == character
            && self.lease == lease
            && self.scope == scope
            && self.scope_generation == scope_generation
    }
}

pub(crate) async fn prove_pending_spell_reconnect(
    tx: &mut Transaction<'_, Postgres>,
    authority: &SpellItemAuthority,
    original: &CurrentCharacterGameplayFence,
    fresh: &CurrentCharacterItemFence,
) -> Result<Option<SpellReconnectTransition>, DurabilityError> {
    spell_item_transaction::check_transaction(tx, authority)
        .await
        .map_err(|_| DurabilityError::Unavailable)?;
    if original.character_id != fresh.character_id
        || original.game_session_id != fresh.game_session_id
        || original.character_lease_generation != fresh.character_lease_generation
        || original.runtime_scope != fresh.runtime_scope
        || original.scope_ownership_generation != fresh.scope_ownership_generation
        || authority.character_id_bytes() != *fresh.character_id.as_bytes()
        || authority.game_session_id() != fresh.game_session_id
        || authority.connection_generation() != fresh.connection_generation
        || authority.character_lease_generation() != fresh.character_lease_generation
        || authority.runtime_scope() != fresh.runtime_scope
        || authority.scope_generation() != fresh.scope_ownership_generation.get()
    {
        return Err(DurabilityError::Unavailable);
    }
    if original.connection_generation == fresh.connection_generation {
        return Ok(None);
    }
    let session_id = fresh.game_session_id.as_bytes();
    let session = load_session_for_update(tx, session_id)
        .await?
        .ok_or(DurabilityError::Unavailable)?;
    if session.try_get::<i16, _>("session_state")? != ACTIVE
        || session.try_get::<String, _>("predecessor_generation")?
            != original.connection_generation.get().to_string()
        || session.try_get::<String, _>("current_generation")?
            != fresh.connection_generation.get().to_string()
        || session.try_get::<Vec<u8>, _>("character_id")?.as_slice()
            != fresh.character_id.as_bytes()
        || session.try_get::<String, _>("character_lease_generation")?
            != fresh.character_lease_generation.to_string()
        || session.try_get::<String, _>("scope_ownership_generation")?
            != fresh.scope_ownership_generation.get().to_string()
        || !active_committed_binding_is_valid(tx, session_id, &session).await?
    {
        return Err(DurabilityError::Unavailable);
    }
    let transport: Option<Vec<u8>> = session.try_get("current_transport_ref")?;
    let transport = transport.ok_or(DurabilityError::Unavailable)?;
    let rows = sqlx::query("SELECT fnd02_next_command_id::text AS next_command,record_json FROM game_durability_reconnect_attempts WHERE game_session_id=encode($1,'hex')::uuid AND state=$2 AND transport_ref=$3 LIMIT 2")
        .bind(session_id.as_slice()).bind(COMMITTED).bind(transport).fetch_all(&mut **tx).await?;
    if rows.len() != 1 {
        return Err(DurabilityError::Unavailable);
    }
    let command = authority.command();
    let canonical: serde_json::Value =
        serde_json::from_str(&rows[0].try_get::<String, _>("record_json")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
    let expected = command.command_id().get().to_string();
    if rows[0].try_get::<String, _>("next_command")? != expected
        || super::canonical_u64_text(&canonical["fnd02"]["next_command_id"]) != Some(expected)
    {
        return Err(DurabilityError::Unavailable);
    }
    Ok(Some(SpellReconnectTransition {
        predecessor: original.connection_generation,
        current: fresh.connection_generation,
        command,
        character: fresh.character_id,
        lease: fresh.character_lease_generation,
        scope: fresh.runtime_scope,
        scope_generation: fresh.scope_ownership_generation.get(),
    }))
}
