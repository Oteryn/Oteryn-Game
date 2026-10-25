//! Initial retained Account balance snapshots, before exercising any mechanic. Native runtime
//! writes and negative SQL probes below run with every production guard enabled.
use super::*;
pub(super) async fn balance(
    h: &Harness,
    account: [u8; 16],
    actor: [u8; 16],
    amount: i64,
    tag: u8,
) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    // This is a retained prefix anchor, like BANK-1's historical balance fixture. Only the
    // tested suffix is authored by the runtime; no claim or Guild financial row is seeded.
    sqlx::query("INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,sha256('seed'),encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,1,1,0,1,sha256('seed'),0,encode($8,'hex')::uuid,encode($9,'hex')::uuid,encode($10,'hex')::uuid)")
        .bind(id(tag).as_slice()).bind(id(tag+1).as_slice()).bind(id(tag+2).as_slice()).bind(actor.as_slice()).bind(account.as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(id(tag+3).as_slice()).bind(id(tag+4).as_slice()).bind(id(tag+5).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,acting_character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,1,$5-1,$5,encode($6,'hex')::uuid)")
        .bind(id(tag+6).as_slice()).bind(id(tag+1).as_slice()).bind(account.as_slice()).bind(id(42).as_slice()).bind(amount).bind(actor.as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_account_bank_balances(account_id,world_id,balance,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,$3,encode($4,'hex')::uuid)")
        .bind(account.as_slice()).bind(id(42).as_slice()).bind(amount).bind(id(tag+6).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}
pub(super) async fn financial_snapshot(h: &Harness) -> TestResult<String> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_array((SELECT jsonb_agg(to_jsonb(b) ORDER BY account_id) FROM game_account_bank_balances b),(SELECT jsonb_agg(to_jsonb(b) ORDER BY guild_id) FROM game_guild_bank_balances b),(SELECT jsonb_agg(to_jsonb(b) ORDER BY entry_id) FROM game_account_bank_entries b),(SELECT jsonb_agg(to_jsonb(b) ORDER BY entry_id) FROM game_guild_bank_entries b),(SELECT jsonb_agg(to_jsonb(b) ORDER BY guild_id) FROM game_guild_disband_claims b),(SELECT jsonb_agg(to_jsonb(b) ORDER BY operation_id,owner) FROM game_guild_bank_event_outbox b))::text").fetch_one(&h.pool).await?)
}

/// Retained custody from the existing Market owner is part of shared credit headroom.
pub(super) async fn retained_market_custody(h: &Harness) -> TestResult {
    sqlx::query("INSERT INTO game_market_books(world_id,definition_key) VALUES(encode($1,'hex')::uuid,'oteryn:item.tibia.i3031')")
        .bind(id(42).as_slice()).execute(&h.pool).await?;
    sqlx::query("INSERT INTO game_market_offers(offer_id,world_id,definition_key,definition_revision,side,account_id,character_id,piece_price,amount,remaining,created_at_us,expires_at_us,state,escrow_gold,held_credit_gold) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,'oteryn:item.tibia.i3031','definition-r1',1,encode($3,'hex')::uuid,encode($4,'hex')::uuid,7,1,1,1,2592000000001,1,7,0)")
        .bind(id(200).as_slice()).bind(id(42).as_slice()).bind(id(40).as_slice()).bind(id(41).as_slice()).execute(&h.pool).await?;
    Ok(())
}
