//! World-wide admission layered on the existing fenced Channel clock transaction.
use super::*;

pub(super) struct WorldSpawn {
    pub active: bool,
    pub same_clock: bool,
}

pub(super) async fn lock_world_spawn(
    tx: &mut Transaction<'_, Postgres>,
    key: &BossClockKey,
    boss: &str,
) -> Result<WorldSpawn> {
    if boss.is_empty() {
        return Err(BossClockError::InvalidInput);
    }
    sqlx::query("INSERT INTO game_boss_world_spawns(world_id,boss_key) VALUES(encode($1,'hex')::uuid,$2) ON CONFLICT DO NOTHING")
        .bind(key.world.as_bytes().as_slice()).bind(boss).execute(&mut **tx).await?;
    let row = sqlx::query("SELECT active,COALESCE(channel_id=encode($3,'hex')::uuid AND spawn_key=$4,FALSE) AS same_clock FROM game_boss_world_spawns WHERE world_id=encode($1,'hex')::uuid AND boss_key=$2 FOR UPDATE")
        .bind(key.world.as_bytes().as_slice()).bind(boss).bind(key.channel.as_bytes().as_slice()).bind(&key.spawn).fetch_one(&mut **tx).await?;
    Ok(WorldSpawn {
        active: row.try_get("active")?,
        same_clock: row.try_get("same_clock")?,
    })
}

/// The retained row wins over any subsequent content chance or process restart.
pub(super) async fn retain_roll(
    tx: &mut Transaction<'_, Postgres>,
    key: &BossClockKey,
    boss: &str,
    epoch: u64,
    chance: oteryn_gameplay_content::content::BossChance,
    world_spawn: &WorldSpawn,
) -> Result<bool> {
    const READ: &str = "SELECT boss_key,won,blocked,spawned FROM game_boss_spawn_rolls WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric AND spawn_key=$3 FOR UPDATE";
    let stored: Option<(String, bool, bool, bool)> = sqlx::query_as(READ)
        .bind(key.world.as_bytes().as_slice())
        .bind(epoch.to_string())
        .bind(&key.spawn)
        .fetch_optional(&mut **tx)
        .await?;
    if let Some((stored_boss, won, blocked, spawned)) = stored {
        if stored_boss != boss {
            return Err(BossClockError::InvalidStoredState);
        }
        return Ok(won && !blocked && !spawned);
    }
    let won = oteryn_gameplay_domain::domain::uniform_draw::uniform_draw(
        draw(key, epoch, "boss_at_reset")?,
        0,
        (chance.denominator - 1) as i64,
    ) < chance.numerator as i64;
    sqlx::query("INSERT INTO game_boss_spawn_rolls(world_id,reset_epoch,spawn_key,boss_key,won,blocked) VALUES(encode($1,'hex')::uuid,$2::text::numeric,$3,$4,$5,$6) ON CONFLICT DO NOTHING")
        .bind(key.world.as_bytes().as_slice()).bind(epoch.to_string()).bind(&key.spawn).bind(boss).bind(won).bind(world_spawn.active).execute(&mut **tx).await?;
    let (stored_boss, won, blocked, spawned): (String, bool, bool, bool) = sqlx::query_as(READ)
        .bind(key.world.as_bytes().as_slice())
        .bind(epoch.to_string())
        .bind(&key.spawn)
        .fetch_one(&mut **tx)
        .await?;
    if stored_boss != boss {
        return Err(BossClockError::InvalidStoredState);
    }
    Ok(won && !blocked && !spawned)
}

pub(super) async fn mark_claimed(
    tx: &mut Transaction<'_, Postgres>,
    key: &BossClockKey,
    boss: &str,
    epoch: u64,
    at_reset: bool,
) -> Result<()> {
    sqlx::query("UPDATE game_boss_world_spawns SET active=TRUE,channel_id=encode($3,'hex')::uuid,spawn_key=$4 WHERE world_id=encode($1,'hex')::uuid AND boss_key=$2")
        .bind(key.world.as_bytes().as_slice()).bind(boss).bind(key.channel.as_bytes().as_slice()).bind(&key.spawn).execute(&mut **tx).await?;
    if at_reset {
        sqlx::query("UPDATE game_boss_spawn_rolls SET spawned=TRUE WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric AND spawn_key=$3 AND won AND NOT blocked")
            .bind(key.world.as_bytes().as_slice()).bind(epoch.to_string()).bind(&key.spawn).execute(&mut **tx).await?;
    }
    Ok(())
}

/// Death already has the committed actor's exact generation/cycle fence in the caller.
pub(super) async fn release_world_spawn(
    tx: &mut Transaction<'_, Postgres>,
    key: &BossClockKey,
) -> Result<()> {
    sqlx::query("UPDATE game_boss_world_spawns SET active=FALSE WHERE world_id=encode($1,'hex')::uuid AND channel_id=encode($2,'hex')::uuid AND spawn_key=$3 AND active")
        .bind(key.world.as_bytes().as_slice()).bind(key.channel.as_bytes().as_slice()).bind(&key.spawn).execute(&mut **tx).await?;
    Ok(())
}

/// Read-only Game-owned projection for Platform G-12. A row means a committed spawn
/// claim, including bosses killed later that day. Occupancy can delay actor installation.
/// The bundle limits the set to MAX_POINTS per world/epoch; no protocol is introduced.
pub async fn spawned_bosses_for_epoch(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    epoch: u64,
) -> Result<Vec<(String, String)>> {
    Ok(sqlx::query_as("SELECT boss_key,spawn_key FROM game_boss_spawn_rolls WHERE world_id=encode($1,'hex')::uuid AND reset_epoch=$2::text::numeric AND spawned ORDER BY boss_key,spawn_key LIMIT 131072")
        .bind(world.as_bytes().as_slice()).bind(epoch.to_string()).fetch_all(&mut **tx).await?)
}
