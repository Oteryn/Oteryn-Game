//! Account + World joker ledger, serialized after the native Character root.
use super::DurabilityError;
use super::daily_reward_claim::DailyRewardClaimRequest;
use super::item_transfer::CurrentCharacterItemFence;
use sqlx::{Postgres, Row, Transaction};

pub(super) struct Jokers {
    pub balance: u8,
    account: String,
    month: String,
    entry: i64,
}

pub(super) async fn grant(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    request: DailyRewardClaimRequest,
) -> Result<Jokers, DurabilityError> {
    let world = fence.runtime_scope.world_id();
    let account: String = sqlx::query_scalar("SELECT g.account_id::text FROM public.game_character_roots r JOIN public.game_character_account_guards g USING(account_id) WHERE r.character_id=encode($1,'hex')::uuid FOR UPDATE OF g")
        .bind(fence.character_id.as_bytes().as_slice()).fetch_one(&mut **tx).await?;
    let row = sqlx::query("SELECT jokers,last_grant_month,last_entry_id FROM public.game_account_daily_reward_jokers WHERE account_id=$1::text::uuid AND world_id=encode($2,'hex')::uuid FOR UPDATE")
        .bind(&account).bind(world.as_bytes().as_slice()).fetch_optional(&mut **tx).await?;
    let (before, last, entry): (i16, String, i64) = match row {
        Some(r) => (
            r.try_get("jokers")?,
            r.try_get("last_grant_month")?,
            r.try_get("last_entry_id")?,
        ),
        None => (0, String::new(), 0),
    };
    let months: Vec<String> = sqlx::query_scalar("SELECT DISTINCT to_char(to_timestamp(reset_at_unix_ms/1000.0) AT TIME ZONE 'UTC','YYYY-MM') AS month FROM public.game_world_reset_history WHERE world_id=encode($1,'hex')::uuid AND reset_epoch<=$2::text::numeric AND to_char(to_timestamp(reset_at_unix_ms/1000.0) AT TIME ZONE 'UTC','YYYY-MM')>$3 ORDER BY month DESC LIMIT 3")
        .bind(world.as_bytes().as_slice()).bind(request.reset_epoch.to_string()).bind(&last).fetch_all(&mut **tx).await?;
    let Some(month) = months.first() else {
        return Ok(Jokers {
            balance: before as u8,
            account,
            month: last,
            entry,
        });
    };
    let amount = (months.len() as i16).min(3 - before);
    let next = entry
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    ledger(
        tx,
        &account,
        world.as_bytes(),
        request.transaction_id,
        next,
        "GRANT",
        amount,
        before,
        before + amount,
    )
    .await?;
    sqlx::query("INSERT INTO public.game_account_daily_reward_jokers VALUES($1::text::uuid,encode($2,'hex')::uuid,$3,$4,$5) ON CONFLICT(account_id,world_id) DO UPDATE SET jokers=EXCLUDED.jokers,last_grant_month=EXCLUDED.last_grant_month,last_entry_id=EXCLUDED.last_entry_id")
        .bind(&account).bind(world.as_bytes().as_slice()).bind(before+amount).bind(month).bind(next).execute(&mut **tx).await?;
    Ok(Jokers {
        balance: (before + amount) as u8,
        account,
        month: month.clone(),
        entry: next,
    })
}

pub(super) async fn spend(
    tx: &mut Transaction<'_, Postgres>,
    fence: CurrentCharacterItemFence,
    request: DailyRewardClaimRequest,
    joker: Jokers,
    amount: u8,
) -> Result<(), DurabilityError> {
    if amount == 0 {
        return Ok(());
    }
    let next = joker
        .entry
        .checked_add(1)
        .ok_or(DurabilityError::InvalidStoredState)?;
    let after = joker
        .balance
        .checked_sub(amount)
        .ok_or(DurabilityError::InvalidStoredState)?;
    ledger(
        tx,
        &joker.account,
        fence.runtime_scope.world_id().as_bytes(),
        request.transaction_id,
        next,
        "SPEND",
        i16::from(amount),
        i16::from(joker.balance),
        i16::from(after),
    )
    .await?;
    sqlx::query("UPDATE public.game_account_daily_reward_jokers SET jokers=$3,last_entry_id=$4,last_grant_month=$5 WHERE account_id=$1::text::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(joker.account).bind(fence.runtime_scope.world_id().as_bytes().as_slice()).bind(i16::from(after)).bind(next).bind(joker.month).execute(&mut **tx).await?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn ledger(
    tx: &mut Transaction<'_, Postgres>,
    account: &str,
    world: &[u8; 16],
    claim: [u8; 16],
    entry: i64,
    kind: &str,
    amount: i16,
    before: i16,
    after: i16,
) -> Result<(), DurabilityError> {
    sqlx::query("INSERT INTO public.game_account_daily_reward_joker_entries VALUES($1::text::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid,$5,$6,$7,$8)")
        .bind(account).bind(world.as_slice()).bind(entry).bind(claim.as_slice()).bind(kind).bind(amount).bind(before).bind(after).execute(&mut **tx).await?;
    Ok(())
}
