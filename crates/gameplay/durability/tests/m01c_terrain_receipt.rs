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
use durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use native_inventory::mint_to_bag;
use oteryn_gameplay_items::item_ref_admission::generation_item_facts;
use oteryn_gameplay_quests::quest::{QuestRefusal, QuestStateCatalogue};
use oteryn_protocol_oteryn::world_object::UseDisposition;
use std::sync::Arc;

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
    assert!(
        h.root
            .read_quest_terrain_use(&authority, &h.node, f::item_fence()?, command)
            .await
            .map_err(f::debug)?
            .is_none()
    );
    let catalogue = terrain_catalogue()?;
    let request = |suffix: &str| QuestTransitionRequest {
        transition_key: format!("oteryn:quest-terrain-use/m01c-{suffix}"),
        cause: QuestCause::Command(command),
    };
    let missing = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            f::fence(1)?,
            request("exact"),
            Arc::clone(&catalogue),
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(
        missing,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    ));
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
    let observed = h
        .root
        .read_quest_terrain_use(&authority, &h.node, f::item_fence()?, command)
        .await
        .map_err(f::debug)?
        .ok_or("committed native observation")?;
    assert_eq!(observed.tool.key, "oteryn:item.tibia.i3457");
    assert_eq!(observed.tool.revision, "definition-r1");
    let candidates = catalogue
        .authored_bindings()
        .ok_or("bindings")?
        .terrain_use_candidates(
            &catalogue,
            &std::collections::BTreeMap::new(),
            &observed.tool,
            &observed.placement,
            observed.result,
        );
    assert_eq!(candidates, vec!["oteryn:quest-terrain-use/m01c-exact"]);
    let wrong_pair = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            f::fence(1)?,
            request("cross-pair"),
            Arc::clone(&catalogue),
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(
        wrong_pair,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    ));
    let committed = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            f::fence(1)?,
            request("exact"),
            Arc::clone(&catalogue),
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(committed, QuestTransitionOutcome::Committed(_)));
    let replayed = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            f::fence(1)?,
            request("exact"),
            Arc::clone(&catalogue),
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(
        replayed,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    let second_stage = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            f::fence(2)?,
            request("second-stage"),
            Arc::clone(&catalogue),
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(
        second_stage,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    ));
    let advances: i64 = sqlx::query_scalar("SELECT count(*) FROM game_character_quest_receipts WHERE quest_key='oteryn:quest.m01c_terrain_fixture'").fetch_one(&h.pool).await?;
    assert_eq!(advances, 1);
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
    production_dead_archer_entry(h, &authority, native, tool).await?;
    Ok(())
}

fn terrain_catalogue() -> f::TestResult<Arc<QuestStateCatalogue>> {
    use oteryn_gameplay_quests::quest::authored_bindings::Bindings;
    let mut stages = Vec::new();
    for (suffix, before, after, tools) in [
        (
            "exact",
            0,
            1,
            serde_json::json!([{"key":"oteryn:item.tibia.i3457","revision":"definition-r1"}]),
        ),
        (
            "cross-pair",
            0,
            1,
            serde_json::json!([{"key":"oteryn:item.tibia.i3457","revision":"definition-r2"},{"key":"oteryn:item.tibia.i3458","revision":"definition-r1"}]),
        ),
        (
            "second-stage",
            1,
            2,
            serde_json::json!([{"key":"oteryn:item.tibia.i3457","revision":"definition-r1"}]),
        ),
    ] {
        stages.push(serde_json::json!({"key":suffix,"completes":false,"trigger":{"kind":"terrain_use","usage":{
            "key":format!("oteryn:quest-terrain-use/m01c-{suffix}"),"tools":tools,"placement":vec![0x31_u8;40],"result":469,
            "storage":[{"track":"oteryn:quest-progress/m01c-terrain","minimum":before,"maximum":before,"set":after,"preserve":false}],
            "sources":["M01c fenced engine fixture; no production quest qualification"]
        }}}));
    }
    let value = serde_json::json!({"schema":1,"quests":[{
        "quest":"oteryn:quest.m01c_terrain_fixture","recipe_sha256":"a".repeat(64),
        "tracks":[{"key":"oteryn:quest-progress/m01c-terrain","initial":0,"minimum":0,"maximum":2}],
        "stages":stages,"sources":["M01c engine fixture"]
    }]});
    let bindings = Bindings::parse(&serde_json::to_vec(&value)?).map_err(f::debug)?;
    let base = QuestStateCatalogue::empty("content-1").ok_or("catalogue revision")?;
    Ok(Arc::new(bindings.attach(&base).map_err(f::debug)?))
}

async fn production_dead_archer_entry(
    h: &f::Harness,
    authority: &durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
    tool: [u8; 16],
) -> f::TestResult {
    let loaded = oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
        .map_err(f::debug)?;
    let catalogue = Arc::new(loaded.catalogue().clone());
    let transition = "oteryn:quest-terrain-use/m01c-dead-archer-607";
    assert!(
        catalogue.transition(transition).is_some(),
        "production boot binding is absent"
    );
    let scope = durability::terrain_tool::TerrainScope {
        map_revision: "map-1".into(),
        content_generation: native.source_digest(),
        reset_epoch: 0,
    };
    let permit = durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(
        f::item_fence()?.runtime_scope,
    )
    .await
    .map_err(f::debug)?;
    for (x, ordinal, expected) in [(32494_u64, 9599_u64, false), (32493, 9600, true)] {
        let key = (x << 32) | (32259_u64 << 16) | (7_u64 << 8);
        let mut placement = vec![0x42; 32];
        placement.extend_from_slice(&key.to_be_bytes());
        let command = reward_fixture::command(ordinal)?;
        let mutation = durability::terrain_tool::TerrainMutation {
            tool,
            tool_key: "oteryn:item.tibia.i3457".into(),
            tool_revision: "definition-r1".into(),
            placement,
            expected_revision: 0,
            before: 606,
            after: 607,
            decay: vec![],
        };
        let result = h
            .root
            .apply_terrain_tool(
                &permit,
                authority,
                &h.node,
                f::item_fence()?,
                command,
                ordinal.to_be_bytes().to_vec(),
                scope.clone(),
                Some(mutation),
            )
            .await
            .map_err(f::debug)?;
        assert_eq!(result, Some(UseDisposition::Committed));
        let observed = h
            .root
            .read_quest_terrain_use(authority, &h.node, f::item_fence()?, command)
            .await
            .map_err(f::debug)?
            .ok_or("production native receipt")?;
        let keys = catalogue
            .authored_bindings()
            .ok_or("production bindings")?
            .terrain_use_candidates(
                &catalogue,
                &std::collections::BTreeMap::new(),
                &observed.tool,
                &observed.placement,
                observed.result,
            );
        assert_eq!(keys.contains(&transition.to_owned()), expected);
        let request = QuestTransitionRequest {
            transition_key: transition.into(),
            cause: QuestCause::Command(command),
        };
        let result = h
            .root
            .commit_character_quest_transition(
                authority,
                &h.node,
                f::fence(2)?,
                request.clone(),
                Arc::clone(&catalogue),
            )
            .await
            .map_err(f::debug)?;
        if expected {
            assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
            let replay = h
                .root
                .commit_character_quest_transition(
                    authority,
                    &h.node,
                    f::fence(2)?,
                    request,
                    Arc::clone(&catalogue),
                )
                .await
                .map_err(f::debug)?;
            assert!(matches!(
                replay,
                QuestTransitionOutcome::AlreadyCommitted(_)
            ));
        } else {
            assert!(matches!(
                result,
                QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
            ));
        }
    }
    let value:i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/m01c-dead-archer-route'").bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(value, 1);
    let not_completed:bool=sqlx::query_scalar("SELECT completed_character_revision IS NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.dead_archer_quest'").bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert!(
        not_completed,
        "entry dig must not complete or grant the quest"
    );
    eprintln!(
        "M01C_DEAD_ARCHER production_entry wrong_tile_refused exact_tile_committed replay_safe completion_held"
    );
    Ok(())
}
