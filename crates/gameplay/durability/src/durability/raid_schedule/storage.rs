use super::{ProductionKey, RaidSchedule, RaidStoreError, RaidTransition, WorldId};
use oteryn_gameplay_encounter_rules::raid::{Firing, FiringState};
use sqlx::{Postgres, Row, Transaction, postgres::PgRow};

/// One statement gives a coherent snapshot even to a reader without a key lock.
/// The numeric ORDER BY is intentional: text order selects 9 ahead of 10/11.
pub async fn load(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    key: &ProductionKey,
) -> Result<Option<RaidSchedule>, RaidStoreError> {
    let row = sqlx::query("SELECT s.revision::text,s.reset_epoch::text,s.check_ordinal::text,s.next_firing_ordinal::text,s.next_check_at_ms,s.last_fired_at_ms,s.last_fixed_slot_ms,s.retired,f.ordinal::text,f.firing_id::text,f.reset_epoch::text AS firing_epoch,f.fired_at_ms,f.deadline_ms,f.bundle_digest,f.state FROM public.game_world_raid_schedules s LEFT JOIN LATERAL (SELECT * FROM public.game_world_raid_firings WHERE world_id=s.world_id AND raid_key=s.raid_key ORDER BY ordinal DESC LIMIT 1) f ON true WHERE s.world_id=encode($1,'hex')::uuid AND s.raid_key=$2 LIMIT 1")
        .bind(world.as_bytes().as_slice()).bind(key.as_str()).fetch_optional(&mut **tx).await?;
    row.as_ref().map(decode).transpose()
}

fn decode(row: &PgRow) -> Result<RaidSchedule, RaidStoreError> {
    let invalid = || RaidStoreError::InvalidStoredState;
    let latest_firing = if row.try_get::<Option<String>, _>("ordinal")?.is_some() {
        let id = super::super::item_mint::uuid_text(row.try_get("firing_id")?)?;
        Some(Firing {
            ordinal: number(row, "ordinal")?,
            firing_id: id,
            reset_epoch: number(row, "firing_epoch")?,
            fired_at_ms: row.try_get("fired_at_ms")?,
            deadline_ms: row.try_get("deadline_ms")?,
            bundle_digest: row
                .try_get::<Vec<u8>, _>("bundle_digest")?
                .try_into()
                .map_err(|_| invalid())?,
            state: match row.try_get::<i16, _>("state")? {
                1 => FiringState::Running,
                2 => FiringState::Done,
                3 => FiringState::Cancelled,
                _ => return Err(invalid()),
            },
        })
    } else {
        None
    };
    let schedule = RaidSchedule {
        revision: number(row, "revision")?,
        reset_epoch: number(row, "reset_epoch")?,
        check_ordinal: number(row, "check_ordinal")?,
        next_firing_ordinal: number(row, "next_firing_ordinal")?,
        next_check_at_ms: row.try_get("next_check_at_ms")?,
        last_fired_at_ms: row.try_get("last_fired_at_ms")?,
        last_fixed_slot_ms: row.try_get("last_fixed_slot_ms")?,
        retired: row.try_get("retired")?,
        latest_firing,
    };
    schedule.validate().map_err(|_| invalid())?;
    Ok(schedule)
}

fn number(row: &PgRow, name: &str) -> Result<u64, RaidStoreError> {
    row.try_get::<String, _>(name)?
        .parse()
        .map_err(|_| RaidStoreError::InvalidStoredState)
}

pub(super) async fn insert_schedule(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    key: &ProductionKey,
    state: &RaidSchedule,
) -> Result<(), RaidStoreError> {
    sqlx::query("INSERT INTO public.game_world_raid_schedules(world_id,raid_key,revision,reset_epoch,check_ordinal,next_firing_ordinal,next_check_at_ms,last_fired_at_ms,last_fixed_slot_ms,retired) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric,$4::text::numeric,$5::text::numeric,$6::text::numeric,$7,$8,$9,$10)")
        .bind(world.as_bytes().as_slice()).bind(key.as_str()).bind(state.revision.to_string())
        .bind(state.reset_epoch.to_string()).bind(state.check_ordinal.to_string()).bind(state.next_firing_ordinal.to_string())
        .bind(state.next_check_at_ms).bind(state.last_fired_at_ms).bind(state.last_fixed_slot_ms).bind(state.retired)
        .execute(&mut **tx).await?;
    Ok(())
}

pub(super) async fn write_transition(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    key: &ProductionKey,
    transition: &RaidTransition,
) -> Result<(), RaidStoreError> {
    let after = transition.after();
    if let Some(firing) = &after.latest_firing {
        let state: i16 = match firing.state {
            FiringState::Running => 1,
            FiringState::Done => 2,
            FiringState::Cancelled => 3,
        };
        if transition
            .before()
            .latest_firing
            .as_ref()
            .map(|f| f.ordinal)
            == Some(firing.ordinal)
        {
            sqlx::query("UPDATE public.game_world_raid_firings SET state=$4 WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2 AND ordinal=$3::text::numeric")
                .bind(world.as_bytes().as_slice()).bind(key.as_str()).bind(firing.ordinal.to_string()).bind(state)
                .execute(&mut **tx).await?;
        } else {
            sqlx::query("INSERT INTO public.game_world_raid_firings(world_id,raid_key,ordinal,firing_id,reset_epoch,fired_at_ms,deadline_ms,bundle_digest,state) VALUES(encode($1,'hex')::uuid,$2,$3::text::numeric,encode($4,'hex')::uuid,$5::text::numeric,$6,$7,$8,$9)")
                .bind(world.as_bytes().as_slice()).bind(key.as_str()).bind(firing.ordinal.to_string()).bind(firing.firing_id.as_slice())
                .bind(firing.reset_epoch.to_string()).bind(firing.fired_at_ms).bind(firing.deadline_ms).bind(firing.bundle_digest.as_slice()).bind(state)
                .execute(&mut **tx).await?;
        }
    }
    sqlx::query("UPDATE public.game_world_raid_schedules SET revision=$3::text::numeric,reset_epoch=$4::text::numeric,check_ordinal=$5::text::numeric,next_firing_ordinal=$6::text::numeric,next_check_at_ms=$7,last_fired_at_ms=$8,last_fixed_slot_ms=$9,retired=$10 WHERE world_id=encode($1,'hex')::uuid AND raid_key=$2")
        .bind(world.as_bytes().as_slice()).bind(key.as_str()).bind(after.revision.to_string()).bind(after.reset_epoch.to_string())
        .bind(after.check_ordinal.to_string()).bind(after.next_firing_ordinal.to_string()).bind(after.next_check_at_ms)
        .bind(after.last_fired_at_ms).bind(after.last_fixed_slot_ms).bind(after.retired).execute(&mut **tx).await?;
    Ok(())
}
