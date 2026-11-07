use super::*;

pub(super) fn select_columns() -> &'static str {
    "SELECT character_id::text,cause_id::text,cause_ordinal::text,boss_id,world_id::text,channel_id::text,scope_generation::text,lease_generation::text,exit_x,exit_y,exit_floor,entered_at,deadline_at,state,deadline_at<=floor(extract(epoch FROM clock_timestamp())*1000)::bigint AS expired FROM game_character_grizzly_boss_entries"
}

pub(super) fn decode(row: &sqlx::postgres::PgRow) -> StoredResult<GrizzlyBossEntry> {
    Ok(GrizzlyBossEntry {
        character_id: CharacterId::from_bytes(uuid_text(row.try_get("character_id")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        cause: CommandRef::new(
            GameSessionId::decode(&uuid_text(row.try_get("cause_id")?)?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
            CommandId::new(numeric_u64(row, "cause_ordinal")?)
                .map_err(|_| DurabilityError::InvalidStoredState)?,
        ),
        boss_id: u8::try_from(row.try_get::<i16, _>("boss_id")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        world_id: WorldId::decode(&uuid_text(row.try_get("world_id")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        channel_id: ChannelId::decode(&uuid_text(row.try_get("channel_id")?)?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        scope_generation: ScopeOwnershipGeneration::new(numeric_u64(row, "scope_generation")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        character_lease_generation: numeric_u64(row, "lease_generation")?,
        exit: MovementLocalPosition {
            x: row.try_get("exit_x")?,
            y: row.try_get("exit_y")?,
            floor: row.try_get("exit_floor")?,
        },
        entered_at_unix_ms: row.try_get("entered_at")?,
        deadline_unix_ms: row.try_get("deadline_at")?,
        state: match row.try_get::<&str, _>("state")? {
            "ACTIVE" => GrizzlyBossEntryState::Active,
            "RETURN_PENDING" => GrizzlyBossEntryState::ReturnPending,
            _ => return Err(DurabilityError::InvalidStoredState),
        },
        expired: row.try_get("expired")?,
    })
}
