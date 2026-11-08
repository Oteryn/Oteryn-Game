//! M51 shared pit custody, under the existing fenced Character arena transaction.
//! Crystal201189f movements_arena_enter.lua:22-30, movements_arena_pit.lua:52-62;
//! data-global/lib/quests/svargrond_arena.lua:193-256,285-313.
use super::*;
use crate::foundation::RuntimeScopeRefV1;

fn scope(fence: CurrentCharacterGameplayFence) -> Option<([u8; 16], [u8; 16])> {
    match fence.runtime_scope {
        RuntimeScopeRefV1::Channel {
            world_id,
            channel_id,
        } => Some((*world_id.as_bytes(), *channel_id.as_bytes())),
        _ => None,
    }
}

/// Caller already holds the current Character/node/scope fence and root lock.
/// The transaction advisory lock closes the empty-pit race across server tasks.
pub(super) async fn claim(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
    state: &MatchState,
    now_ms: i64,
) -> Result<bool, DurabilityError> {
    let Some((world, channel)) = scope(fence) else {
        return Ok(false);
    };
    if !matches!(state.phase, 1 | 2)
        || !(1..=3).contains(&state.rank)
        || !(1..=10).contains(&state.pit)
        || state.deadline_ms <= now_ms
        || state.session != *fence.game_session_id.as_bytes()
        || state.scope_generation != fence.scope_ownership_generation.get()
    {
        return Ok(false);
    }
    let key = format!(
        "oteryn:svargrond-pit:{}:{}:{}",
        sqlx::types::Uuid::from_bytes(world),
        sqlx::types::Uuid::from_bytes(channel),
        state.pit
    );
    sqlx::query("SELECT pg_advisory_xact_lock(hashtextextended($1,0))")
        .bind(key)
        .execute(&mut **tx)
        .await?;
    let occupied: bool = sqlx::query_scalar("SELECT EXISTS(SELECT 1 FROM game_svargrond_pit_leases WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND pit=$3 AND scope_generation=$4::text::numeric AND expires_at_ms>$5 AND NOT(character_id=encode($6,'hex')::uuid AND game_session_id=encode($7,'hex')::uuid AND run_id=encode($8,'hex')::uuid))")
        .bind(world.as_slice()).bind(channel.as_slice()).bind(i16::from(state.pit)).bind(state.scope_generation.to_string()).bind(now_ms)
        .bind(fence.character_id.as_bytes().as_slice()).bind(state.session.as_slice()).bind(state.run.as_slice()).fetch_one(&mut **tx).await?;
    if occupied {
        return Ok(false);
    }
    // Replace only expired/revoked custody at the selected pit. An active other
    // Character is never evicted merely because this caller reached a portal.
    sqlx::query("DELETE FROM game_svargrond_pit_leases WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND pit=$3 AND (expires_at_ms<=$4 OR scope_generation<>$5::text::numeric)")
        .bind(world.as_slice()).bind(channel.as_slice()).bind(i16::from(state.pit)).bind(now_ms).bind(state.scope_generation.to_string()).execute(&mut **tx).await?;
    sqlx::query("DELETE FROM game_svargrond_pit_leases WHERE character_id=encode($1,'hex')::uuid AND NOT(world_id=encode($2,'hex')::uuid AND channel_id=encode($3,'hex')::uuid AND pit=$4)")
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_slice()).bind(channel.as_slice()).bind(i16::from(state.pit)).execute(&mut **tx).await?;
    let rows = sqlx::query("INSERT INTO game_svargrond_pit_leases(world_id,channel_id,pit,character_id,game_session_id,run_id,scope_generation,expires_at_ms) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,$7::text::numeric,$8) ON CONFLICT(world_id,channel_id,pit) DO UPDATE SET expires_at_ms=EXCLUDED.expires_at_ms WHERE game_svargrond_pit_leases.character_id=EXCLUDED.character_id AND game_svargrond_pit_leases.game_session_id=EXCLUDED.game_session_id AND game_svargrond_pit_leases.run_id=EXCLUDED.run_id AND game_svargrond_pit_leases.scope_generation=EXCLUDED.scope_generation")
        .bind(world.as_slice()).bind(channel.as_slice()).bind(i16::from(state.pit)).bind(fence.character_id.as_bytes().as_slice()).bind(state.session.as_slice()).bind(state.run.as_slice()).bind(state.scope_generation.to_string()).bind(state.deadline_ms)
        .execute(&mut **tx).await?.rows_affected();
    if rows != 1 {
        return Err(DurabilityError::InvalidStoredState);
    }
    Ok(true)
}

pub(super) async fn release(
    tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
    fence: CurrentCharacterGameplayFence,
) -> Result<(), DurabilityError> {
    sqlx::query("DELETE FROM game_svargrond_pit_leases WHERE character_id=encode($1,'hex')::uuid")
        .bind(fence.character_id.as_bytes().as_slice())
        .execute(&mut **tx)
        .await?;
    Ok(())
}

#[cfg(test)]
#[path = "pit_lease_tests.rs"]
mod tests;
