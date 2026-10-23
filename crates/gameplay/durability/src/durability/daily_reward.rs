//! DAILY-1 durable state/receipt decoding. No epoch or joker authority lives here.

use super::DurabilityError;
use oteryn_gameplay_progression::daily_reward::DailyRewardState;
use sqlx::{Postgres, Row, Transaction, postgres::PgRow};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DailyRewardReceipt {
    pub transaction_id: [u8; 16],
    pub request_binding: [u8; 32],
    pub reset_epoch: u64,
    pub reset_day: i64,
    pub lane_day: u8,
    pub streak_before: u32,
    pub streak_after: u32,
    pub jokers_spent: u8,
}

pub(super) async fn load_state(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8; 16],
    world: &[u8; 16],
) -> Result<DailyRewardState, DurabilityError> {
    let row = sqlx::query("SELECT lane_day, streak, last_claim_epoch::text FROM public.game_character_daily_rewards WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(character.as_slice()).bind(world.as_slice()).fetch_optional(&mut **tx).await?;
    let Some(row) = row else {
        return Ok(DailyRewardState::default());
    };
    Ok(DailyRewardState {
        lane_day: u8::try_from(row.try_get::<i16, _>("lane_day")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        streak: u32::try_from(row.try_get::<i32, _>("streak")?)
            .map_err(|_| DurabilityError::InvalidStoredState)?,
        last_claim_epoch: row
            .try_get::<Option<String>, _>("last_claim_epoch")?
            .map(|value| {
                value
                    .parse()
                    .map_err(|_| DurabilityError::InvalidStoredState)
            })
            .transpose()?,
    })
}

pub(super) async fn load_receipt(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8; 16],
    world: &[u8; 16],
    epoch: u64,
) -> Result<Option<DailyRewardReceipt>, DurabilityError> {
    sqlx::query("SELECT transaction_id::text, request_binding, reset_epoch::text, reset_day, lane_day, streak_before, streak_after, jokers_spent FROM public.game_character_daily_reward_claims WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid AND reset_epoch=$3::text::numeric(20,0)")
        .bind(character.as_slice()).bind(world.as_slice()).bind(epoch.to_string()).fetch_optional(&mut **tx).await?.as_ref().map(decode_receipt).transpose()
}

fn decode_receipt(row: &PgRow) -> Result<DailyRewardReceipt, DurabilityError> {
    let invalid = || DurabilityError::InvalidStoredState;
    Ok(DailyRewardReceipt {
        transaction_id: super::item_mint::uuid_text(row.try_get("transaction_id")?)?,
        request_binding: row
            .try_get::<Vec<u8>, _>("request_binding")?
            .try_into()
            .map_err(|_| invalid())?,
        reset_epoch: row
            .try_get::<String, _>("reset_epoch")?
            .parse()
            .map_err(|_| invalid())?,
        reset_day: row.try_get("reset_day")?,
        lane_day: u8::try_from(row.try_get::<i16, _>("lane_day")?).map_err(|_| invalid())?,
        streak_before: u32::try_from(row.try_get::<i32, _>("streak_before")?)
            .map_err(|_| invalid())?,
        streak_after: u32::try_from(row.try_get::<i32, _>("streak_after")?)
            .map_err(|_| invalid())?,
        jokers_spent: u8::try_from(row.try_get::<i16, _>("jokers_spent")?)
            .map_err(|_| invalid())?,
    })
}
