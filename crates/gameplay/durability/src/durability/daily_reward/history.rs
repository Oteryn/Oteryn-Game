//! Immutable account/World receipts, read inside the caller's current-character transaction.
use super::super::DurabilityError;
use oteryn_protocol_oteryn::daily_reward::details::Receipt;
use sqlx::{Postgres, Row, Transaction};
pub(in crate::durability) async fn load(
    tx: &mut Transaction<'_, Postgres>,
    character: &[u8; 16],
    world: &[u8; 16],
    epoch: u64,
) -> Result<Vec<Receipt>, DurabilityError> {
    let rows=sqlx::query("SELECT c.reset_epoch::text, c.reset_day, c.lane_day, c.streak_after, c.jokers_spent, c.character_id=encode($1,'hex')::uuid AS own_character FROM public.game_character_daily_reward_claims c JOIN public.game_character_roots owner ON owner.character_id=c.character_id JOIN public.game_character_roots subject ON subject.account_id=owner.account_id AND subject.character_id=encode($1,'hex')::uuid WHERE c.world_id=encode($2,'hex')::uuid AND c.reset_epoch<=$3::text::numeric ORDER BY c.reset_epoch DESC,c.transaction_id DESC LIMIT 30")
 .bind(character.as_slice()).bind(world.as_slice()).bind(epoch.to_string()).fetch_all(&mut **tx).await?;
    let invalid = || DurabilityError::InvalidStoredState;
    rows.iter()
        .map(|r| {
            Ok(Receipt {
                epoch: r
                    .try_get::<String, _>("reset_epoch")?
                    .parse()
                    .map_err(|_| invalid())?,
                reset_day: r.try_get("reset_day")?,
                lane_day: u8::try_from(r.try_get::<i16, _>("lane_day")?).map_err(|_| invalid())?,
                streak: u32::try_from(r.try_get::<i32, _>("streak_after")?)
                    .map_err(|_| invalid())?,
                jokers_spent: u8::try_from(r.try_get::<i16, _>("jokers_spent")?)
                    .map_err(|_| invalid())?,
                own_character: r.try_get("own_character")?,
            })
        })
        .collect()
}
