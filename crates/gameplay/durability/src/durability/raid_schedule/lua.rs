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
    let (day, weekday) =
        check_calendar(clock.day, clock.weekday, clock.civil_day, before.check_day);
    let check = LuaRaidCheck {
        now_ms,
        day,
        weekday,
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

// A raid may already have checked after civil midnight under the old calendar.
// Retain that recorded day through cutover so its budget cannot rewind. The
// caller still rejects a recorded day beyond the current civil date.
fn check_calendar(save_day: i64, weekday: u8, civil_day: i64, recorded_day: i64) -> (i64, u8) {
    if recorded_day > save_day && recorded_day == civil_day {
        (recorded_day, weekday % 7 + 1)
    } else {
        (save_day, weekday)
    }
}

#[cfg(test)]
mod tests {
    use super::check_calendar;

    #[test]
    fn raid_cutover_preserves_only_the_already_recorded_civil_day() {
        // Sunday is still open at 09:59 on Monday. A raid that already checked
        // under the old rule keeps Monday's counter through the first save.
        assert_eq!(check_calendar(20_779, 7, 20_780, 20_780), (20_780, 1));
        assert_eq!(check_calendar(20_779, 7, 20_780, 20_779), (20_779, 7));
        assert_eq!(check_calendar(20_779, 7, 20_780, 20_781), (20_779, 7));
        assert_eq!(check_calendar(20_780, 1, 20_780, 20_780), (20_780, 1));
    }
}
