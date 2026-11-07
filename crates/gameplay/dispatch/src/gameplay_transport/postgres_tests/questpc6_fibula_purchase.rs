//! Source Dermot key acquisition: atomic payment, exact key, replay and repurchase.
use super::fixture::{Harness, debug};
use super::{AdmittedSession, ComposedFreshAdmission, TestResult};
use crate::gameplay_transport::connection::FreshAdmissionAuthority;
use oteryn_gameplay_quests::quest::world_callbacks::{Offering, Position};
use oteryn_protocol_oteryn::npc_service::{NpcIntentDisposition, NpcTalkIntent};

#[test]
fn questpc6_fibula_purchase_carried_price_exact_key_replay_and_repurchase() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-fibula-key-coins")
}

#[test]
fn questpc6_fibula_purchase_bank_price_exact_key_replay_and_repurchase() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-fibula-key-bank")
}

#[test]
fn questpc6_fibula_purchase_full_backpack_rolls_back_bank_fee_and_key() -> TestResult {
    super::qengine_rewards::scenario("qengine-rewards-fibula-key-full")
}

async fn counts(h: &Harness) -> TestResult<(i64, i64, i64)> {
    Ok(sqlx::query_as("SELECT (SELECT count(*) FROM game_quest_reward_sources),(SELECT count(*) FROM game_item_fee_burns WHERE cause_kind=18),(SELECT count(*) FROM game_item_keys)")
        .fetch_one(&h.pool).await?)
}

async fn bank(h: &Harness, amount: i64) -> TestResult {
    let uuid = crate::durability::item_fee_burn::test_support::uuid;
    // Fixture-only funding retains the ordinary bank ledger head. Gameplay below
    // uses the restricted runtime payment and MINT owners, with no seeded key/quest flag.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    let seed = format!(
        "INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) VALUES({},{},{},1,sha256('seed'),{},{},{},{},1,{amount},0,1,sha256('seed'),0,{},{},{}); INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,acting_character_id) VALUES({},{},{},{},1,{amount},0,{amount},{}); INSERT INTO game_account_bank_balances VALUES({},{},{amount},{});",
        uuid(80),
        uuid(81),
        uuid(82),
        uuid(41),
        uuid(40),
        uuid(42),
        uuid(43),
        uuid(83),
        uuid(84),
        uuid(70),
        uuid(85),
        uuid(81),
        uuid(40),
        uuid(42),
        uuid(41),
        uuid(40),
        uuid(42),
        uuid(85)
    );
    sqlx::raw_sql(sqlx::AssertSqlSafe(seed))
        .execute(&mut *tx)
        .await?;
    tx.commit().await?;
    Ok(())
}

pub(super) async fn exercise(
    owner: &mut ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut AdmittedSession,
    h: &Harness,
    name: &str,
) -> TestResult {
    let started = std::time::Instant::now();
    let callbacks = oteryn_gameplay_quests::quest::reward_callbacks::load()?;
    let first = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/fibula-key-first")
        .ok_or("source Dermot key purchase is not bound")?;
    let repeat = callbacks
        .iter()
        .find(|c| c.key == "oteryn:quest-reward/fibula-key-repeat")
        .ok_or("source Dermot repurchase is not bound")?;
    for callback in [first, repeat] {
        let value = serde_json::to_value(callback)?;
        assert_eq!(
            value["fee"],
            serde_json::json!({"gold":2000,"include_bank":true})
        );
        assert_eq!(
            value["give"],
            serde_json::json!({"item":"oteryn:item.tibia.i2968","count":1,"contents":[],"key_number":3940})
        );
    }
    let [npc] = super::classic_quests::install(owner, admitted, ["oteryn:npc.dermot"]).await?;
    let mut selected = admitted
        .continuity
        .selected_capabilities
        .as_slice()
        .to_vec();
    selected.extend([3, 6, 25]);
    selected.sort_unstable();
    selected.dedup();
    let offered = crate::gameplay_transport::appearance_change::offer::offered(
        owner.offered_capabilities(),
        owner.appearance_available(),
    );
    admitted.continuity.selected_capabilities =
        crate::gameplay_transport::capabilities::SelectedCapabilities::select(&offered, &selected)
            .ok_or("NPC capabilities")?;
    if !name.ends_with("coins") {
        // Bank participation is deliberately unavailable under the historical
        // V1 tuple. These cases exercise the deployed native V2 bank owner.
        sqlx::query(
            "INSERT INTO game_type2_audit_activation VALUES(1,now()) ON CONFLICT DO NOTHING",
        )
        .execute(&h.pool)
        .await?;
    }
    let before = counts(h).await?;
    assert_eq!(before.2, 0, "the fixture never pre-grants a key");
    let (result, _, _) = super::qengine_rewards::wire_grant(owner, admitted, npc, first).await?;
    assert_eq!(result, NpcIntentDisposition::Rejected, "insufficient funds");
    assert_eq!(
        counts(h).await?,
        before,
        "failed purchase writes no value or source"
    );
    let offering = Offering {
        position: Position {
            x: 33293,
            y: 31677,
            floor: -15,
        },
        item: "oteryn:item.tibia.i3035".into(),
        count: 80,
    };
    let fence = owner
        .current_quest_fence(admitted.game_session_id)
        .await
        .map_err(|_| "fence")?
        .ok_or("fence")?;
    if name.ends_with("coins") {
        super::quest_world_callbacks::input_item(owner, fence.item_fence(), &offering, 700, true)
            .await?;
    } else {
        bank(h, 8000).await?;
    }
    if name.ends_with("full") {
        let native = owner
            .active_generation
            .and_then(|g| g.native_gameplay())
            .ok_or("native")?;
        let facts = crate::gameplay_transport::item_ref_admission::generation_item_facts(
            native,
            &crate::durability::item_mint::TypedDefinitionRef {
                family: "Item".into(),
                production_key: "oteryn:item.tibia.i2854".into(),
                revision_ref: "definition-r1".into(),
            },
        )
        .map_err(debug)?;
        for n in 0..facts.container_capacity.ok_or("backpack capacity")? {
            super::quest_world_callbacks::input_item(
                owner,
                fence.item_fence(),
                &Offering {
                    item: "oteryn:item.tibia.i2854".into(),
                    count: 1,
                    ..offering.clone()
                },
                710 + u64::from(n),
                true,
            )
            .await?;
        }
        let (result, _, _) =
            super::qengine_rewards::wire_grant(owner, admitted, npc, first).await?;
        assert_eq!(result, NpcIntentDisposition::Rejected, "full backpack");
        assert_eq!(
            counts(h).await?,
            before,
            "capacity refusal must roll back the fee"
        );
        let balance: i64 = sqlx::query_scalar("SELECT balance FROM game_account_bank_balances")
            .fetch_one(&h.pool)
            .await?;
        assert_eq!(balance, 8000);
    } else {
        for (index, callback) in [first, repeat, repeat].into_iter().enumerate() {
            let (result, _, command) =
                super::qengine_rewards::wire_grant(owner, admitted, npc, callback).await?;
            assert_eq!(result, NpcIntentDisposition::Ok);
            let expected = i64::try_from(index + 1)?;
            assert_eq!(
                counts(h).await?,
                (before.0 + expected, before.1 + expected, expected)
            );
            let keys:Vec<(String,i64,i32)> = sqlx::query_as("SELECT i.definition_production_key,i.quantity,k.key_number FROM game_item_keys k JOIN game_item_instances i USING(item_instance_id) ORDER BY i.item_instance_id")
                .fetch_all(&h.pool).await?;
            assert_eq!(
                keys,
                vec![("oteryn:item.tibia.i2968".into(), 1, 3940); index + 1]
            );
            let payload =
                oteryn_protocol_oteryn::npc_service::encode_npc_talk_intent(&NpcTalkIntent {
                    npc_actor: npc,
                    text: callback.keywords[0].clone(),
                })
                .map_err(debug)?;
            let reply =
                super::quest_world_callbacks::wire(owner, admitted, command, 7, payload).await?;
            assert_eq!(
                oteryn_protocol_oteryn::npc_service::decode_npc_intent_result(&reply)
                    .map_err(debug)?,
                NpcIntentDisposition::Ok
            );
            assert_eq!(
                counts(h).await?,
                (before.0 + expected, before.1 + expected, expected),
                "literal replay"
            );
        }
        let paid:(i64,i64,i64) = sqlx::query_as("SELECT count(*),sum(fee_gold_units)::bigint,sum(bank_debit_gold_units)::bigint FROM game_item_fee_burns WHERE cause_kind=18")
            .fetch_one(&h.pool).await?;
        assert_eq!(
            paid,
            (3, 6000, if name.ends_with("bank") { 6000 } else { 0 })
        );
    }
    println!(
        "POSTGRES_CASE_COMPLETED {name} duration={:?}",
        started.elapsed()
    );
    Ok(())
}
