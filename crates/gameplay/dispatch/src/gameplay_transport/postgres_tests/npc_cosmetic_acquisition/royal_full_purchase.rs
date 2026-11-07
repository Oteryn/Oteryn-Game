use super::*;
pub(super) async fn exercise_full_purchase(
    h: &Harness,
    owner: &mut crate::gameplay_transport::ComposedFreshAdmission<'_, '_, '_>,
    admitted: &mut crate::gameplay_transport::connection::AdmittedSession,
    npc: oteryn_protocol_oteryn::world_spatial_entities::EntityRef,
) -> TestResult {
    // Historical standing wealth fixture, with a matching ledger head. This
    // does not claim these1b were earned by this acquisition test.
    let mut tx = h.pool.begin().await?;
    sqlx::query("SET LOCAL session_replication_role=replica")
        .execute(&mut *tx)
        .await?;
    // Existing GOLD-FEE-ACT-2 fixture boundary, as exercised by
    // type2_audit_activation_postgres_cases.rs::Harness::activate. The mature
    // fee owner permits a bank residual only under the actually active V2 tuple.
    sqlx::query("INSERT INTO game_type2_audit_activation VALUES (1, now())")
        .execute(&h.pool)
        .await?;
    // BANK0-RL-02 allows20m per deposit. Keep50 historical operation/entry
    // pairs and one exact ledger head, rather than inventing a500m deposit.
    let mut previous = None::<sqlx::types::Uuid>;
    for n in 0_i64..50 {
        let (occurrence,operation,event,platinum,gold,entry):(sqlx::types::Uuid,sqlx::types::Uuid,sqlx::types::Uuid,sqlx::types::Uuid,sqlx::types::Uuid,sqlx::types::Uuid)=sqlx::query_as("SELECT game_character_uuid_v7(),game_character_uuid_v7(),game_character_uuid_v7(),game_character_uuid_v7(),game_character_uuid_v7(),game_character_uuid_v7()").fetch_one(&mut *tx).await?;
        sqlx::query("INSERT INTO game_account_bank_operations(operation_occurrence_id,transaction_id,event_id,kind,request_binding,acting_character_id,account_id,world_id,channel_id,runtime_scope_ownership_generation,amount,outcome,occurred_at,envelope_sha256,committed_at,planned_platinum_item_instance_id,planned_gold_item_instance_id,backpack_item_instance_id) VALUES($1,$2,$3,1,sha256('historical royal wealth'),encode($4,'hex')::uuid,encode($5,'hex')::uuid,encode($6,'hex')::uuid,encode($7,'hex')::uuid,1,20000000,0,1,sha256('historical royal wealth'),0,$8,$9,encode($10,'hex')::uuid)")
            .bind(occurrence).bind(operation).bind(event).bind(id(41).as_slice()).bind(id(40).as_slice()).bind(id(42).as_slice()).bind(id(43).as_slice()).bind(platinum).bind(gold).bind(id(70).as_slice()).execute(&mut *tx).await?;
        sqlx::query("INSERT INTO game_account_bank_entries(entry_id,transaction_id,account_id,world_id,kind,amount,balance_before,balance_after,previous_entry_id,acting_character_id) VALUES($1,$2,encode($3,'hex')::uuid,encode($4,'hex')::uuid,1,20000000,$5,$6,$7,encode($8,'hex')::uuid)")
            .bind(entry).bind(operation).bind(id(40).as_slice()).bind(id(42).as_slice()).bind(n*20000000).bind((n+1)*20000000).bind(previous).bind(id(41).as_slice()).execute(&mut *tx).await?;
        previous = Some(entry);
    }
    sqlx::query("INSERT INTO game_account_bank_balances(account_id,world_id,balance,last_entry_id) VALUES(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1000000000,$3)")
        .bind(id(40).as_slice()).bind(id(42).as_slice()).bind(previous).execute(&mut *tx).await?;
    tx.commit().await?;
    super::super::grizzly_npc::wire::talk_paced(
        owner,
        admitted,
        npc,
        &[
            (1, "hi"),
            (2, "outfit"),
            (3, "yes"),
            (4, "armor"),
            (5, "yes"),
        ],
    )
    .await?;
    let kit:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_item_character_inbox_locations inbox JOIN game_item_instances i USING(item_instance_id) WHERE inbox.character_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i23398'").bind(id(41).as_slice()).fetch_one(&h.pool).await.map_err(|error|format!("actual royal StoreInbox kit missing: {error}"))?;
    assert_eq!(kit.len(), 16);
    let stage:Option<i64>=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/crystalserver/outfit_quest/golden_outfit'").bind(id(41).as_slice()).fetch_optional(&h.pool).await?;
    assert_eq!(stage, Some(1));
    let bank: i64 = sqlx::query_scalar(
        "SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid",
    )
    .bind(id(40).as_slice())
    .fetch_one(&h.pool)
    .await?;
    assert_eq!(
        bank, 500000000,
        "actual500m fee and grant must commit with one native kit"
    );
    for (first, name, expected_stage, expected_balance) in [
        (6, "helmet", 2_i64, 250000000_i64),
        (11, "boots", 3_i64, 0_i64),
    ] {
        super::super::grizzly_npc::wire::talk_paced(
            owner,
            admitted,
            npc,
            &[
                (first, "hi"),
                (first + 1, "outfit"),
                (first + 2, "yes"),
                (first + 3, name),
                (first + 4, "yes"),
            ],
        )
        .await?;
        let stage:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/crystalserver/outfit_quest/golden_outfit'").bind(id(41).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(stage, expected_stage);
        let balance:i64=sqlx::query_scalar("SELECT balance FROM game_account_bank_balances WHERE account_id=encode($1,'hex')::uuid").bind(id(40).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(balance, expected_balance);
        let current:Vec<Vec<u8>>=sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_item_character_inbox_locations inbox JOIN game_item_instances i USING(item_instance_id) WHERE inbox.character_id=encode($1,'hex')::uuid AND i.definition_production_key='oteryn:item.tibia.i23398'").bind(id(41).as_slice()).fetch_all(&h.pool).await?;
        assert_eq!(
            current,
            vec![kit.clone()],
            "addons must not mint another display kit"
        );
    }
    let target:String=sqlx::query_scalar("SELECT target_key FROM game_house_furniture_kit_origins WHERE item_instance_id=encode($1,'hex')::uuid").bind(&kit).fetch_one(&h.pool).await?;
    assert_eq!(target, "oteryn:item.tibia.i31510");
    assert_actual_royal_grants(h).await?;
    super::royal_downstream::exercise(
        h,
        owner,
        admitted,
        kit.try_into().map_err(|_| "Royal kit UUID length")?,
    )
    .await?;
    Ok(())
}

async fn assert_actual_royal_grants(h: &Harness) -> TestResult {
    use oteryn_gameplay_quests::quest::npc_cosmetics::source_program as source;
    let npc = source::embedded()
        .map_err(debug)?
        .npc("oteryn:npc.king_tibianus")
        .ok_or("embedded actual King programme")?;
    let mut keys = std::collections::BTreeSet::new();
    for step_id in ["golden-buy-armor", "golden-buy-helmet", "golden-buy-boots"] {
        let transition = npc
            .transitions
            .iter()
            .find(|step| step.id == step_id)
            .ok_or("actual Royal paid transition")?;
        let grants = transition
            .grants
            .iter()
            .filter(|grant| grant.outfit_key == "oteryn:content.outfit.golden_outfits")
            .map(|grant| grant.key.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        assert_eq!(
            grants.len(),
            1,
            "one paired-sex canonical Golden grant per paid piece"
        );
        for key in grants {
            keys.insert(key);
            let count:i64=sqlx::query_scalar("SELECT count(*) FROM game_account_appearance_unlocks u JOIN game_account_appearance_unlock_receipts r USING(grant_occurrence_id,account_id,unlock_key) JOIN game_npc_cosmetic_receipts p ON uuid_send(p.transaction_id)=r.source_event_id WHERE u.account_id=encode($1,'hex')::uuid AND u.unlock_key=$2 AND r.source_kind='npc' AND r.granting_character_id=encode($3,'hex')::uuid AND p.npc_key='oteryn:npc.king_tibianus' AND p.source_key=$4 AND p.program_sha256=$5 AND u.unlock_key=ANY(p.grant_keys)").bind(id(40).as_slice()).bind(key).bind(id(41).as_slice()).bind(&transition.source_key).bind(source::digest().as_slice()).fetch_one(&h.pool).await?;
            assert_eq!(
                count, 1,
                "actual immutable Account Golden grant and source parent for {step_id}"
            );
        }
    }
    assert_eq!(keys.len(), 3);
    Ok(())
}
