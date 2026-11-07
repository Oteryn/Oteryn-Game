//! Specific durable World state for A Piece of Cake; no player intent activates an event.
//! Crystal 201189f8 `data-global/scripts/lib/a_piece_of_cake_config.lua:101-124,373-385`.
use super::DurabilityRoot;
use super::boss_encounter::{BossClockError, assert_channel_owner};
use super::character_authority::ReconciledCharacterAuthority;
use super::db::{begin_semantic_transaction, commit_semantic_transaction};
use super::runtime_scope_assignment::NodeIncarnationProof;
use crate::foundation::{ChannelId, ScopeOwnershipGeneration, WorldId};
use serde::{Deserialize, Serialize};
use sqlx::{Postgres, Row, Transaction};

mod authority;
pub(crate) mod bite;
pub(crate) mod bonus;
pub mod character;
mod delivery;
mod extraction;
pub(crate) mod food;
pub mod footprint;
mod map_projection;
mod raid_slot;
mod timing;
pub use delivery::{BAKERS, CakeDelivery, CakeDeliveryRequest};
pub use food::CakeFoodContext;
pub use map_projection::{CakeMapProjection, CakeTerrainProjection};
type Result<T> = std::result::Result<T, BossClockError>;

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CakeWorldState {
    pub stage: u8,
    pub substances: u16,
    pub golems: u16,
    pub layer: u8,
    pub layer_progress: u16,
    pub window_start: Option<i64>,
    pub window_end: Option<i64>,
    pub bonus_from: Option<i64>,
    pub bonus_until: Option<i64>,
    pub revision: u64,
}

impl CakeWorldState {
    fn initial() -> Self {
        Self {
            stage: 1,
            substances: 0,
            golems: 0,
            layer: 1,
            layer_progress: 0,
            window_start: None,
            window_end: None,
            bonus_from: None,
            bonus_until: None,
            revision: 1,
        }
    }

    pub(super) fn validate(&self) -> Result<()> {
        if self.stage > 3
            || self.substances > 200
            || self.golems > 200
            || !(1..=4).contains(&self.layer)
            || self.layer_progress
                > footprint::layer_goal(self.layer).ok_or(BossClockError::InvalidStoredState)?
            || self.revision == 0
            || self.window_start.is_some() != self.window_end.is_some()
            || self
                .window_start
                .zip(self.window_end)
                .is_some_and(|(start, end)| start < 0 || end.checked_sub(start) != Some(1800))
            || self.bonus_from.is_some() != self.bonus_until.is_some()
            || self
                .bonus_from
                .zip(self.bonus_until)
                .is_some_and(|(start, end)| start < 0 || end.checked_sub(start) != Some(7 * 86400))
        {
            return Err(BossClockError::InvalidStoredState);
        }
        Ok(())
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CakeWorldSnapshot {
    pub world: WorldId,
    pub event_start: i64,
    pub event_end: i64,
    pub state: CakeWorldState,
}

/// Acquires the current occurrence's World row after the caller proves its own authority.
/// The World lock precedes every character/placement row in this specific event owner.
pub(super) async fn lock_world(
    tx: &mut Transaction<'_, Postgres>,
    world: WorldId,
    now_seconds: i64,
) -> Result<Option<CakeWorldSnapshot>> {
    let row = sqlx::query("SELECT event_start,event_end,state FROM game_seasonal_cake_worlds WHERE world_id=encode($1,'hex')::uuid AND event_start<=$2 ORDER BY event_start DESC LIMIT 1 FOR UPDATE")
        .bind(world.as_bytes().as_slice()).bind(now_seconds).fetch_optional(&mut **tx).await?;
    row.map(|row| {
        let state: CakeWorldState = serde_json::from_value(row.try_get("state")?)
            .map_err(|_| BossClockError::InvalidStoredState)?;
        state.validate()?;
        let event_start = row.try_get("event_start")?;
        let event_end = row.try_get("event_end")?;
        if !(1..=253_402_300_799).contains(&event_start)
            || event_end <= event_start
            || event_end > 253_402_300_799
        {
            return Err(BossClockError::InvalidStoredState);
        }
        Ok(CakeWorldSnapshot {
            world,
            event_start,
            event_end,
            state,
        })
    })
    .transpose()
}

pub(super) async fn save_world(
    tx: &mut Transaction<'_, Postgres>,
    world: &CakeWorldSnapshot,
) -> Result<()> {
    world.state.validate()?;
    let state =
        serde_json::to_value(&world.state).map_err(|_| BossClockError::InvalidStoredState)?;
    let changed = sqlx::query("UPDATE game_seasonal_cake_worlds SET state=$3 WHERE world_id=encode($1,'hex')::uuid AND event_start=$2")
        .bind(world.world.as_bytes().as_slice()).bind(world.event_start).bind(state).execute(&mut **tx).await?;
    if changed.rows_affected() != 1 {
        return Err(BossClockError::InvalidStoredState);
    }
    Ok(())
}

/// The only deadlines and thresholds are literal donor configuration, and all transitions
/// run under this World's row lock. A second Channel observes the same retained transition.
pub(super) async fn advance_world(
    tx: &mut Transaction<'_, Postgres>,
    world: &mut CakeWorldSnapshot,
    now: i64,
) -> Result<()> {
    let previous = world.state.clone();
    finish_world_change(tx, world, now, previous).await
}

pub(super) async fn finish_world_change(
    tx: &mut Transaction<'_, Postgres>,
    world: &mut CakeWorldSnapshot,
    now: i64,
    previous: CakeWorldState,
) -> Result<()> {
    settle_clock_state(tx, world, now).await?;
    save_changed_world(tx, world, previous).await
}

pub(super) async fn settle_clock_state(
    tx: &mut Transaction<'_, Postgres>,
    world: &mut CakeWorldSnapshot,
    now: i64,
) -> Result<()> {
    let goal =
        footprint::layer_goal(world.state.layer).ok_or(BossClockError::InvalidStoredState)?;
    if now >= world.event_end {
        world.state.stage = 0;
        world.state.window_start = None;
        world.state.window_end = None;
    } else if world.state.stage == 2
        && world
            .state
            .window_end
            .is_some_and(|end| now >= end || world.state.layer_progress >= goal)
    {
        let completed = world.state.layer_progress >= goal;
        // Restore the edible ground on failure and between nonfinal successful rounds.
        if !completed || world.state.layer < 4 {
            sqlx::query("UPDATE game_seasonal_cake_tiles SET removed=false WHERE world_id=encode($1,'hex')::uuid AND event_start=$2 AND layer=$3 AND removed")
                .bind(world.world.as_bytes().as_slice()).bind(world.event_start).bind(i16::from(world.state.layer)).execute(&mut **tx).await?;
        }
        world.state.window_start = None;
        world.state.window_end = None;
        if completed {
            if world.state.layer == 4 {
                world.state.stage = 3;
                let bonus_from: i64 = sqlx::query_scalar("SELECT floor(extract(epoch FROM ((date_trunc('day',to_timestamp($1::double precision) AT TIME ZONE 'Europe/Berlin'-interval '6 hours')+interval '1 day 6 hours') AT TIME ZONE 'Europe/Berlin')))::bigint")
                    .bind(now).fetch_one(&mut **tx).await?;
                world.state.bonus_from = Some(bonus_from);
                world.state.bonus_until = Some(
                    bonus_from
                        .checked_add(7 * 86400)
                        .ok_or(BossClockError::InvalidInput)?,
                );
            } else {
                world.state.layer += 1;
                world.state.layer_progress = 0;
            }
        } else {
            world.state.layer_progress = 0;
        }
    }
    Ok(())
}

pub(super) async fn save_changed_world(
    tx: &mut Transaction<'_, Postgres>,
    world: &mut CakeWorldSnapshot,
    previous: CakeWorldState,
) -> Result<()> {
    if world.state != previous {
        world.state.revision = previous
            .revision
            .checked_add(1)
            .ok_or(BossClockError::InvalidStoredState)?;
        save_world(tx, world).await?;
    }
    Ok(())
}

impl DurabilityRoot {
    /// The current physical Channel owner refreshes the calendar occurrence and deadlines.
    /// Returns no occurrence before the first admitted event, and keeps an ended occurrence
    /// available so each Channel can clean up its projection and retain the sourced bonus.
    pub async fn refresh_cake_world(
        &self,
        authority: &ReconciledCharacterAuthority<'_, '_>,
        node: &NodeIncarnationProof,
        world: WorldId,
        channel: ChannelId,
        generation: ScopeOwnershipGeneration,
        now_seconds: i64,
    ) -> Result<Option<CakeWorldSnapshot>> {
        if now_seconds < 0 {
            return Err(BossClockError::InvalidInput);
        }
        let recovery = authority
            .record_for(self)
            .map_err(|_| BossClockError::AuthorityRejected)?;
        let node = node.clone();
        self.try_issue_semantic_pass()?.run(move |holder, deadline| Box::pin(async move {
            let mut tx = begin_semantic_transaction(holder, deadline).await?;
            let result = async {
                assert_channel_owner(&mut tx, &recovery, &node, world, channel, generation).await?;
                authority::arm(&mut tx,&node,world,channel,generation,now_seconds).await?;
            let active = oteryn_gameplay_foundation::scheduled_events::seasonal_events_at(now_seconds)
                .into_iter().find(|event| event.event == oteryn_protocol_oteryn::world_event::SeasonalEventKind::BirthdayCake && event.active);
            let mut inserted=false;
            if let Some(event) = active {
                let start = i64::try_from(event.start_unix).map_err(|_| BossClockError::InvalidInput)?;
                let end = i64::try_from(event.end_unix).map_err(|_| BossClockError::InvalidInput)?;
                let initial = serde_json::to_value(CakeWorldState::initial()).map_err(|_| BossClockError::InvalidStoredState)?;
                inserted=sqlx::query("INSERT INTO game_seasonal_cake_worlds(world_id,event_start,event_end,state) VALUES(encode($1,'hex')::uuid,$2,$3,$4) ON CONFLICT DO NOTHING")
                    .bind(world.as_bytes().as_slice()).bind(start).bind(end).bind(initial).execute(&mut *tx).await?.rows_affected()==1;
            }
            let mut snapshot = lock_world(&mut tx, world, now_seconds).await?;
            if let Some(world) = &mut snapshot
                && !inserted { timing::scheduled_clock(&mut tx,world,now_seconds).await?; }
                Ok(snapshot)
            }.await;
            if result.is_ok() { commit_semantic_transaction(tx, deadline).await?; }
            Ok(result)
        })).await?
    }
}
