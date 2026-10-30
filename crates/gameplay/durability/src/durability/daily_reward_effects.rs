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
    let reward = oteryn_gameplay_progression::daily_reward::reward_for_day(day, premium)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let mut line: i16 = 0;
    for (kind, quantity) in [
        ("BOUNTY_REROLL", reward.bounty_reroll_tokens),
        ("COSMETIC_TOKEN", reward.cosmetic_tokens),
        ("TEMPLE_SCROLL", reward.temple_teleport_scrolls),
    ] {
        if quantity == 0 {
            continue;
        }
        if kind != "TEMPLE_SCROLL" {
            grant_balance(tx, fence, request, kind, quantity).await?;
        }
        // i25718 exists as reference content but is not materializable. Retain the
        // entitlement pending admission instead of fabricating a native item.
        sqlx::query("INSERT INTO public.game_character_daily_reward_lines(claim_id,line,kind,quantity,status,delivering_transaction_id) VALUES(encode($1,'hex')::uuid,$2,$3,$4,$5,CASE WHEN $5='DELIVERED' THEN encode($1,'hex')::uuid ELSE NULL END)")
            .bind(claim).bind(line).bind(kind).bind(i32::from(quantity))
            .bind(if kind == "TEMPLE_SCROLL" { "PENDING" } else { "DELIVERED" })
            .execute(&mut **tx).await?;
        line += 1;
    }
    if reward.prey_wildcards > 0 {
        // daily_reward_jokers already holds the Account guard, including absent balances.
        let account: String = sqlx::query_scalar("SELECT account_id::text FROM public.game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(character).fetch_one(&mut **tx).await?;
        let previous = sqlx::query("SELECT balance,last_claim_id::text FROM public.game_account_prey_wildcards WHERE account_id=$1::text::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
            .bind(&account).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
        let (before, last): (i16, Option<String>) = match previous {
            Some(row) => (row.try_get("balance")?, Some(row.try_get("last_claim_id")?)),
            None => (0, None),
        };
        let requested = i16::from(reward.prey_wildcards);
        let credited = requested.min(52 - before);
        sqlx::query("INSERT INTO public.game_account_prey_wildcard_entries(claim_id,account_id,world_id,previous_claim_id,kind,requested,credited,balance_before,balance_after) VALUES(encode($1,'hex')::uuid,$2::text::uuid,encode($3,'hex')::uuid,$4::text::uuid,'DAILY_REWARD',$5,$6,$7,$8)")
            .bind(claim).bind(&account).bind(world.as_bytes().as_slice()).bind(last)
            .bind(requested).bind(credited).bind(before).bind(before+credited).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO public.game_account_prey_wildcards(account_id,world_id,balance,last_claim_id) VALUES($1::text::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid) ON CONFLICT(account_id,world_id) DO UPDATE SET balance=EXCLUDED.balance,last_claim_id=EXCLUDED.last_claim_id")
            .bind(&account).bind(world.as_bytes().as_slice()).bind(before+credited).bind(claim).execute(&mut **tx).await?;
        sqlx::query("INSERT INTO public.game_character_daily_reward_lines(claim_id,line,kind,quantity,status,delivering_transaction_id) VALUES(encode($1,'hex')::uuid,$3,'PREY_WILDCARD',$2,'DELIVERED',encode($1,'hex')::uuid)")
            .bind(claim).bind(i32::from(requested)).bind(line).execute(&mut **tx).await?;
        line += 1;
    }
    if reward.xp_boost_percent > 0 {
        let minutes = i32::from(reward.xp_boost_minutes);
        // Persist a wall-clock interval, not a session timer. Offline time counts.
        sqlx::query("WITH instant AS (SELECT public.game_daily_effect_now() AS at) INSERT INTO public.game_character_daily_reward_lines(claim_id,line,kind,quantity,status,delivering_transaction_id,effect_started_at,effect_expires_at,xp_boost_percent) SELECT encode($1,'hex')::uuid,$3,'XP_BOOST',$2,'DELIVERED',encode($1,'hex')::uuid,at,at+make_interval(mins=>$2),$4 FROM instant")
            .bind(claim).bind(minutes).bind(line).bind(i16::from(reward.xp_boost_percent)).execute(&mut **tx).await?;
    }
    Ok(())
}

/// Character root serialization is already held by the native claim boundary.
async fn grant_balance(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    request: DailyRewardClaimRequest,
    kind: &str,
    quantity: u8,
) -> Result<(), DurabilityError> {
    // SQL identifiers come exclusively from this closed server-owned match.
    let (balance_table, entries_table) = match kind {
        "COSMETIC_TOKEN" => (
            "game_character_cosmetic_tokens",
            "game_character_cosmetic_token_entries",
        ),
        "BOUNTY_REROLL" => (
            "game_character_bounty_reroll_tokens",
            "game_character_bounty_reroll_token_entries",
        ),
        _ => return Err(DurabilityError::InvalidStoredState),
    };
    let world = fence.runtime_scope.world_id();
    let previous = sqlx::query(sqlx::AssertSqlSafe(format!("SELECT balance,last_claim_id::text FROM public.{balance_table} WHERE character_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")))
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice())
        .fetch_optional(&mut **tx).await?;
    let (before, last): (i64, Option<String>) = match previous {
        Some(row) => (row.try_get("balance")?, Some(row.try_get("last_claim_id")?)),
        None => (0, None),
    };
    let after = before
        .checked_add(i64::from(quantity))
        .ok_or(DurabilityError::InvalidStoredState)?;
    sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO public.{entries_table}(claim_id,character_id,world_id,previous_claim_id,amount,balance_before,balance_after) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4::text::uuid,$5,$6,$7)")))
        .bind(request.transaction_id.as_slice()).bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice())
        .bind(last).bind(i16::from(quantity)).bind(before).bind(after).execute(&mut **tx).await?;
    sqlx::query(sqlx::AssertSqlSafe(format!("INSERT INTO public.{balance_table}(character_id,world_id,balance,last_claim_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid) ON CONFLICT(character_id,world_id) DO UPDATE SET balance=EXCLUDED.balance,last_claim_id=EXCLUDED.last_claim_id")))
        .bind(fence.character_id.as_bytes().as_slice()).bind(world.as_bytes().as_slice()).bind(after).bind(request.transaction_id.as_slice()).execute(&mut **tx).await?;
    Ok(())
}
