//! M22: real carried craft admission, custody, receipt, audit and exact replay.
#![cfg(feature = "test-support")]
use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
use oteryn_gameplay_domain::domain;
use oteryn_gameplay_durability::durability;
use oteryn_gameplay_entities as foundation;
#[allow(dead_code)]
#[path = "../../../../apps/game-server/tests/support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
#[allow(dead_code)]
#[path = "../../../../apps/game-server/tests/support/authority_cases/engitem_reward_text.rs"]
mod reward_fixture;
use bestiary_postgres_harness as f;
use durability::item_loop_move::ItemLoopContent;
use durability::item_mint::TypedDefinitionRef;
#[path = "support/native_inventory.rs"]
mod native_inventory;
use native_inventory::mint_to_bag;
use oteryn_gameplay_items::item_ref_admission::generation_item_facts;
use oteryn_protocol_oteryn::world_object::UseDisposition;

impl ItemLoopContent for reward_fixture::Weights<'_> {
    fn loop_content_digest(&self) -> [u8; 32] {
        self.0.source_digest()
    }
    fn loop_facts(
        &self,
        definition: &TypedDefinitionRef,
    ) -> Option<durability::item_transfer::ItemDefinitionFacts> {
        generation_item_facts(self.0, definition).ok()
    }
}

#[test]
fn m22_tainted_souls_consumes_captured_antlers_and_replays_postgres() -> f::TestResult {
    let Some(admin) = f::configured_admin() else {
        return Ok(());
    };
    std::thread::Builder::new()
        .stack_size(64 << 20)
        .spawn(move || -> Result<(), String> {
            f::runtime()
                .map_err(f::debug)?
                .block_on(async {
                    let started = std::time::Instant::now();
                    let h = f::Harness::create(admin, "questpc3_m22_craft", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M22_CRAFT POSTGRES_CASE_COMPLETED duration_ms={} success=true",
                        started.elapsed().as_millis()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M22 craft PG thread failed")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        version,
        oteryn_gameplay_durability::CANONICAL_POSTGRES_SERVER_VERSION_NUM
    );
    eprintln!("M22_CRAFT POSTGRES_CONNECTED version={version}");
    let (controller, _) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            f::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native generation")?;
    eprintln!("M22_CRAFT NATIVE_READY");
    let weights = reward_fixture::Weights(native);
    sqlx::query("INSERT INTO game_control_scope_grants VALUES(session_user,encode($1,'hex')::uuid,encode($2,'hex')::uuid,4)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).execute(&h.pool).await?;
    sqlx::query("SELECT game_content_record_activation(encode($1,'hex')::uuid,encode($2,'hex')::uuid,1,NULL,$3,$4,$5)")
        .bind(f::id(f::WORLD).as_slice()).bind(f::id(f::CHANNEL).as_slice()).bind(native.source_digest().as_slice()).bind([2_u8;32].as_slice()).bind([3_u8;32].as_slice()).execute(&h.pool).await?;
    let seal = h.recovery.seal_current().map_err(f::debug)?;
    let authority = h
        .root
        .open_character_authority(&seal)
        .await
        .map_err(f::debug)?;
    reward_fixture::equip_backpack(h, &authority, native, &weights).await?;
    eprintln!("M22_CRAFT BACKPACK_READY");
    let herb = mint_to_bag(h, &authority, native, &weights, 3736, 2, 1).await?;
    let antlers = mint_to_bag(h, &authority, native, &weights, 10297, 7, 2).await?;
    // A whole-target debit frees exactly one entry even when the used Herb stack survives.
    // Fill the existing bag so failing to account for that entry produces NoRoom.
    let capacity = weights
        .loop_facts(&reward_fixture::definition(2854))
        .and_then(|facts| facts.container_capacity)
        .ok_or("native bag capacity")?
        .min(20);
    assert!(capacity >= 2);
    for ordinal in 3..=capacity {
        mint_to_bag(h, &authority, native, &weights, 22008, 1, ordinal).await?;
    }
    let entries_before: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(entries_before, i64::from(capacity));
    let request = durability::item_craft::ItemCraftRequest {
        command: reward_fixture::command(9500)?,
        source: herb,
        tool: None,
        ingredient: Some(antlers),
        station: Some(reward_fixture::definition(10297)),
        intent: vec![22, 7],
        gathering: None,
        enchanting: None,
    };
    eprintln!("M22_CRAFT INPUTS_READY full_bag_entries={entries_before}");
    let unowned = h
        .root
        .craft_item_owned::<_, ()>(
            &authority,
            &h.node,
            f::item_fence()?,
            request.clone(),
            native,
            &weights,
            false,
            None,
        )
        .await
        .map_err(f::debug)?;
    assert_eq!(
        unowned,
        Some(UseDisposition::Rejected),
        "a fresh craft needs the actual lane or live window"
    );
    let unowned_receipts: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_craft_records WHERE game_session_id=encode($1,'hex')::uuid AND command_id=9500")
        .bind(f::item_fence()?.game_session_id.as_bytes().as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        unowned_receipts, 0,
        "missing lane proof must not mint a craft receipt"
    );
    let permit = durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(f::scope()?)
        .await
        .map_err(f::debug)?;
    for _ in 0..2 {
        let result = h
            .root
            .craft_item(
                &permit,
                &authority,
                &h.node,
                f::item_fence()?,
                request.clone(),
                native,
                &weights,
                false,
            )
            .await
            .map_err(f::debug)?;
        assert_eq!(result, Some(UseDisposition::Committed));
    }
    let source: (i64, i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(herb.as_slice()).fetch_one(&h.pool).await?;
    let target: (i64, i16) = sqlx::query_as("SELECT quantity,lifecycle FROM game_item_instances WHERE item_instance_id=encode($1,'hex')::uuid").bind(antlers.as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(source, (1, 1));
    assert_eq!(target, (0, 2));
    let receipt: (i64, i32) = sqlx::query_as("SELECT count(*)::bigint,max(ingredient_count)::integer FROM game_item_craft_records WHERE recipe='oteryn:item-conversion.tainted-souls-antler-talisman'").fetch_one(&h.pool).await?;
    assert_eq!(receipt, (1, 7));
    let output: (i64, i64) = sqlx::query_as("SELECT count(*)::bigint,coalesce(sum(i.quantity),0)::bigint FROM game_item_instances i JOIN game_item_container_entries e USING(item_instance_id) JOIN game_item_craft_records r ON r.transaction_id=i.minted_transaction_id WHERE r.recipe='oteryn:item-conversion.tainted-souls-antler-talisman' AND i.definition_production_key='oteryn:item.tibia.i22008' AND i.lifecycle=1").fetch_one(&h.pool).await?;
    assert_eq!(output, (1, 1));
    let entries_after: i64 = sqlx::query_scalar("SELECT count(*) FROM game_item_container_entries WHERE character_id=encode($1,'hex')::uuid")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(entries_after, entries_before);
    Ok(())
}
