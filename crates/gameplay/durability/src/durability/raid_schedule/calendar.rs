use super::WorldId;
use crate::durability::DurabilityError;
use oteryn_gameplay_encounter_rules::raid::RaidCalendarWindow;
use sqlx::{Postgres, Row, Transaction};

#[derive(Debug, Clone)]
pub struct CalendarRaidScheduleCandidate {
    pub window: RaidCalendarWindow,
    pub interval_ms: u32,
}

pub(super) struct RaidCalendarClock {
    pub day: i64,
    pub weekday: u8,
    pub month: u8,
    pub month_day: u8,
    pub server_save_at_ms: i64,
}

/// Calendar conversion belongs to the existing reset policy. The explicit
/// clock keeps coordinator cases deterministic without a process timezone.
pub(super) async fn clock(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    now_ms: i64,
) -> Result<RaidCalendarClock, DurabilityError> {
    let row = sqlx::query("WITH c AS (SELECT to_timestamp($2::double precision/1000.0) AT TIME ZONE 'Europe/Berlin' AS t) SELECT (c.t::date-DATE '1970-01-01')::bigint AS day,extract(isodow FROM c.t)::smallint AS weekday,extract(month FROM c.t)::smallint AS month,extract(day FROM c.t)::smallint AS month_day,floor(extract(epoch FROM ((date_trunc('day',c.t-interval '6 hours')+interval '1 day 6 hours') AT TIME ZONE 'Europe/Berlin'))*1000)::bigint AS server_save FROM c,game_world_reset_active a JOIN game_world_reset_epochs e USING(world_id,reset_epoch) WHERE a.world_id=encode($1,'hex')::uuid AND e.calendar='TIBIA_SERVER_SAVE_BERLIN_06'")
        .bind(world.as_bytes().as_slice()).bind(now_ms).fetch_optional(&mut **tx).await?
        .ok_or(DurabilityError::InvalidStoredState)?;
    let byte = |name| -> Result<u8, DurabilityError> {
        u8::try_from(row.try_get::<i16, _>(name)?).map_err(|_| DurabilityError::InvalidStoredState)
    };
    Ok(RaidCalendarClock {
        day: row.try_get("day")?,
        weekday: byte("weekday")?,
        month: byte("month")?,
        month_day: byte("month_day")?,
        server_save_at_ms: row.try_get("server_save")?,
    })
}
