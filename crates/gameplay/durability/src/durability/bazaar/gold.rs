//! Account+World bank holds, incoming credit reservations and exact settlement.
//! All paths serialize on the existing bank balance rows; this owns no balance.
use super::*;
use serde_json::Value;
use sqlx::postgres::PgRow;

const CONFLICT: &str = "CHAR_CMD_BAZAAR_HOLD_CONFLICT";
const STALE: &str = "CHAR_CMD_BAZAAR_STALE_REVISION";
const PAYOUT: &str = "CHAR_CMD_BAZAAR_PAYOUT_INVALID";
const CEILING: u64 = 999_999_999_999;

fn s<'a>(v: &'a Value, key: &str) -> Result<&'a str, DurabilityError> {
    v[key].as_str().ok_or(DurabilityError::InvalidStoredState)
}
fn n(v: &Value, key: &str) -> Result<u64, DurabilityError> {
    s(v, key)?
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)
}
fn unsigned(row: &PgRow, key: &str) -> Result<u64, DurabilityError> {
    u64::try_from(row.try_get::<i64, _>(key)?).map_err(|_| DurabilityError::InvalidStoredState)
}
fn revision(row: &PgRow) -> Result<u64, DurabilityError> {
    row.try_get::<String, _>("revision")?
        .parse()
        .map_err(|_| DurabilityError::InvalidStoredState)
}
fn net(gross: u64) -> Option<u64> {
    let fee = u128::from(gross).checked_mul(250)?.div_ceil(10000);
    u64::try_from(u128::from(gross).checked_sub(fee)?)
        .ok()
        .filter(|v| *v > 0)
}
fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect()
}
async fn mint_id(tx: &mut Tx<'_>) -> Result<String, DurabilityError> {
    Ok(sqlx::query_scalar("SELECT game_character_uuid_v7()::text")
        .fetch_one(&mut **tx)
        .await?)
}

async fn lock_banks(
    tx: &mut Tx<'_>,
    buyer: &str,
    seller: &str,
    world: &str,
) -> Result<(), DurabilityError> {
    for account in if buyer < seller {
        [buyer, seller]
    } else {
        [seller, buyer]
    } {
        sqlx::query("INSERT INTO game_account_bank_balances(account_id,world_id,balance,last_entry_id) VALUES($1::uuid,$2::uuid,0,NULL) ON CONFLICT DO NOTHING").bind(account).bind(world).execute(&mut **tx).await?;
        sqlx::query("SELECT 1 FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=$2::uuid FOR UPDATE").bind(account).bind(world).fetch_one(&mut **tx).await?;
    }
    Ok(())
}

async fn totals(
    tx: &mut Tx<'_>,
    account: &str,
    world: &str,
) -> Result<(u64, u64, u64), DurabilityError> {
    let row=sqlx::query("SELECT balance,(SELECT coalesce(sum(amount),0)::bigint FROM game_bazaar_gold_holds WHERE buyer_account_id=$1::uuid AND world_id=$2::uuid AND amount>0) held,(SELECT coalesce(sum(incoming_net),0)::bigint FROM game_bazaar_gold_holds WHERE seller_account_id=$1::uuid AND world_id=$2::uuid AND incoming_net>0) incoming FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=$2::uuid").bind(account).bind(world).fetch_one(&mut **tx).await?;
    Ok((
        unsigned(&row, "balance")?,
        unsigned(&row, "held")?,
        unsigned(&row, "incoming")?,
    ))
}

fn parties_match(row: &PgRow, v: &Value) -> Result<bool, DurabilityError> {
    Ok(
        row.try_get::<String, _>("buyer_account_id")? == s(v, "buyer_account_id")?
            && row.try_get::<String, _>("seller_account_id")? == s(v, "seller_account_id")?
            && row.try_get::<String, _>("world_id")? == s(v, "world_id")?
            && row.try_get::<String, _>("listing_id")? == s(v, "listing_id")?,
    )
}

async fn hold_row(tx: &mut Tx<'_>, hold: &str) -> Result<Option<PgRow>, DurabilityError> {
    Ok(sqlx::query("SELECT hold_id::text,listing_id::text,offer_id::text,buyer_account_id::text,seller_account_id::text,world_id::text,purpose,revision::text,amount,incoming_net,state,sale_id::text,gross_amount,delivery_operation_id::text,delivery_receipt_digest FROM game_bazaar_gold_holds WHERE hold_id=$1::uuid FOR UPDATE").bind(hold).fetch_optional(&mut **tx).await?)
}

pub(super) async fn apply(
    tx: &mut Tx<'_>,
    r: &Request,
    p: &Policy,
    v: &Value,
    _now: i64,
) -> Result<Effect, DurabilityError> {
    if p.gold_fee_bps != Some(250) || !p.reserve_incoming_headroom || !p.common_restore_scope {
        return Ok(Effect::Pending);
    }
    match r.command.as_str() {
        "HoldBazaarGold" => hold(tx, v).await,
        "ReleaseBazaarGold" => release(tx, r, v).await,
        "PayoutBazaarGold" => payout(tx, r, v).await,
        _ => Ok(Effect::Rejected("CHAR_CMD_UNSUPPORTED")),
    }
}

async fn hold(tx: &mut Tx<'_>, v: &Value) -> Result<Effect, DurabilityError> {
    let buyer = s(v, "buyer_account_id")?;
    let seller = s(v, "seller_account_id")?;
    let world = s(v, "world_id")?;
    let amount = n(v, "amount")?;
    let expected = n(v, "expected_hold_revision")?;
    if sqlx::query("SELECT 1 FROM game_character_roots WHERE character_id=$1::uuid AND account_id=$2::uuid AND world_id=$3::uuid AND lifecycle=1 FOR SHARE").bind(s(v,"buyer_character_id")?).bind(buyer).bind(world).fetch_optional(&mut **tx).await?.is_none(){return Ok(Effect::Rejected("CHAR_CMD_OWNER_MISMATCH"));}
    let Some(incoming) = net(amount) else {
        return Ok(Effect::Rejected(CONFLICT));
    };
    lock_banks(tx, buyer, seller, world).await?;
    let existing=sqlx::query("SELECT hold_id::text,revision::text,amount,incoming_net,state,purpose,seller_account_id::text FROM game_bazaar_gold_holds WHERE listing_id=$1::uuid AND offer_id=$2::uuid AND buyer_account_id=$3::uuid AND world_id=$4::uuid FOR UPDATE").bind(s(v,"listing_id")?).bind(s(v,"offer_id")?).bind(buyer).bind(world).fetch_optional(&mut **tx).await?;
    let (hold_id, old_amount, old_incoming, new_revision) = match existing {
        Some(row) => {
            if revision(&row)? != expected {
                return Ok(Effect::Rejected(STALE));
            }
            if row.try_get::<String, _>("state")? != "ACTIVE"
                || row.try_get::<String, _>("purpose")? != s(v, "hold_purpose")?
                || row.try_get::<String, _>("seller_account_id")? != seller
                || amount < unsigned(&row, "amount")?
            {
                return Ok(Effect::Rejected(CONFLICT));
            }
            (
                row.try_get::<String, _>("hold_id")?,
                unsigned(&row, "amount")?,
                unsigned(&row, "incoming_net")?,
                expected
                    .checked_add(1)
                    .ok_or(DurabilityError::Unavailable)?,
            )
        }
        None => {
            if expected != 0 {
                return Ok(Effect::Rejected(STALE));
            }
            (mint_id(tx).await?, 0, 0, 1)
        }
    };
    let (balance, held, _) = totals(tx, buyer, world).await?;
    let (seller_balance, _, reserved) = totals(tx, seller, world).await?;
    let native_headroom: i64 = sqlx::query_scalar(
        "SELECT game_account_bank_credit_headroom_before_bazaar($1::uuid,$2::uuid)",
    )
    .bind(seller)
    .bind(world)
    .fetch_one(&mut **tx)
    .await?;
    // Existing native custody can return to this bank. Preserve its already
    // reserved value before promising any new incoming Bazaar proceeds.
    let native_reserved_total = 9_000_000_000_000_000_u64
        .checked_sub(
            u64::try_from(native_headroom).map_err(|_| DurabilityError::InvalidStoredState)?,
        )
        .ok_or(DurabilityError::InvalidStoredState)?;
    if balance > CEILING
        || held > balance
        || amount - old_amount > balance - held
        || seller_balance
            .max(native_reserved_total)
            .checked_add(reserved)
            .and_then(|n| n.checked_sub(old_incoming))
            .and_then(|n| n.checked_add(incoming))
            .is_none_or(|n| n > CEILING)
    {
        return Ok(Effect::Rejected(CONFLICT));
    }
    if expected == 0 {
        sqlx::query("INSERT INTO game_bazaar_gold_holds(hold_id,listing_id,offer_id,buyer_account_id,seller_account_id,world_id,purpose,revision,amount,incoming_net,state) VALUES($1::uuid,$2::uuid,$3::uuid,$4::uuid,$5::uuid,$6::uuid,$7,1,$8,$9,'ACTIVE')").bind(&hold_id).bind(s(v,"listing_id")?).bind(s(v,"offer_id")?).bind(buyer).bind(seller).bind(world).bind(s(v,"hold_purpose")?).bind(amount as i64).bind(incoming as i64).execute(&mut **tx).await?;
    } else {
        sqlx::query("UPDATE game_bazaar_gold_holds SET revision=$2::numeric,amount=$3,incoming_net=$4 WHERE hold_id=$1::uuid").bind(&hold_id).bind(new_revision.to_string()).bind(amount as i64).bind(incoming as i64).execute(&mut **tx).await?;
    }
    Ok(Effect::Committed(format!(
        r#"{{"hold_id":"{hold_id}","hold_revision":"{new_revision}","buyer_account_id":"{buyer}","offer_id":"{}","hold_purpose":"{}","held_amount":"{amount}"}}"#,
        s(v, "offer_id")?,
        s(v, "hold_purpose")?
    )))
}

async fn release(tx: &mut Tx<'_>, r: &Request, v: &Value) -> Result<Effect, DurabilityError> {
    let buyer = s(v, "buyer_account_id")?;
    let seller = s(v, "seller_account_id")?;
    let world = s(v, "world_id")?;
    lock_banks(tx, buyer, seller, world).await?;
    let Some(row) = hold_row(tx, s(v, "hold_id")?).await? else {
        return Ok(Effect::Rejected(CONFLICT));
    };
    if !parties_match(&row, v)? {
        return Ok(Effect::Rejected(CONFLICT));
    }
    if revision(&row)? != n(v, "expected_hold_revision")? {
        return Ok(Effect::Rejected(STALE));
    }
    let state: String = row.try_get("state")?;
    if state == "RELEASED" || state == "AWAITING_PAYOUT" {
        return Ok(Effect::Rejected(CONFLICT));
    }
    if state == "PAID" && s(v, "release_reason")? != "EXCESS" {
        return Ok(Effect::Rejected(CONFLICT));
    }
    if state == "DISPOSITION_PENDING" {
        if s(v, "release_reason")? != "TERMINAL_REFUSAL" {
            return Ok(Effect::Rejected(CONFLICT));
        }
        let Some(operation) = row.try_get::<Option<String>, _>("delivery_operation_id")? else {
            return Err(DurabilityError::InvalidStoredState);
        };
        // Only a committed Character bind persists DISPOSITION_PENDING across
        // transactions. Item delivery either commits AWAITING_PAYOUT together
        // with its receipt, or rolls its temporary fence back with the effect.
        let proofs=sqlx::query("SELECT result_code,recovery_generation::text,intent FROM game_bazaar_decisions WHERE operation_id=$1::uuid AND issuer='OTERYN_PLATFORM_CHARACTER_AUTHORITY' AND command='TransferCharacterOwnership' LIMIT 1").bind(operation).fetch_all(&mut **tx).await?;
        if proofs.len() != 1 {
            return Ok(Effect::Pending);
        }
        let proof = &proofs[0];
        if proof.try_get::<String, _>("recovery_generation")? != r.generation.to_string() {
            return Ok(Effect::Pending);
        }
        if proof.try_get::<String, _>("result_code")? == OK {
            return Ok(Effect::Rejected(CONFLICT));
        }
    }
    let amount = n(v, "amount")?;
    let held = unsigned(&row, "amount")?;
    if amount > held {
        return Ok(Effect::Rejected(CONFLICT));
    }
    let remaining = held - amount;
    let next = revision(&row)?
        .checked_add(1)
        .ok_or(DurabilityError::Unavailable)?;
    let incoming = net(remaining)
        .unwrap_or(0)
        .min(unsigned(&row, "incoming_net")?);
    let next_state = if remaining == 0 {
        "RELEASED"
    } else if state == "PAID" {
        "PAID"
    } else {
        "ACTIVE"
    };
    // A verified terminal refusal ends this binding, including when only part
    // of its absolute reservation is released. The remainder is a fresh ACTIVE
    // maximum; stale sale metadata must not make it executable or unrestorable.
    let refused = state == "DISPOSITION_PENDING";
    sqlx::query("UPDATE game_bazaar_gold_holds SET amount=$2,incoming_net=$3,revision=$4::numeric,state=$5,sale_id=CASE WHEN $6 THEN NULL ELSE sale_id END,gross_amount=CASE WHEN $6 THEN NULL ELSE gross_amount END,delivery_operation_id=CASE WHEN $6 THEN NULL ELSE delivery_operation_id END,delivery_receipt_digest=CASE WHEN $6 THEN NULL ELSE delivery_receipt_digest END WHERE hold_id=$1::uuid").bind(s(v,"hold_id")?).bind(remaining as i64).bind(incoming as i64).bind(next.to_string()).bind(next_state).bind(refused).execute(&mut **tx).await?;
    Ok(Effect::Committed(format!(
        r#"{{"hold_id":"{}","hold_revision":"{next}","released_amount":"{amount}","remaining_held":"{remaining}","hold_state":"{}"}}"#,
        s(v, "hold_id")?,
        if remaining == 0 { "RELEASED" } else { "ACTIVE" }
    )))
}

/// Fence the one exact Game hold before a delivery or bound transfer is executable.
/// COINS never authorizes a Game bank write: Platform owns that reservation fence.
#[allow(
    clippy::too_many_arguments,
    reason = "exact funding saga identity and delivery occurrence"
)]
pub(super) async fn fence_funding(
    tx: &mut Tx<'_>,
    r: &Request,
    funding: &Value,
    buyer: &str,
    sale: &str,
    delivery_operation: &str,
) -> Result<Effect, DurabilityError> {
    if s(funding, "currency")? == "COINS" {
        return Ok(Effect::Committed(String::new()));
    }
    let configuration = sqlx::query(
        "SELECT enabled,policy::text FROM game_bazaar_configuration WHERE singleton FOR SHARE",
    )
    .fetch_one(&mut **tx)
    .await?;
    let policy: Policy = serde_json::from_str(configuration.try_get("policy")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    if !configuration.try_get::<bool, _>("enabled")?
        || policy.gold_fee_bps != Some(250)
        || !policy.reserve_incoming_headroom
        || !policy.common_restore_scope
    {
        return Ok(Effect::Pending);
    }
    let v: Value =
        serde_json::from_slice(&r.raw).map_err(|_| DurabilityError::InvalidStoredState)?;
    let seller = s(&v, "seller_account_id")?;
    let world = s(&v, "world_id")?;
    lock_banks(tx, buyer, seller, world).await?;
    let Some(row) = hold_row(tx, s(funding, "reservation_id")?).await? else {
        return Ok(Effect::Rejected(CONFLICT));
    };
    if row.try_get::<String, _>("buyer_account_id")? != buyer
        || row.try_get::<String, _>("seller_account_id")? != seller
        || row.try_get::<String, _>("world_id")? != world
        || row.try_get::<String, _>("listing_id")? != s(&v, "listing_id")?
    {
        return Ok(Effect::Rejected(CONFLICT));
    }
    if revision(&row)? != n(funding, "hold_revision")? {
        return Ok(Effect::Rejected(STALE));
    }
    let gross = n(funding, "gross_amount")?;
    let Some(proceeds) = net(gross) else {
        return Ok(Effect::Rejected(CONFLICT));
    };
    if gross > unsigned(&row, "amount")? || proceeds > unsigned(&row, "incoming_net")? {
        return Ok(Effect::Rejected(CONFLICT));
    }
    let state: String = row.try_get("state")?;
    if state == "DISPOSITION_PENDING"
        && row.try_get::<Option<String>, _>("sale_id")?.as_deref() == Some(sale)
        && row
            .try_get::<Option<String>, _>("delivery_operation_id")?
            .as_deref()
            == Some(delivery_operation)
        && row.try_get::<Option<i64>, _>("gross_amount")? == Some(gross as i64)
    {
        return Ok(Effect::Committed(String::new()));
    }
    if state != "ACTIVE" {
        return Ok(Effect::Rejected(CONFLICT));
    }
    let (balance, held, _) = totals(tx, buyer, world).await?;
    let (seller_balance, _, incoming) = totals(tx, seller, world).await?;
    if held > balance
        || balance > CEILING
        || seller_balance
            .checked_add(incoming)
            .is_none_or(|n| n > CEILING)
    {
        return Ok(Effect::Rejected(CONFLICT));
    }
    sqlx::query("UPDATE game_bazaar_gold_holds SET state='DISPOSITION_PENDING',sale_id=$2::uuid,gross_amount=$3,delivery_operation_id=$4::uuid WHERE hold_id=$1::uuid").bind(s(funding,"reservation_id")?).bind(sale).bind(gross as i64).bind(delivery_operation).execute(&mut **tx).await?;
    Ok(Effect::Committed(String::new()))
}

#[allow(
    clippy::too_many_arguments,
    reason = "exact funded delivery acknowledgement binding"
)]
pub(super) async fn delivery_committed(
    tx: &mut Tx<'_>,
    r: &Request,
    funding: &Value,
    buyer: &str,
    sale: &str,
    delivery_operation: &str,
    receipt_digest: &[u8],
) -> Result<(), DurabilityError> {
    if s(funding, "currency")? == "COINS" {
        return Ok(());
    }
    let changed=sqlx::query("UPDATE game_bazaar_gold_holds SET state='AWAITING_PAYOUT',delivery_receipt_digest=$5 WHERE hold_id=$1::uuid AND buyer_account_id=$2::uuid AND sale_id=$3::uuid AND delivery_operation_id=$4::uuid AND state='DISPOSITION_PENDING'")
        .bind(s(funding,"reservation_id")?).bind(buyer).bind(sale).bind(delivery_operation).bind(receipt_digest).execute(&mut **tx).await?.rows_affected();
    let _ = r;
    if changed != 1 {
        return Err(DurabilityError::Unavailable);
    }
    Ok(())
}

async fn payout(tx: &mut Tx<'_>, r: &Request, v: &Value) -> Result<Effect, DurabilityError> {
    let buyer = s(v, "buyer_account_id")?;
    let seller = s(v, "seller_account_id")?;
    let world = s(v, "world_id")?;
    lock_banks(tx, buyer, seller, world).await?;
    let Some(row) = hold_row(tx, s(v, "hold_id")?).await? else {
        return Ok(Effect::Rejected(CONFLICT));
    };
    if !parties_match(&row, v)? {
        return Ok(Effect::Rejected(CONFLICT));
    }
    if revision(&row)? != n(v, "expected_hold_revision")? {
        return Ok(Effect::Rejected(STALE));
    }
    let gross = n(v, "gross_amount")?;
    let fee = n(v, "fee_amount")?;
    let proceeds = n(v, "seller_amount")?;
    if net(gross) != Some(proceeds)
        || gross.checked_sub(proceeds) != Some(fee)
        || v["fee_bps"].as_u64() != Some(250)
    {
        return Ok(Effect::Rejected(PAYOUT));
    }
    if row.try_get::<String, _>("state")? != "AWAITING_PAYOUT"
        || row.try_get::<Option<String>, _>("sale_id")?.as_deref() != Some(s(v, "sale_id")?)
        || row.try_get::<Option<i64>, _>("gross_amount")? != Some(gross as i64)
        || row
            .try_get::<Option<String>, _>("delivery_operation_id")?
            .as_deref()
            != Some(s(v, "delivery_operation_id")?)
        || row
            .try_get::<Option<Vec<u8>>, _>("delivery_receipt_digest")?
            .as_deref()
            .map(hex)
            .as_deref()
            != Some(s(v, "delivery_receipt_digest")?)
    {
        return Ok(Effect::Rejected(CONFLICT));
    }
    let proofs=sqlx::query("SELECT issuer,command,intent,receipt,receipt_digest,recovery_generation::text,result_code FROM game_bazaar_decisions WHERE operation_id=$1::uuid AND receipt_digest=decode($2,'hex') AND result_code='CHAR_CMD_OK' AND command IN('DeliverBazaarItem','TransferCharacterOwnership') LIMIT 2").bind(s(v,"delivery_operation_id")?).bind(s(v,"delivery_receipt_digest")?).fetch_all(&mut **tx).await?;
    if proofs.len() != 1 {
        return Ok(Effect::Pending);
    }
    let proof = &proofs[0];
    if proof.try_get::<String, _>("recovery_generation")? != r.generation.to_string()
        || hex(&proof.try_get::<Vec<u8>, _>("receipt_digest")?) != s(v, "delivery_receipt_digest")?
    {
        return Ok(Effect::Pending);
    }
    let delivered: Value = serde_json::from_slice(&proof.try_get::<Vec<u8>, _>("intent")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let receipt: Value = serde_json::from_slice(&proof.try_get::<Vec<u8>, _>("receipt")?)
        .map_err(|_| DurabilityError::InvalidStoredState)?;
    let command: String = proof.try_get("command")?;
    let (buyer_character, seller_character) = if command == "DeliverBazaarItem" {
        if s(&delivered, "listing_id")? != s(v, "listing_id")?
            || s(&delivered, "seller_account_id")? != seller
            || s(&delivered, "world_id")? != world
            || s(&delivered, "buyer_account_id")? != buyer
            || s(&delivered, "sale_id")? != s(v, "sale_id")?
            || n(&delivered["funding"], "gross_amount")? != gross
            || s(&delivered["funding"], "currency")? != "GOLD"
            || s(&delivered["funding"], "reservation_id")? != s(v, "hold_id")?
            || n(&delivered["funding"], "hold_revision")? != n(v, "expected_hold_revision")?
        {
            return Ok(Effect::Rejected(PAYOUT));
        }
        let seller_character:String=sqlx::query_scalar("SELECT seller_character_id::text FROM game_bazaar_escrows WHERE escrow_id=$1::uuid AND listing_id=$2::uuid AND seller_account_id=$3::uuid AND world_id=$4::uuid AND state='DELIVERED'").bind(s(&delivered,"escrow_id")?).bind(s(v,"listing_id")?).bind(seller).bind(world).fetch_one(&mut **tx).await?;
        (
            s(&delivered, "buyer_character_id")?.to_owned(),
            seller_character,
        )
    } else if command == "TransferCharacterOwnership" && delivered["purpose"] == "BAZAAR_SETTLEMENT"
    {
        if s(&receipt, "to_account_id")? != buyer
            || s(&delivered, "to_account_id")? != buyer
            || s(&receipt, "character_id")? != s(&delivered, "character_id")?
            || s(&receipt, "from_account_id")? != s(&delivered, "expected_from_account_id")?
        {
            return Ok(Effect::Rejected(PAYOUT));
        }
        // Unlock advances the listing's last operation after a legitimate
        // settlement. Its retained sale binding and immutable ownership
        // transition, rather than that mutable pointer, prove delivery.
        let locked=sqlx::query("SELECT l.character_id::text FROM game_bazaar_locks l JOIN game_bazaar_character_transitions t ON t.character_id=l.character_id AND t.issuer='OTERYN_PLATFORM_CHARACTER_AUTHORITY' AND t.operation_id=$5::uuid WHERE l.listing_id=$1::uuid AND l.seller_account_id=$2::uuid AND l.world_id=$3::uuid AND t.new_account=$4::uuid AND l.transfer_buyer=$4::uuid AND l.transfer_sale_id=$6::uuid AND t.new_revision=$7::numeric AND t.old_account=$8::uuid AND l.character_id=$9::uuid AND convert_from(l.transfer_funding,'UTF8')::jsonb->>'reservation_id'=$10 AND convert_from(l.transfer_funding,'UTF8')::jsonb->>'gross_amount'=$11 AND convert_from(l.transfer_funding,'UTF8')::jsonb->>'currency'='GOLD' AND convert_from(l.transfer_funding,'UTF8')::jsonb->>'hold_revision'=$12 FOR SHARE OF l")
            .bind(s(v,"listing_id")?).bind(seller).bind(world).bind(buyer)
            .bind(s(v,"delivery_operation_id")?).bind(s(v,"sale_id")?)
            .bind(s(&receipt,"character_revision")?).bind(s(&delivered,"expected_from_account_id")?)
            .bind(s(&receipt,"character_id")?).bind(s(v,"hold_id")?).bind(gross.to_string())
            .bind(n(v,"expected_hold_revision")?.to_string())
            .fetch_optional(&mut **tx).await?;
        let Some(locked) = locked else {
            return Ok(Effect::Rejected(PAYOUT));
        };
        let character: String = locked.try_get("character_id")?;
        (character.clone(), character)
    } else {
        return Ok(Effect::Rejected(PAYOUT));
    };
    let amount = unsigned(&row, "amount")?;
    let reserved = unsigned(&row, "incoming_net")?;
    let (buyer_balance, held, _) = totals(tx, buyer, world).await?;
    let (seller_balance, _, incoming) = totals(tx, seller, world).await?;
    if gross > amount
        || held > buyer_balance
        || proceeds > reserved
        || seller_balance
            .checked_add(incoming)
            .is_none_or(|n| n > CEILING)
    {
        return Ok(Effect::Rejected(CONFLICT));
    }
    let remaining = amount - gross;
    let next = revision(&row)?
        .checked_add(1)
        .ok_or(DurabilityError::Unavailable)?;
    // Consume exactly this payout's net headroom. Any unused maximum and its
    // remaining incoming reservation survive until explicit EXCESS release.
    sqlx::query("UPDATE game_bazaar_gold_holds SET amount=$2,incoming_net=$5,state=$3,revision=$4::numeric WHERE hold_id=$1::uuid").bind(s(v,"hold_id")?).bind(remaining as i64).bind(if remaining==0{"RELEASED"}else{"PAID"}).bind(next.to_string()).bind((reserved-proceeds) as i64).execute(&mut **tx).await?;
    let debit_id = mint_id(tx).await?;
    let credit_id = mint_id(tx).await?;
    let burn_id = mint_id(tx).await?;
    bank_entry(
        tx,
        r,
        buyer,
        world,
        &buyer_character,
        &debit_id,
        gross,
        false,
    )
    .await?;
    bank_entry(
        tx,
        r,
        seller,
        world,
        &seller_character,
        &credit_id,
        proceeds,
        true,
    )
    .await?;
    sqlx::query("INSERT INTO game_bazaar_gold_burns(burn_entry_id,issuer,operation_id,world_id,amount) VALUES($1::uuid,$2,$3::uuid,$4::uuid,$5)").bind(&burn_id).bind(&r.issuer).bind(&r.operation).bind(world).bind(fee as i64).execute(&mut **tx).await?;
    Ok(Effect::Committed(format!(
        r#"{{"hold_id":"{}","hold_revision":"{next}","sale_id":"{}","gross_amount":"{gross}","fee_amount":"{fee}","seller_amount":"{proceeds}","remaining_held":"{remaining}","buyer_ledger_entry_id":"{debit_id}","seller_ledger_entry_id":"{credit_id}","burn_entry_id":"{burn_id}","delivery_operation_id":"{}","delivery_receipt_digest":"{}"}}"#,
        s(v, "hold_id")?,
        s(v, "sale_id")?,
        s(v, "delivery_operation_id")?,
        s(v, "delivery_receipt_digest")?
    )))
}

#[allow(
    clippy::too_many_arguments,
    reason = "single bank ledger entry identity and exact account/world binding"
)]
async fn bank_entry(
    tx: &mut Tx<'_>,
    r: &Request,
    account: &str,
    world: &str,
    character: &str,
    entry: &str,
    amount: u64,
    credit: bool,
) -> Result<(), DurabilityError> {
    let row=sqlx::query("SELECT balance,last_entry_id::text FROM game_account_bank_balances WHERE account_id=$1::uuid AND world_id=$2::uuid FOR UPDATE").bind(account).bind(world).fetch_one(&mut **tx).await?;
    let before = unsigned(&row, "balance")?;
    let after = if credit {
        before.checked_add(amount)
    } else {
        before.checked_sub(amount)
    }
    .filter(|n| *n <= CEILING)
    .ok_or(DurabilityError::Unavailable)?;
    sqlx::query("INSERT INTO game_account_bank_entries(entry_id,account_id,world_id,kind,amount,balance_before,balance_after,previous_entry_id,acting_character_id,bazaar_issuer,bazaar_operation_id,bazaar_kind) VALUES($1::uuid,$2::uuid,$3::uuid,NULL,$4,$5,$6,$7::uuid,$8::uuid,$9,$10::uuid,$11)").bind(entry).bind(account).bind(world).bind(amount as i64).bind(before as i64).bind(after as i64).bind(row.try_get::<Option<String>,_>("last_entry_id")?).bind(character).bind(&r.issuer).bind(&r.operation).bind(if credit{"BAZAAR_CREDIT"}else{"BAZAAR_DEBIT"}).execute(&mut **tx).await?;
    sqlx::query("UPDATE game_account_bank_balances SET balance=$3,last_entry_id=$4::uuid WHERE account_id=$1::uuid AND world_id=$2::uuid").bind(account).bind(world).bind(after as i64).bind(entry).execute(&mut **tx).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::net;
    #[test]
    fn bazaar_gold_fee_rounds_up_without_overflow_or_zero_net() {
        assert_eq!(net(0), None);
        assert_eq!(net(1), None);
        assert_eq!(net(2), Some(1));
        assert_eq!(net(10000), Some(9750));
        assert_eq!(net(999_999_999_999), Some(974_999_999_999));
    }
}
