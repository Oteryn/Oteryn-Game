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
    terrain_counter_fixture(h, &authority, native, tool).await?;
    production_fanfare_collection(h, &authority, native).await?;
    production_black_knight_collection(h, &authority, native).await?;
    production_grimvale_collection(h, &authority, native, &weights).await?;
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

async fn terrain_counter_fixture(
    h: &f::Harness,
    authority: &durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
    tool: [u8; 16],
) -> f::TestResult {
    use oteryn_gameplay_quests::quest::authored_bindings::Bindings;
    let placement = vec![0x55_u8; 40];
    let document = serde_json::json!({"schema":1,"quests":[{
        "quest":"oteryn:quest.m01c_counter_fixture","recipe_sha256":"a".repeat(64),
        "tracks":[{"key":"oteryn:quest-progress/m01c-use-counter","initial":0,"minimum":0,"maximum":2}],
        "stages":[{"key":"s1","completes":false,"trigger":{"kind":"terrain_use","usage":{
            "key":"oteryn:quest-terrain-use/m01c-counter","tools":[{"key":"oteryn:item.tibia.i3457","revision":"definition-r1"}],
            "placement":placement,"result":607,"storage":[{"track":"oteryn:quest-progress/m01c-use-counter","minimum":0,"maximum":1,"set":1,"increment":true}],
            "sources":["M01c engine counter fixture"]
        }}}],"sources":["M01c engine fixture; no production quest qualification"]
    }]});
    let bindings = Bindings::parse(&serde_json::to_vec(&document)?).map_err(f::debug)?;
    let catalogue = Arc::new(
        bindings
            .attach(&QuestStateCatalogue::empty("content-1").ok_or("revision")?)
            .map_err(f::debug)?,
    );
    let item_fence = f::item_fence()?;
    let permit =
        durability::spell_owner_commit::SpellLanePermit::of_fresh_scope(item_fence.runtime_scope)
            .await
            .map_err(f::debug)?;
    let scope = durability::terrain_tool::TerrainScope {
        map_revision: "map-1".into(),
        content_generation: native.source_digest(),
        reset_epoch: 0,
    };
    let mut expected_revision = 0;
    for n in 0..2_u64 {
        if n > 0 {
            let states = h
                .root
                .terrain_tool_snapshot(
                    &permit,
                    authority,
                    &h.node,
                    durability::item_decay_retire::CorpseDecayFence {
                        runtime_scope: item_fence.runtime_scope,
                        scope_ownership_generation: item_fence.scope_ownership_generation,
                    },
                    scope.clone(),
                )
                .await
                .map_err(f::debug)?;
            let restored = states
                .iter()
                .find(|state| state.placement == placement)
                .ok_or("decayed counter target")?;
            assert_eq!(
                restored.item, 606,
                "the existing clock must restore the dig target"
            );
            expected_revision = restored.revision;
        }
        let command = reward_fixture::command(9700 + n)?;
        let mutation = durability::terrain_tool::TerrainMutation {
            tool,
            tool_key: "oteryn:item.tibia.i3457".into(),
            tool_revision: "definition-r1".into(),
            placement: placement.clone(),
            expected_revision,
            before: 606,
            after: 607,
            decay: vec![(606, 1)],
        };
        let result = h
            .root
            .apply_terrain_tool(
                &permit,
                authority,
                &h.node,
                item_fence,
                command,
                command.command_id().get().to_be_bytes().to_vec(),
                scope.clone(),
                Some(mutation),
            )
            .await
            .map_err(f::debug)?;
        assert_eq!(result, Some(UseDisposition::Committed));
        let request = QuestTransitionRequest {
            transition_key: "oteryn:quest-terrain-use/m01c-counter".into(),
            cause: QuestCause::Command(command),
        };
        let result = h
            .root
            .commit_character_quest_transition(
                authority,
                &h.node,
                f::fence(3 + n)?,
                request.clone(),
                Arc::clone(&catalogue),
            )
            .await
            .map_err(f::debug)?;
        assert!(matches!(result, QuestTransitionOutcome::Committed(_)));
        let replay = h
            .root
            .commit_character_quest_transition(
                authority,
                &h.node,
                f::fence(3 + n)?,
                request,
                Arc::clone(&catalogue),
            )
            .await
            .map_err(f::debug)?;
        assert!(matches!(
            replay,
            QuestTransitionOutcome::AlreadyCommitted(_)
        ));
        let value:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/m01c-use-counter'").bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(value, i64::try_from(n + 1)?);
    }
    eprintln!("M01C_USE_COUNTER two_native_actions_two_increments replay_did_not_increment");
    Ok(())
}

async fn production_fanfare_collection(
    h: &f::Harness,
    authority: &durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
) -> f::TestResult {
    use durability::reward_claim_mint::{RewardClaimMintOutcome, RewardClaimMintRequest};
    let catalogue = Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(f::debug)?
            .catalogue()
            .clone(),
    );
    let transition_key = "oteryn:quest-collection/m01c-fanfare-key";
    assert!(catalogue.transition(transition_key).is_some());
    for (ordinal, number) in [(80_u32, 3519_u16), (81, 3520)] {
        let request = RewardClaimMintRequest {
            backpack_outputs: false,
            root_charges_remaining: None,
            root_fluid_subtype: None,
            command: reward_fixture::command(9800 + u64::from(ordinal))?,
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: format!("fixture:m01c-key-{number}"),
                revision_ref: "definition-r1".into(),
            },
            source_placement: format!("fixture:m01c-key-box-{number}"),
            item: generation_item_facts(native, &reward_fixture::definition(2973))
                .map_err(f::debug)?,
            quantity: 1,
            contents: Vec::new(),
            additional_items: Vec::new(),
            root_text: None,
            backpack: generation_item_facts(native, &reward_fixture::definition(2854))
                .map_err(f::debug)?,
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
            achievement: None,
            quest_transition: None,
            key_number: Some(number),
        };
        let mut candidate = h
            .root
            .freeze_reward_claim_mint(authority, &h.node, f::item_fence()?, request)
            .await
            .map_err(f::debug)?;
        let minted = h
            .root
            .commit_reward_claim_mint(authority, &h.node, f::item_fence()?, &mut candidate)
            .await
            .map_err(f::debug)?;
        assert!(matches!(minted, RewardClaimMintOutcome::Committed(_)));
        // Claim MINT is itself the native acquisition; no unrelated inventory move is needed.
        let request = QuestTransitionRequest {
            transition_key: transition_key.into(),
            cause: QuestCause::Command(reward_fixture::command(9800 + u64::from(ordinal))?),
        };
        if number == 3520 {
            let forged = QuestTransitionRequest {
                transition_key: transition_key.into(),
                cause: QuestCause::Command(reward_fixture::command(9998)?),
            };
            let refused = h
                .root
                .commit_character_quest_transition(
                    authority,
                    &h.node,
                    f::fence(5)?,
                    forged,
                    Arc::clone(&catalogue),
                )
                .await
                .map_err(f::debug)?;
            assert!(
                matches!(
                    refused,
                    QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
                ),
                "custody without a native acquisition receipt is insufficient"
            );
        }
        let outcome = h
            .root
            .commit_character_quest_transition(
                authority,
                &h.node,
                f::fence(5)?,
                request.clone(),
                Arc::clone(&catalogue),
            )
            .await
            .map_err(f::debug)?;
        if number == 3519 {
            assert!(matches!(
                outcome,
                QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
            ));
        } else {
            assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
            let replay = h
                .root
                .commit_character_quest_transition(
                    authority,
                    &h.node,
                    f::fence(5)?,
                    request,
                    Arc::clone(&catalogue),
                )
                .await
                .map_err(f::debug)?;
            assert!(matches!(
                replay,
                QuestTransitionOutcome::AlreadyCommitted(_)
            ));
        }
    }
    let value: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/m01c-fanfare-route'")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(value, 1);
    let keys: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item JOIN game_item_keys k ON k.item_instance_id=i.item_instance_id WHERE k.key_number IN (3519,3520) AND i.lifecycle=1 AND i.quantity=1 AND i.definition_production_key='oteryn:item.tibia.i2973'")
        .bind(f::id(f::CHARACTER).as_slice())
        .bind(f::id(f::WORLD).as_slice())
        .fetch_one(&h.pool).await?;
    assert_eq!(keys, 2, "COLLECT must not consume either key");
    let incomplete: bool = sqlx::query_scalar("SELECT completed_character_revision IS NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.fanfare_quest'")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert!(incomplete);
    level_admission_fixture(h, authority, &catalogue, reward_fixture::command(9881)?).await?;
    eprintln!(
        "M01C_FANFARE production_collect native_reward_receipt wrong_key_refused forged_command_refused exact_key_committed custody_preserved replay_safe completion_held"
    );
    Ok(())
}

async fn level_admission_fixture(
    h: &f::Harness,
    authority: &durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    source: &QuestStateCatalogue,
    command: foundation::CommandRef,
) -> f::TestResult {
    use oteryn_gameplay_quests::quest::authored_bindings::{
        Bindings, Trigger, actor_admission::ActorAdmission,
    };
    let level: i64 = sqlx::query_scalar("SELECT level FROM game_character_progression_state WHERE character_id=encode($1,'hex')::uuid")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    let level = u32::try_from(level)?;
    let above = level.checked_add(1).ok_or("level fixture boundary")?;
    let mut quest = source
        .authored_bindings()
        .and_then(|bindings| {
            bindings
                .quests
                .iter()
                .find(|quest| quest.quest == "oteryn:quest.fanfare_quest")
        })
        .ok_or("Fanfare binding fixture")?
        .clone();
    quest.quest = "oteryn:quest.m01c_level_fixture".into();
    quest.sources = vec!["M01c engine level fixture; no production quest qualification".into()];
    quest.tracks[0].key = "oteryn:quest-progress/m01c-level-fixture".into();
    let Trigger::Collect { collection } = &mut quest.stages[0].trigger else {
        return Err("collection fixture".into());
    };
    collection.key = "oteryn:quest-collection/m01c-level-fixture".into();
    collection.storage[0].track = quest.tracks[0].key.clone();
    let key = collection.key.clone();
    for minimum in [above, level] {
        quest.admission = Some(ActorAdmission {
            minimum_level: minimum,
            premium: false,
            prerequisites: Vec::new(),
        });
        let bindings = Bindings {
            schema: 1,
            quests: vec![quest.clone()],
        };
        let catalogue = Arc::new(
            bindings
                .attach(&QuestStateCatalogue::empty("content-1").ok_or("revision")?)
                .map_err(f::debug)?,
        );
        let request = QuestTransitionRequest {
            transition_key: key.clone(),
            cause: QuestCause::Command(command),
        };
        let outcome = h
            .root
            .commit_character_quest_transition(
                authority,
                &h.node,
                f::fence(6)?,
                request.clone(),
                Arc::clone(&catalogue),
            )
            .await
            .map_err(f::debug)?;
        if minimum == above {
            if !matches!(
                outcome,
                QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
            ) {
                return Err(format!("native level guard must refuse minimum {minimum} for level {level}: {outcome:?}").into());
            }
        } else {
            assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
            let replay = h
                .root
                .commit_character_quest_transition(
                    authority,
                    &h.node,
                    f::fence(6)?,
                    request,
                    catalogue,
                )
                .await
                .map_err(f::debug)?;
            assert!(matches!(
                replay,
                QuestTransitionOutcome::AlreadyCommitted(_)
            ));
        }
    }
    eprintln!("M01C_LEVEL actor_level={level} above_refused threshold_committed replay_stable");
    Ok(())
}

async fn production_black_knight_collection(
    h: &f::Harness,
    authority: &durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
) -> f::TestResult {
    use durability::reward_claim_mint::{RewardClaimMintOutcome, RewardClaimMintRequest};
    let catalogue = Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(f::debug)?
            .catalogue()
            .clone(),
    );
    let transition_key = "oteryn:quest-collection/m01c-black-knight-key";
    assert!(catalogue.transition(transition_key).is_some());
    let admission = catalogue
        .authored_bindings()
        .and_then(|b| {
            b.quests
                .iter()
                .find(|q| q.quest == "oteryn:quest.black_knight_quest")
        })
        .and_then(|q| q.admission.as_ref())
        .ok_or("Black Knight actor admission")?;
    assert_eq!(admission.minimum_level, 50);
    for (ordinal, number) in [(90_u32, 5009_u16), (91, 5010)] {
        let request = RewardClaimMintRequest {
            backpack_outputs: false,
            root_charges_remaining: None,
            root_fluid_subtype: None,
            command: reward_fixture::command(9800 + u64::from(ordinal))?,
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: format!("fixture:m01c-key-{number}"),
                revision_ref: "definition-r1".into(),
            },
            source_placement: format!("fixture:m01c-key-box-{number}"),
            item: generation_item_facts(native, &reward_fixture::definition(2969))
                .map_err(f::debug)?,
            quantity: 1,
            contents: Vec::new(),
            additional_items: Vec::new(),
            root_text: None,
            backpack: generation_item_facts(native, &reward_fixture::definition(2854))
                .map_err(f::debug)?,
            content_revision: "content-1".into(),
            ruleset_revision: "ruleset-1".into(),
            sim_revision: "sim-1".into(),
            achievement: None,
            quest_transition: None,
            key_number: Some(number),
        };
        let mut candidate = h
            .root
            .freeze_reward_claim_mint(authority, &h.node, f::item_fence()?, request)
            .await
            .map_err(f::debug)?;
        let minted = h
            .root
            .commit_reward_claim_mint(authority, &h.node, f::item_fence()?, &mut candidate)
            .await
            .map_err(f::debug)?;
        assert!(matches!(minted, RewardClaimMintOutcome::Committed(_)));
        // Claim MINT is itself the native acquisition; no unrelated inventory move is needed.
        let request = QuestTransitionRequest {
            transition_key: transition_key.into(),
            cause: QuestCause::Command(reward_fixture::command(9800 + u64::from(ordinal))?),
        };
        if number == 5010 {
            let forged = QuestTransitionRequest {
                transition_key: transition_key.into(),
                cause: QuestCause::Command(reward_fixture::command(9997)?),
            };
            let refused = h
                .root
                .commit_character_quest_transition(
                    authority,
                    &h.node,
                    f::fence(7)?,
                    forged,
                    Arc::clone(&catalogue),
                )
                .await
                .map_err(f::debug)?;
            assert!(
                matches!(
                    refused,
                    QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
                ),
                "custody without a native acquisition receipt is insufficient"
            );
        }
        let outcome = h
            .root
            .commit_character_quest_transition(
                authority,
                &h.node,
                f::fence(7)?,
                request.clone(),
                Arc::clone(&catalogue),
            )
            .await
            .map_err(f::debug)?;
        if number == 5009 {
            assert!(matches!(
                outcome,
                QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
            ));
        } else {
            assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
            let replay = h
                .root
                .commit_character_quest_transition(
                    authority,
                    &h.node,
                    f::fence(7)?,
                    request,
                    Arc::clone(&catalogue),
                )
                .await
                .map_err(f::debug)?;
            assert!(matches!(
                replay,
                QuestTransitionOutcome::AlreadyCommitted(_)
            ));
        }
    }
    let value: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/m01c-black-knight-route'")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(value, 1);
    let keys: i64 = sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item JOIN game_item_keys k ON k.item_instance_id=i.item_instance_id WHERE k.key_number IN (5009,5010) AND i.lifecycle=1 AND i.quantity=1 AND i.definition_production_key='oteryn:item.tibia.i2969'")
        .bind(f::id(f::CHARACTER).as_slice())
        .bind(f::id(f::WORLD).as_slice())
        .fetch_one(&h.pool).await?;
    assert_eq!(keys, 2, "COLLECT must not consume either key");
    let incomplete: bool = sqlx::query_scalar("SELECT completed_character_revision IS NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.black_knight_quest'")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert!(incomplete);
    premium_admission_fixture(h, authority, &catalogue, reward_fixture::command(9891)?).await?;
    eprintln!(
        "M01C_BLACK_KNIGHT production_collect native_reward_receipt wrong_key_refused forged_command_refused exact_key_committed custody_preserved replay_safe completion_held"
    );
    Ok(())
}

async fn premium_admission_fixture(
    h: &f::Harness,
    authority: &durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    source: &QuestStateCatalogue,
    command: foundation::CommandRef,
) -> f::TestResult {
    use oteryn_gameplay_quests::quest::authored_bindings::{Bindings, Trigger};
    let mut quest = source
        .authored_bindings()
        .and_then(|bindings| {
            bindings
                .quests
                .iter()
                .find(|quest| quest.quest == "oteryn:quest.black_knight_quest")
        })
        .ok_or("Black Knight binding fixture")?
        .clone();
    quest.quest = "oteryn:quest.m01c_premium_fixture".into();
    quest.sources = vec!["M01c premium engine fixture; no production quest qualification".into()];
    quest.tracks[0].key = "oteryn:quest-progress/m01c-premium-fixture".into();
    quest.admission.as_mut().ok_or("level admission")?.premium = true;
    let Trigger::Collect { collection } = &mut quest.stages[0].trigger else {
        return Err("collection fixture".into());
    };
    collection.key = "oteryn:quest-collection/m01c-premium-fixture".into();
    collection.storage[0].track = quest.tracks[0].key.clone();
    let key = collection.key.clone();
    let bindings = Bindings {
        schema: 1,
        quests: vec![quest],
    };
    let catalogue = Arc::new(
        bindings
            .attach(&QuestStateCatalogue::empty("content-1").ok_or("revision")?)
            .map_err(f::debug)?,
    );
    let request = QuestTransitionRequest {
        transition_key: key,
        cause: QuestCause::Command(command),
    };
    let outcome = h
        .root
        .commit_character_quest_transition(
            authority,
            &h.node,
            f::fence(8)?,
            request.clone(),
            Arc::clone(&catalogue),
        )
        .await
        .map_err(f::debug)?;
    if !matches!(
        outcome,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    ) {
        return Err(format!(
            "premium route must fail closed without its registered owner: {outcome:?}"
        )
        .into());
    }
    let account: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    let account: [u8; 16] = account.try_into().map_err(|_| "fixture account UUID")?;
    let expired = PremiumProbe {
        account,
        allowed: false,
        calls: std::sync::atomic::AtomicU32::new(0),
    };
    let outcome = h
        .root
        .commit_character_quest_transition_with_admission(
            authority,
            &h.node,
            f::fence(8)?,
            request.clone(),
            Arc::clone(&catalogue),
            &expired,
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(
        outcome,
        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
    ));
    assert_eq!(expired.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    let current = PremiumProbe {
        account,
        allowed: true,
        calls: std::sync::atomic::AtomicU32::new(0),
    };
    let outcome = h
        .root
        .commit_character_quest_transition_with_admission(
            authority,
            &h.node,
            f::fence(8)?,
            request.clone(),
            Arc::clone(&catalogue),
            &current,
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    let replay = h
        .root
        .commit_character_quest_transition(authority, &h.node, f::fence(8)?, request, catalogue)
        .await
        .map_err(f::debug)?;
    assert!(matches!(
        replay,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    assert_eq!(current.calls.load(std::sync::atomic::Ordering::SeqCst), 1);
    eprintln!(
        "M01C_PREMIUM missing_owner_refused expired_owner_refused registered_owner_committed replay_without_recheck"
    );
    Ok(())
}

struct PremiumProbe {
    account: [u8; 16],
    allowed: bool,
    calls: std::sync::atomic::AtomicU32,
}
impl durability::quest_state::grizzly::GrizzlyPremiumGate for PremiumProbe {
    async fn task_permitted(
        &self,
        _tx: &mut sqlx::Transaction<'_, sqlx::Postgres>,
        account: [u8; 16],
    ) -> Result<bool, durability::DurabilityError> {
        assert_eq!(account, self.account);
        self.calls.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        Ok(self.allowed)
    }
}

async fn production_grimvale_collection(
    h: &f::Harness,
    authority: &durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    native: &oteryn_gameplay_content::content::native_gameplay::NativeGameplayState,
    weights: &reward_fixture::Weights<'_>,
) -> f::TestResult {
    let catalogue = Arc::new(
        oteryn_gameplay_quests::quest::loader::load_embedded_quest_state("content-1")
            .map_err(f::debug)?
            .catalogue()
            .clone(),
    );
    assert!(
        catalogue
            .transition("oteryn:quest-collection/m01c-grimvale-blossoms")
            .is_some()
    );
    let account: Vec<u8> = sqlx::query_scalar("SELECT uuid_send(account_id) FROM game_character_roots WHERE character_id=encode($1,'hex')::uuid")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    let account: [u8; 16] = account.try_into().map_err(|_| "fixture account UUID")?;
    let premium = PremiumProbe {
        account,
        allowed: true,
        calls: std::sync::atomic::AtomicU32::new(0),
    };
    for (ordinal, quantity) in [(120_u32, 4_u32), (121, 1)] {
        let _ = mint_to_bag(h, authority, native, weights, 22082, quantity, ordinal).await?;
        let request = QuestTransitionRequest {
            transition_key: "oteryn:quest-collection/m01c-grimvale-blossoms".into(),
            cause: QuestCause::Command(reward_fixture::command(9300 + u64::from(ordinal))?),
        };
        if quantity == 1 {
            let refused = h
                .root
                .commit_character_quest_transition(
                    authority,
                    &h.node,
                    f::fence(9)?,
                    request.clone(),
                    Arc::clone(&catalogue),
                )
                .await
                .map_err(f::debug)?;
            assert!(
                matches!(
                    refused,
                    QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
                ),
                "five flowers without premium must not advance"
            );
        }
        let outcome = h
            .root
            .commit_character_quest_transition_with_admission(
                authority,
                &h.node,
                f::fence(9)?,
                request.clone(),
                Arc::clone(&catalogue),
                &premium,
            )
            .await
            .map_err(f::debug)?;
        if quantity == 4 {
            assert!(
                matches!(
                    outcome,
                    QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
                ),
                "four blossoms must not satisfy five"
            );
        } else {
            assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
            let replay = h
                .root
                .commit_character_quest_transition(
                    authority,
                    &h.node,
                    f::fence(9)?,
                    request,
                    Arc::clone(&catalogue),
                )
                .await
                .map_err(f::debug)?;
            assert!(matches!(
                replay,
                QuestTransitionOutcome::AlreadyCommitted(_)
            ));
        }
    }
    assert_eq!(premium.calls.load(std::sync::atomic::Ordering::SeqCst), 2);
    let quantity: i64 = sqlx::query_scalar("SELECT coalesce(sum(i.quantity),0)::bigint FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item WHERE i.lifecycle=1 AND i.definition_production_key='oteryn:item.tibia.i22082'")
        .bind(f::id(f::CHARACTER).as_slice()).bind(f::id(f::WORLD).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(quantity, 5, "COLLECT preserves all five carried blossoms");
    let progress: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/m01c-grimvale-route'")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(progress, 1);
    let incomplete: bool = sqlx::query_scalar("SELECT completed_character_revision IS NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.grimvale_quest'")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert!(incomplete);
    prerequisite_admission_fixture(h, authority, &catalogue).await?;
    production_grimvale_hunt(h, authority, &catalogue, &premium).await?;
    eprintln!(
        "M01C_GRIMVALE production_catalogue count4_refused count5_committed missing_premium_refused live_custody5 replay_safe completion_held"
    );
    Ok(())
}

async fn prerequisite_admission_fixture(
    h: &f::Harness,
    authority: &durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    source: &QuestStateCatalogue,
) -> f::TestResult {
    use oteryn_gameplay_quests::quest::authored_bindings::{
        Bindings, Trigger, actor_admission::QuestPrerequisite,
    };
    let template = source
        .authored_bindings()
        .and_then(|b| {
            b.quests
                .iter()
                .find(|q| q.quest == "oteryn:quest.black_knight_quest")
        })
        .ok_or("prerequisite template")?;
    let mut parent = template.clone();
    parent.quest = "oteryn:quest.m01c_dependency".into();
    parent.sources = vec!["M01c prerequisite engine fixture, not a production quest".into()];
    parent.tracks[0].key = "oteryn:quest-progress/m01c-dependency".into();
    parent.tracks[0].maximum = 2;
    let Trigger::Collect { collection } = &mut parent.stages[0].trigger else {
        return Err("parent collection".into());
    };
    collection.key = "oteryn:quest-collection/m01c-dependency-start".into();
    collection.storage[0].track = parent.tracks[0].key.clone();
    let mut finish = parent.stages[0].clone();
    finish.key = "complete".into();
    finish.completes = true;
    let Trigger::Collect { collection } = &mut finish.trigger else {
        return Err("finish collection".into());
    };
    collection.key = "oteryn:quest-collection/m01c-dependency-complete".into();
    collection.storage[0].minimum = 1;
    collection.storage[0].maximum = 1;
    collection.storage[0].set = 2;
    parent.stages.push(finish);
    let mut child = template.clone();
    child.quest = "oteryn:quest.m01c_prerequisite_child".into();
    child.sources = vec!["M01c prerequisite engine fixture, not a production quest".into()];
    child.tracks[0].key = "oteryn:quest-progress/m01c-prerequisite-child".into();
    child.tracks[0].maximum = 1;
    child
        .admission
        .as_mut()
        .ok_or("child admission")?
        .prerequisites = vec![QuestPrerequisite {
        requirement: "M01c dependency".into(),
        quest: parent.quest.clone(),
    }];
    let Trigger::Collect { collection } = &mut child.stages[0].trigger else {
        return Err("child collection".into());
    };
    collection.key = "oteryn:quest-collection/m01c-prerequisite-child".into();
    collection.storage[0].track = child.tracks[0].key.clone();
    let bindings = Bindings {
        schema: 1,
        quests: vec![parent, child],
    };
    let catalogue = Arc::new(
        bindings
            .attach(&QuestStateCatalogue::empty("content-1").ok_or("revision")?)
            .map_err(f::debug)?,
    );
    let request = |key: &str, command: u64| -> f::TestResult<QuestTransitionRequest> {
        Ok(QuestTransitionRequest {
            transition_key: key.into(),
            cause: QuestCause::Command(reward_fixture::command(command)?),
        })
    };
    for (revision, parent_step) in [(10_u64, None), (11, Some("start"))] {
        if let Some(step) = parent_step {
            let outcome = h
                .root
                .commit_character_quest_transition(
                    authority,
                    &h.node,
                    f::fence(10)?,
                    request(
                        &format!("oteryn:quest-collection/m01c-dependency-{step}"),
                        9891,
                    )?,
                    Arc::clone(&catalogue),
                )
                .await
                .map_err(f::debug)?;
            assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
        }
        let outcome = h
            .root
            .commit_character_quest_transition(
                authority,
                &h.node,
                f::fence(revision)?,
                request("oteryn:quest-collection/m01c-prerequisite-child", 9891)?,
                Arc::clone(&catalogue),
            )
            .await
            .map_err(f::debug)?;
        if !matches!(
            outcome,
            QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
        ) {
            return Err(format!(
                "incomplete prerequisite must refuse child at revision {revision}: {outcome:?}"
            )
            .into());
        }
    }
    let finished = h
        .root
        .commit_character_quest_transition(
            authority,
            &h.node,
            f::fence(11)?,
            request("oteryn:quest-collection/m01c-dependency-complete", 9421)?,
            Arc::clone(&catalogue),
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(finished, QuestTransitionOutcome::Committed(_)));
    let child_request = request("oteryn:quest-collection/m01c-prerequisite-child", 9891)?;
    let outcome = h
        .root
        .commit_character_quest_transition(
            authority,
            &h.node,
            f::fence(12)?,
            child_request.clone(),
            Arc::clone(&catalogue),
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(outcome, QuestTransitionOutcome::Committed(_)));
    let replay = h
        .root
        .commit_character_quest_transition(
            authority,
            &h.node,
            f::fence(12)?,
            child_request,
            catalogue,
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(
        replay,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    eprintln!(
        "M01C_PREREQUISITE missing_refused started_refused completed_accepted owner_writes_only replay_stable"
    );
    Ok(())
}

async fn production_grimvale_hunt(
    h: &f::Harness,
    authority: &durability::character_authority::ReconciledCharacterAuthority<'_, '_>,
    catalogue: &Arc<QuestStateCatalogue>,
    premium: &PremiumProbe,
) -> f::TestResult {
    use durability::character_progression::ExperienceRewardOccurrence;
    use durability::quest_state::native_kills::{NativeDeathSource, NativeQuestDeath};
    let hash = catalogue
        .definition_hash("oteryn:quest.grimvale_quest")
        .ok_or("Grimvale definition")?;
    let observed_database_micros: i64 =
        sqlx::query_scalar("SELECT (extract(epoch FROM clock_timestamp())*1000000)::bigint")
            .fetch_one(&h.pool)
            .await?;
    let party = Arc::new(foundation::PartySnapshot {
        world: f::id(f::WORLD),
        character: f::id(f::CHARACTER),
        party_id: None,
        revision: None,
        leader: None,
        shared_xp_enabled: false,
        members: Vec::new(),
        observed_database_micros,
    });
    for n in 0..11_u8 {
        let species = ["werewolf", "werebear", "wereboar"][usize::from(n) % 3];
        let death = NativeQuestDeath {
            transition: format!("oteryn:quest-transition/native-kills/m01c-grimvale-{species}"),
            creature: format!("oteryn:creature.{species}"),
            observed_revision: 13 + u64::from(n),
            definition_hash: Some(hash),
            source: NativeDeathSource::Party(Arc::clone(&party)),
        };
        let cause = QuestCause::CreatureDeath(
            ExperienceRewardOccurrence::from_bytes(f::id(150 + n)).map_err(f::debug)?,
        );
        for replay in [false, true] {
            let outcome = h
                .root
                .commit_character_native_quest_kill_with_admission(
                    authority,
                    &h.node,
                    f::fence(13 + u64::from(n))?,
                    cause,
                    death.clone(),
                    Arc::clone(catalogue),
                    premium,
                )
                .await
                .map_err(f::debug)?;
            assert!(
                if n == 10 {
                    matches!(
                        outcome,
                        QuestTransitionOutcome::Refused(QuestRefusal::StageMismatch)
                    )
                } else if replay {
                    matches!(outcome, QuestTransitionOutcome::AlreadyCommitted(_))
                } else {
                    matches!(outcome, QuestTransitionOutcome::Committed(_))
                },
                "kill {n} replay={replay}: {outcome:?}"
            );
        }
        let count: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/m01c-grimvale-kills'")
            .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(count, (i64::from(n) + 1).min(10));
        let route: i64 = sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/m01c-grimvale-route'")
            .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
        assert_eq!(route, if n >= 9 { 2 } else { 1 });
    }
    let incomplete: bool = sqlx::query_scalar("SELECT completed_character_revision IS NULL FROM game_character_quest_states WHERE character_id=encode($1,'hex')::uuid AND quest_key='oteryn:quest.grimvale_quest'")
        .bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert!(incomplete, "the hunt must not complete the whole quest");
    eprintln!(
        "M01C_GRIMVALE_HUNT mixed_species10 eleventh_refused bounded_counter phase2 replay_stable completion_held root_notification_fixture"
    );
    Ok(())
}
