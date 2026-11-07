//! Current-Tibia cadence: ten-minute goal polls and fixed half-hour eating rounds.
use super::*;

pub(super) async fn scheduled_clock(
    tx: &mut Transaction<'_, Postgres>,
    world: &mut CakeWorldSnapshot,
    now: i64,
) -> Result<()> {
    let previous = world.state.clone();
    super::settle_clock_state(tx, world, now).await?;
    if world.state.stage == 1 && now < world.event_end {
        let elapsed = now
            .checked_sub(world.event_start)
            .ok_or(BossClockError::InvalidStoredState)?;
        let slot = elapsed / 600;
        if slot >= 1 {
            let ready = world.state.substances >= 200 && world.state.golems >= 200;
            let claimed=sqlx::query("INSERT INTO game_seasonal_cake_stage_polls(world_id,event_start,poll_slot,observed_at,opened_bridge) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5) ON CONFLICT DO NOTHING")
                .bind(world.world.as_bytes().as_slice()).bind(world.event_start).bind(slot).bind(now).bind(ready).execute(&mut **tx).await?.rows_affected()==1;
            if claimed && ready {
                world.state.stage = 2;
            }
        }
    }
    if world.state.stage == 2 && world.state.window_start.is_none() && now < world.event_end {
        let bridge:Option<i64>=sqlx::query_scalar("SELECT observed_at FROM game_seasonal_cake_stage_polls WHERE world_id=encode($1,'hex')::uuid AND event_start=$2 AND opened_bridge")
            .bind(world.world.as_bytes().as_slice()).bind(world.event_start).fetch_optional(&mut **tx).await?;
        let first = bridge
            .ok_or(BossClockError::InvalidStoredState)?
            .checked_add(1800)
            .ok_or(BossClockError::InvalidStoredState)?;
        if now >= first {
            let start = first + (now - first) / 1800 * 1800;
            let previous_round: Option<i64> = if world.state.layer > 1 {
                sqlx::query_scalar("SELECT max(window_start) FROM game_seasonal_cake_tiles WHERE world_id=encode($1,'hex')::uuid AND event_start=$2 AND layer=$3")
                    .bind(world.world.as_bytes().as_slice()).bind(world.event_start).bind(i16::from(world.state.layer-1)).fetch_one(&mut **tx).await?
            } else {
                None
            };
            if previous_round.is_none_or(|old| start > old) {
                let claimed=sqlx::query("INSERT INTO game_seasonal_cake_windows(world_id,event_start,window_start,layer,observed_at) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5) ON CONFLICT DO NOTHING")
                    .bind(world.world.as_bytes().as_slice()).bind(world.event_start).bind(start).bind(i16::from(world.state.layer)).bind(now).execute(&mut **tx).await?.rows_affected()==1;
                if claimed {
                    world.state.window_start = Some(start);
                    world.state.window_end = Some(
                        start
                            .checked_add(1800)
                            .ok_or(BossClockError::InvalidStoredState)?,
                    );
                }
            }
        }
    }
    super::save_changed_world(tx, world, previous).await
}
