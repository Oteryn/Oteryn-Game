//! DAILYFX sinks borrow the claim transaction and its Character -> Account lock order.
//! Wildcards belong to PREY (Account + World); boost grants remain immutable DAILY lines.
use super::DurabilityError;
use super::daily_reward_claim::DailyRewardClaimRequest;
use super::item_transfer::CurrentCharacterItemFence;
use sqlx::{Postgres, Row, Transaction};

pub(super) async fn grant(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    request: DailyRewardClaimRequest,
    day: u8,
    premium: bool,
) -> Result<(), DurabilityError> {
    let character = fence.character_id.as_bytes().as_slice();
    let world = fence.runtime_scope.world_id();
    let claim = request.transaction_id.as_slice();
    if day == 3 {
        // daily_reward_jokers already holds the Account guard, including absent balances.
        let account: String = sqlx::query_scalar("SELECT account_id::text FROM public.game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(character).fetch_one(&mut **tx).await?;
        let previous = sqlx::query("SELECT balance,last_claim_id::text FROM public.game_account_prey_wildcards WHERE account_id=$1::text::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
            .bind(&account).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        let (before, last): (i16, Option<String>) = match previous {
            Some(row) => (row.try_get("balance")?, Some(row.try_get("last_claim_id")?)),
            None => (0, None),
        };
        let requested: i16 = if premium { 3 } else { 1 };
        let credited = requested.min(52 - before);
        sqlx::query("INSERT INTO public.game_account_prey_wildcard_entries(claim_id,account_id,world_id,previous_claim_id,kind,requested,credited,balance_before,balance_after) VALUES(encode($1,'hex')::uuid,$2::text::uuid,encode($3,'hex')::uuid,$4::text::uuid,'DAILY_REWARD',$5,$6,$7,$8)")
            .bind(claim).bind(&account).bind(world.as_bytes().as_slice()).bind(last)
            .bind(requested).bind(credited).bind(before).bind(before+credited).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO public.game_account_prey_wildcards(account_id,world_id,balance,last_claim_id) VALUES($1::text::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid) ON CONFLICT(account_id,world_id) DO UPDATE SET balance=EXCLUDED.balance,last_claim_id=EXCLUDED.last_claim_id")
            .bind(&account).bind(world.as_bytes().as_slice()).bind(before+credited).bind(claim).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO public.game_character_daily_reward_lines(claim_id,line,kind,quantity,status,delivering_transaction_id) VALUES(encode($1,'hex')::uuid,0,'PREY_WILDCARD',$2,'DELIVERED',encode($1,'hex')::uuid)")
            .bind(claim).bind(i32::from(requested)).execute(&mut **tx).await?;
    } else {
        let minutes: i32 = if premium { 30 } else { 10 };
        // Persist a wall-clock interval, not a session timer. Offline time counts.
        sqlx::query("WITH instant AS (SELECT public.game_daily_effect_now() AS at) INSERT INTO public.game_character_daily_reward_lines(claim_id,line,kind,quantity,status,delivering_transaction_id,effect_started_at,effect_expires_at) SELECT encode($1,'hex')::uuid,0,'XP_BOOST',$2,'DELIVERED',encode($1,'hex')::uuid,at,at+make_interval(mins=>$2) FROM instant")
            .bind(claim).bind(minutes).execute(&mut **tx).await?;
    }
    Ok(())
}
