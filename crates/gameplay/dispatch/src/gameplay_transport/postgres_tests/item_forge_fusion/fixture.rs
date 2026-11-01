//! Native fixtures and assertions. Only setup rows use the existing fixture bypass;
//! every tested action goes through the fully guarded production command path.
use super::*;
use oteryn_simulation_determinism::{
    DecisionOccurrenceId, GameplayDecisionRoot, deterministic_decision_u64,
};
use sha2::{Digest, Sha256};

pub(super) async fn seed_bank(h: &Harness) -> TestResult {
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    sqlx::query("INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,1,sha256('seed'),encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,1,1,0,1,sha256('seed'),0,encode($8,'hex')::uuid,encode($9,'hex')::uuid,encode($10,'hex')::uuid)")
        .bind(id(240).as_slice()).bind(id(241).as_slice()).bind(id(242).as_slice()).bind(id(41).as_slice()).bind(id(40).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(id(243).as_slice()).bind(id(244).as_slice()).bind(id(70).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,acting_character_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,1000000000,0,1000000000,encode($5,'hex')::uuid)")
        .bind(id(245).as_slice()).bind(id(241).as_slice()).bind(id(40).as_slice()).bind(id(42).as_slice()).bind(id(41).as_slice()).execute(&mut *tx).await?;
    sqlx::query("INSERT INTO game_account_bank_balances(account_id,world_id,balance,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1000000000,encode($3,'hex')::uuid)")
        .bind(id(40).as_slice()).bind(id(42).as_slice()).bind(id(245).as_slice()).execute(&mut *tx).await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn bank(h: &Harness) -> TestResult<i64> {
    Ok(sqlx::query_scalar::<_,i64>("SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid AND world_id=encode($2,'hex')::uuid")
        .bind(id(40).as_slice()).bind(id(42).as_slice()).fetch_optional(&h.pool).await?.unwrap_or(0))
}

/// Input custody setup: both transfer items are children of a carried backpack.
pub(super) async fn nest_pair(h: &Harness) -> TestResult {
    seed_items(h, &[(104, "oteryn:item.tibia.i2854", 1, 0)]).await?;
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    for (ordinal, item) in [(1_i64, 101), (2_i64, 102)] {
        sqlx::query(
            "DELETE FROM game_item_container_entries WHERE item_instance_id=encode($1,'hex')::uuid",
        )
        .bind(id(item).as_slice())
        .execute(&mut *tx)
        .await?;
        sqlx::query("INSERT INTO game_item_nested_entries(item_instance_id,world_id,parent_item_instance_id,placement_ordinal,placed_transaction_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,encode($3,'hex')::uuid,$4,encode($5,'hex')::uuid)")
            .bind(id(item).as_slice()).bind(id(42).as_slice()).bind(id(104).as_slice()).bind(ordinal).bind(id(72).as_slice()).execute(&mut *tx).await?;
    }
    tx.commit().await?;
    Ok(())
}

/// Select an input occurrence using this isolated test database's generated root.
/// No production RNG hook, changed root, overwritten reservation or logged secret.
pub(super) async fn occurrence_for(
    h: &Harness,
    tag: u8,
    accepts: fn([u16; 3]) -> bool,
) -> TestResult<[u8; 16]> {
    let seed: Vec<u8> =
        sqlx::query_scalar("SELECT secret FROM game_forge_decision_root WHERE singleton")
            .fetch_one(&h.pool)
            .await?;
    let mut context = Sha256::new();
    context.update(seed);
    context.update(id(41));
    let root = GameplayDecisionRoot::from_bytes(context.finalize().into());
    for n in 0_u32..100_000 {
        let mut occurrence = id(tag);
        occurrence[9..13].copy_from_slice(&n.to_be_bytes());
        let candidate = DecisionOccurrenceId::from_bytes(occurrence);
        let draw = |index, bound| {
            deterministic_decision_u64(&root, candidate, "forge_fusion", index)
                .map(|value| (value % bound) as u16)
                .map_err(debug)
        };
        if accepts([draw(0, 100)?, draw(1, 100)?, draw(2, 10_001)?]) {
            return Ok(occurrence);
        }
    }
    Err("bounded Forge fixture occurrence search exhausted".into())
}

pub(super) async fn item_state(h: &Harness, item: [u8; 16], tier: Option<u8>) -> TestResult {
    let actual: (i64,i16,i16,bool) = sqlx::query_as("SELECT i.quantity,i.lifecycle,coalesce(t.tier,0)::smallint,EXISTS(SELECT 1 FROM game_item_container_entries e WHERE e.item_instance_id=i.item_instance_id) OR EXISTS(SELECT 1 FROM game_item_nested_entries e WHERE e.item_instance_id=i.item_instance_id) FROM game_item_instances i LEFT JOIN game_item_tiers t USING(item_instance_id) WHERE i.item_instance_id=encode($1,'hex')::uuid")
        .bind(item.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        actual,
        match tier {
            Some(tier) => (1, 1, i16::from(tier), true),
            None => (0, 2, 0, false),
        }
    );
    Ok(())
}

/// Native assets and their append-only evidence, excluding cooldown/command bookkeeping.
pub(super) async fn assets(h: &Harness) -> TestResult<Value> {
    Ok(sqlx::query_scalar("SELECT jsonb_build_object('items',(SELECT jsonb_agg(to_jsonb(i) ORDER BY item_instance_id) FROM game_item_instances i),'tiers',(SELECT jsonb_agg(to_jsonb(t) ORDER BY item_instance_id) FROM game_item_tiers t),'custody',(SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_container_entries e),'nested',(SELECT jsonb_agg(to_jsonb(e) ORDER BY item_instance_id) FROM game_item_nested_entries e),'dust',(SELECT jsonb_agg(to_jsonb(d) ORDER BY character_id) FROM game_character_forge_dust d),'bank',(SELECT jsonb_agg(to_jsonb(b) ORDER BY account_id,world_id) FROM game_account_bank_balances b),'dust_entries',(SELECT count(*) FROM game_character_forge_dust_entries),'bank_entries',(SELECT count(*) FROM game_account_bank_entries),'tier_audit',(SELECT count(*) FROM game_item_tier_audit),'audit',(SELECT count(*) FROM game_item_audit_outbox),'fees',(SELECT count(*) FROM game_item_fee_burns))")
        .fetch_one(&h.pool).await?)
}

pub(super) async fn receipt(
    h: &Harness,
    action: Request,
    reply: &Value,
    cost: Option<(i64, i64, i64)>,
) -> TestResult {
    let (saved,applied,dust,gold):(Value,bool,i64,i64)=sqlx::query_as("SELECT reply,applied,dust_spent,gold_spent FROM game_forge_operations WHERE character_id=encode($1,'hex')::uuid AND occurrence_id=encode($2,'hex')::uuid")
        .bind(id(41).as_slice()).bind(action.occurrence.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(&saved, reply);
    assert_eq!(applied, cost.is_some());
    assert_eq!(
        (dust, gold),
        cost.map_or((0, 0), |(dust, _, gold)| (dust, gold))
    );
    let (history,audit,fee):(i64,i64,i64)=sqlx::query_as("SELECT (SELECT count(*) FROM game_forge_history h WHERE h.character_id=p.character_id AND h.occurrence_id=p.occurrence_id AND h.reply=p.reply),(SELECT count(*) FROM game_item_audit_outbox a WHERE a.transaction_id=p.transaction_id AND a.event_id=p.event_id AND a.envelope_sha256=p.envelope_sha256),(SELECT coalesce(sum(fee_gold_units),0)::bigint FROM game_item_fee_burns f WHERE f.transaction_id=p.transaction_id) FROM game_forge_operations p WHERE p.character_id=encode($1,'hex')::uuid AND p.occurrence_id=encode($2,'hex')::uuid")
        .bind(id(41).as_slice()).bind(action.occurrence.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        (history, audit, fee),
        (i64::from(cost.is_some()), i64::from(cost.is_some()), gold)
    );
    Ok(())
}
