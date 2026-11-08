//! M01c: native terrain USE receipts freeze their typed source definition.
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
fn m01c_terrain_use_receipt_freezes_tool_definition_postgres() -> f::TestResult {
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
                    let h = f::Harness::create(admin, "questpc3_m01c_terrain", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M01C_TERRAIN POSTGRES_CASE_COMPLETED duration_ms={} success=true",
                        started.elapsed().as_millis()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "M01c terrain PG thread failed")?
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
    eprintln!("M01C_TERRAIN POSTGRES_CONNECTED version={version}");
    let (controller, _) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            f::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|generation| generation.native_gameplay())
        .ok_or("native generation")?;
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
    let tool = mint_to_bag(h, &authority, native, &weights, 3457, 1, 1).await?;
    let scope = durability::terrain_tool::TerrainScope {
        map_revision: "map-1".into(),
        content_generation: native.source_digest(),
        reset_epoch: 0,
    };
    let mutation = durability::terrain_tool::TerrainMutation {
        tool,
        tool_key: "oteryn:item.tibia.i3457".into(),
        tool_revision: "definition-r1".into(),
        placement: vec![0x31; 40],
        expected_revision: 0,
        before: 468,
        after: 469,
        decay: vec![],
    };
    let command = reward_fixture::command(9500)?;
    let permit = durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(
        f::item_fence()?.runtime_scope,
    )
    .await
    .map_err(f::debug)?;
    for _ in 0..2 {
        let outcome = h
            .root
            .apply_terrain_tool(
                &permit,
                &authority,
                &h.node,
                f::item_fence()?,
                command,
                vec![1, 31],
                scope.clone(),
                Some(mutation.clone()),
            )
            .await
            .map_err(f::debug)?;
        assert_eq!(outcome, Some(UseDisposition::Committed));
    }
    let captured: (String,String,String,i64) = sqlx::query_as("SELECT tool_definition_family,tool_definition_production_key,tool_definition_revision_ref,transformed_item_id FROM game_terrain_tool_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=9500")
        .bind(f::id(f::SESSION).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(
        captured,
        (
            "Item".into(),
            "oteryn:item.tibia.i3457".into(),
            "definition-r1".into(),
            469
        )
    );
    let count: i64 = sqlx::query_scalar("SELECT count(*) FROM game_terrain_tool_receipts WHERE game_session_id=encode($1,'hex')::uuid AND command_id=9500")
        .bind(f::id(f::SESSION).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(count, 1);
    let refused = sqlx::query("UPDATE game_terrain_tool_receipts SET tool_definition_production_key='oteryn:item.tibia.i3456' WHERE game_session_id=encode($1,'hex')::uuid AND command_id=9500")
        .bind(f::id(f::SESSION).as_slice()).execute(&h.pool).await;
    let Err(sqlx::Error::Database(error)) = refused else {
        return Err("rewriting the historical tool definition must fail".into());
    };
    assert_eq!(
        error.message(),
        "terrain tool definition snapshot is immutable"
    );
    Ok(())
}
