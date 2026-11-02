use super::{ProductionKey, WorldId};
use crate::durability::DurabilityError;
use oteryn_gameplay_encounter_rules::raid::{
    LuaRaidCheck, LuaRaidPolicy, LuaRaidState, lua_raid_roll,
};
use sqlx::{Postgres, Transaction};

#[derive(Debug, Clone)]
pub struct LuaRaidScheduleCandidate {
    pub policy: LuaRaidPolicy,
}

pub(super) enum LuaSelection {
    NotDue,
    AwaitActivity,
    Selected(bool),
}

pub(super) async fn selected(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    key: &ProductionKey,
    candidate: &LuaRaidScheduleCandidate,
    now_ms: i64,
    due_at_ms: i64,
    running: bool,
) -> Result<LuaSelection, DurabilityError> {
    let clock = super::calendar::clock(tx, world, now_ms).await?;
    let raw: Option<String> = sqlx::query_scalar("SELECT state::text FROM game_world_raid_lua_states WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2 FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(key.as_str()).fetch_optional(&mut **tx).await?;
    let before: LuaRaidState = if let Some(raw) = raw {
        serde_json::from_str(&raw).map_err(|_| DurabilityError::InvalidStoredState)?
    } else {
        let initial = LuaRaidState::initial(now_ms, clock.day)
            .map_err(|_| DurabilityError::InvalidStoredState)?;
        let state =
            serde_json::to_string(&initial).map_err(|_| DurabilityError::InvalidStoredState)?;
        sqlx::query("INSERT INTO game_world_raid_lua_states(world_id,raid_key,state) VALUES(encode($1,'hex')::uuid,$2,$3::text::jsonb)")
            .bind(world.as_bytes().as_slice()).bind(key.as_str()).bind(state).execute(&mut **tx).await?;
        initial
    };
    if now_ms < due_at_ms || now_ms < before.next_check_at_ms {
        return Ok(LuaSelection::NotDue);
    }
    let epoch: String = sqlx::query_scalar("SELECT reset_epoch::text FROM game_world_raid_schedules WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2")
        .bind(world.as_bytes().as_slice()).bind(key.as_str()).fetch_one(&mut **tx).await?;
    let epoch = epoch
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let active_players = if candidate.policy.min_active_players == 0 || running {
        0
    } else {
        let Some(count) = super::activity::world_count(tx, world, epoch, now_ms).await? else {
            return Ok(LuaSelection::AwaitActivity);
        };
        count
    };
    let check = LuaRaidCheck {
        now_ms,
        day: clock.day,
        weekday: clock.weekday,
        server_save_at_ms: clock.server_save_at_ms,
        active_players,
        running,
        roll: lua_raid_roll(world, key, before.check_ordinal)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
    };
    let transition = before
        .plan(&candidate.policy, check)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let state = serde_json::to_string(&transition.state)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    sqlx::query("UPDATE game_world_raid_lua_states SET state=$3::text::jsonb WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2")
        .bind(world.as_bytes().as_slice()).bind(key.as_str()).bind(state).execute(&mut **tx).await?;
    Ok(LuaSelection::Selected(transition.start))
}
