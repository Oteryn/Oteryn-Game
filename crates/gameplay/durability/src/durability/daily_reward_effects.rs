//! DAILYFX sinks borrow the claim transaction and its Character -> Account lock order.
//! Wildcards belong to PREY (Account + World); boost grants remain immutable DAILY lines.
use super::DurabilityError;
use super::daily_reward_claim::DailyRewardClaimRequest;
use super::item_mint_audit::Type2Transaction;
use super::item_transfer::CurrentCharacterItemFence;
use sqlx::{Postgres, Row, Transaction};

pub(super) async fn grant(
    tx: &mut Type2Transaction<'_>,
    fence: CurrentCharacterItemFence,
    request: DailyRewardClaimRequest,
    day: u8,
    premium: bool,
    selection: Option<&oteryn_protocol_oteryn::daily_reward::picker::Claim>,
) -> Result<(), DurabilityError> {
    let character = fence.character_id.as_bytes().as_slice();
    let world = fence.runtime_scope.world_id();
    let claim = request.transaction_id.as_slice();
    let reward = oteryn_gameplay_progression::daily_reward::reward_for_day(day, premium)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let temple = super::daily_reward_picked_items::mint(
        tx,
        fence,
        request,
        day,
        reward.temple_teleport_scrolls,
        selection,
    )
    .await?;
    let mut line: i16 = 0;
    for (kind, quantity) in [
        ("BOUNTY_REROLL", reward.bounty_reroll_tokens),
        ("COSMETIC_TOKEN", reward.cosmetic_tokens),
        ("TEMPLE_SCROLL", reward.temple_teleport_scrolls),
    ] {
        if quantity == 0 {
            continue;
        }
        let item = if kind == "TEMPLE_SCROLL" {
            temple.clone()
        } else {
            grant_balance(tx, fence, request, kind, quantity).await?;
            None
        };
        sqlx::query("INSERT INTO public.game_character_daily_reward_lines(claim_id,line,kind,definition,quantity,status,delivering_transaction_id,item_instance_id) VALUES(encode($1,'hex')::uuid,$2,$3,CASE WHEN $3='TEMPLE_SCROLL' THEN 'oteryn:item.tibia.i25718' END,$4,'DELIVERED',encode($1,'hex')::uuid,$5::uuid)")
            .bind(claim).bind(line).bind(kind).bind(i32::from(quantity)).bind(item)
            .execute(&mut ***tx).await?;
        line += 1;
    }
    if reward.prey_wildcards > 0 {
        // daily_reward_jokers already holds the Account guard, including absent balances.
        let account: String = sqlx::query_scalar("SELECT account_id::text FROM public.game_character_roots WHERE character_id=encode($1,'hex')::uuid")
            .bind(character).fetch_one(&mut ***tx).await?;
        let previous = sqlx::query("SELECT balance,last_claim_id::text FROM public.game_account_prey_wildcards WHERE account_id=$1::text::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
            .bind(&account).bind(world.as_bytes().as_slice()).fetch_optional(&mut ***tx).await?;
        let (before, last): (i16, Option<String>) = match previous {
            Some(row) => (row.try_get("balance")?, Some(row.try_get("last_claim_id")?)),
            None => (0, None),
        };
        let requested = i16::from(reward.prey_wildcards);
        let credited = requested.min(52 - before);
        sqlx::query("INSERT INTO public.game_account_prey_wildcard_entries(claim_id,account_id,world_id,previous_claim_id,kind,requested,credited,balance_before,balance_after) VALUES(encode($1,'hex')::uuid,$2::text::uuid,encode($3,'hex')::uuid,$4::text::uuid,'DAILY_REWARD',$5,$6,$7,$8)")
            .bind(claim).bind(&account).bind(world.as_bytes().as_slice()).bind(last)
            .bind(requested).bind(credited).bind(before).bind(before+credited).execute(&mut ***tx).await?;
        sqlx::query("INSERT INTO public.game_account_prey_wildcards(account_id,world_id,balance,last_claim_id) VALUES($1::text::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid) ON CONFLICT(account_id,world_id) DO UPDATE SET balance=EXCLUDED.balance,last_claim_id=EXCLUDED.last_claim_id")
            .bind(&account).bind(world.as_bytes().as_slice()).bind(before+credited).bind(claim).execute(&mut ***tx).await?;
        sqlx::query("INSERT INTO public.game_character_daily_reward_lines(claim_id,line,kind,quantity,status,delivering_transaction_id) VALUES(encode($1,'hex')::uuid,$3,'PREY_WILDCARD',$2,'DELIVERED',encode($1,'hex')::uuid)")
            .bind(claim).bind(i32::from(requested)).bind(line).execute(&mut ***tx).await?;
        line += 1;
    }
    if reward.xp_boost_percent > 0 {
        let minutes = i32::from(reward.xp_boost_minutes);
        // Crystal 201189f: data/modules/scripts/daily_reward/daily_reward.lua:518-527
        // appends the daily duration to the Store timer instead of replacing it.
        sqlx::query("WITH instant AS (SELECT public.game_daily_effect_now() AS at), endpoint AS (SELECT greatest(instant.at,coalesce((SELECT max(expires_at) FROM public.game_character_store_xp_boosts WHERE character_id=encode($5,'hex')::uuid AND world_id=encode($6,'hex')::uuid),instant.at),coalesce((SELECT max(effect_expires_at) FROM public.game_character_daily_reward_claims c JOIN public.game_character_daily_reward_lines l ON l.claim_id=c.transaction_id WHERE c.character_id=encode($5,'hex')::uuid AND c.world_id=encode($6,'hex')::uuid AND l.kind='XP_BOOST'),instant.at)) AS at FROM instant) INSERT INTO public.game_character_daily_reward_lines(claim_id,line,kind,quantity,status,delivering_transaction_id,effect_started_at,effect_expires_at,xp_boost_percent) SELECT encode($1,'hex')::uuid,$3,'XP_BOOST',$2,'DELIVERED',encode($1,'hex')::uuid,at,at+make_interval(mins=>$2),$4 FROM endpoint")
            .bind(claim).bind(minutes).bind(line).bind(i16::from(reward.xp_boost_percent)).bind(character).bind(world.as_bytes().as_slice()).execute(&mut ***tx).await?;
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
