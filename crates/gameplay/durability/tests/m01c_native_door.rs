//! Registered door-source admission; the real World capture is exercised separately.
#![cfg(feature = "test-support")]
use oteryn_gameplay_character_recovery_fence as character_recovery_fence;
use oteryn_gameplay_domain::domain;
use oteryn_gameplay_durability::durability;
use oteryn_gameplay_entities as foundation;
#[allow(
    dead_code,
    reason = "reuse canonical PostgreSQL session-generation harness"
)]
#[path = "../../../../apps/game-server/tests/support/bestiary_postgres_harness.rs"]
mod bestiary_postgres_harness;
#[allow(dead_code, reason = "reuse native backpack setup and item facts")]
#[path = "../../../../apps/game-server/tests/support/authority_cases/engitem_reward_text.rs"]
mod reward_fixture;
use bestiary_postgres_harness as f;
use durability::quest_state::native_doors::NativeQuestDoorOpening;
use durability::quest_state::{QuestCause, QuestTransitionOutcome, QuestTransitionRequest};
use std::sync::Arc;

#[test]
fn m01c_native_door_requires_captured_source_and_exact_owned_key_postgres() -> f::TestResult {
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
                    let h = f::Harness::create(admin, "questpc3_m01c_door", true).await?;
                    let result = exercise(&h).await;
                    h.cleanup().await?;
                    result?;
                    assert!(started.elapsed().as_secs_f64() > 0.01);
                    eprintln!(
                        "M01C_DOOR POSTGRES_CASE_COMPLETED duration_ms={} success=true",
                        started.elapsed().as_millis()
                    );
                    Ok::<(), Box<dyn std::error::Error>>(())
                })
                .map_err(f::debug)
        })?
        .join()
        .map_err(|_| "native door PostgreSQL thread")?
        .map_err(Into::into)
}

async fn exercise(h: &f::Harness) -> f::TestResult {
    use durability::item_mint::TypedDefinitionRef;
    use durability::reward_claim_mint::{RewardClaimMintOutcome, RewardClaimMintRequest};
    use oteryn_gameplay_items::item_ref_admission::generation_item_facts;
    let version: String = sqlx::query_scalar("SHOW server_version_num")
        .fetch_one(&h.pool)
        .await?;
    assert_eq!(
        version,
        oteryn_gameplay_durability::CANONICAL_POSTGRES_SERVER_VERSION_NUM
    );
    eprintln!("M01C_DOOR POSTGRES_CONNECTED version={version}");
    let (controller, _) =
        oteryn_gameplay_content::content::native_gameplay::test_support::activated_with_loot_tables(
            f::scope()?.world_id(),
            None,
        );
    let native = controller
        .active()
        .and_then(|a| a.native_gameplay())
        .ok_or("native content")?;
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
    for (ordinal, number) in [(101_u64, 3519_u16), (102, 3520)] {
        let request = RewardClaimMintRequest {
            command: reward_fixture::command(10100 + ordinal)?,
            claim: TypedDefinitionRef {
                family: "RewardClaim".into(),
                production_key: format!("fixture:m01c-door-key-{number}"),
                revision_ref: "definition-r1".into(),
            },
            source_placement: format!("fixture:m01c-door-key-box-{number}"),
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
            .freeze_reward_claim_mint(&authority, &h.node, f::item_fence()?, request)
            .await
            .map_err(f::debug)?;
        let minted = h
            .root
            .commit_reward_claim_mint(&authority, &h.node, f::item_fence()?, &mut candidate)
            .await
            .map_err(f::debug)?;
        assert!(matches!(minted, RewardClaimMintOutcome::Committed(_)));
    }
    let bindings = oteryn_gameplay_quests::quest::authored_bindings::Bindings::parse(
        include_bytes!("../../quests/src/quest/authored_bindings/fixtures/door_use.json"),
    )
    .map_err(f::debug)?;
    let catalogue = Arc::new(
        bindings
            .attach(
                &oteryn_gameplay_quests::quest::QuestStateCatalogue::empty("content-1")
                    .ok_or("catalogue")?,
            )
            .map_err(f::debug)?,
    );
    let key_instance:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item JOIN game_item_keys k ON k.item_instance_id=i.item_instance_id WHERE k.key_number=3520 LIMIT 2")
        .bind(f::id(f::CHARACTER).as_slice()).bind(f::id(f::WORLD).as_slice()).fetch_one(&h.pool).await?;
    let mut placement = [19_u8; 40];
    placement[32..].copy_from_slice(&7_u64.to_be_bytes());
    let opening = NativeQuestDoorOpening {
        transition: "oteryn:quest-door-use/m01c-fixture".into(),
        placement,
        key_instance: key_instance.try_into().map_err(|_| "key UUID")?,
        key_number: 3520,
        door_revision: 1,
        observed_character_revision: 1,
        definition_hash: catalogue
            .definition_hash("oteryn:quest.m01c_door_fixture")
            .ok_or("definition hash")?,
    };
    let cause = QuestCause::Command(reward_fixture::command(10300)?);
    let plain = h
        .root
        .commit_character_quest_transition(
            &authority,
            &h.node,
            f::fence(1)?,
            QuestTransitionRequest {
                transition_key: opening.transition.clone(),
                cause,
            },
            Arc::clone(&catalogue),
        )
        .await;
    assert!(
        plain.is_err(),
        "a client quest command cannot claim a door opening"
    );
    let sequencer = durability::character_revision_sequencer::CharacterRevisionSequencer::new();
    let mut slot = sequencer.acquire(f::fence(1)?.character_id).await;
    let wrong_key:Vec<u8>=sqlx::query_scalar("SELECT uuid_send(i.item_instance_id) FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item JOIN game_item_keys k ON k.item_instance_id=i.item_instance_id WHERE k.key_number=3519 LIMIT 2")
        .bind(f::id(f::CHARACTER).as_slice()).bind(f::id(f::WORLD).as_slice()).fetch_one(&h.pool).await?;
    for bad in 0..3 {
        let mut missing = opening.clone();
        match bad {
            0 => missing.key_instance = f::id(250),
            1 => {
                missing.key_instance = wrong_key
                    .as_slice()
                    .try_into()
                    .map_err(|_| "wrong key UUID")?
            }
            _ => missing.observed_character_revision = 2,
        }
        let refused = slot
            .commit_native_quest_door(
                &h.root,
                &authority,
                &h.node,
                f::fence(1)?,
                cause,
                &missing,
                Arc::clone(&catalogue),
            )
            .await
            .map_err(f::debug)?;
        assert!(
            matches!(refused, QuestTransitionOutcome::Refused(_)),
            "missing/wrong key or future capture must refuse case={bad}"
        );
    }
    for bad in 0..3 {
        let mut invalid = opening.clone();
        match bad {
            0 => invalid.placement[39] = 8,
            1 => invalid.key_number = 3519,
            _ => invalid.definition_hash = [0; 32],
        }
        let refused = slot
            .commit_native_quest_door(
                &h.root,
                &authority,
                &h.node,
                f::fence(1)?,
                cause,
                &invalid,
                Arc::clone(&catalogue),
            )
            .await;
        assert!(
            matches!(
                refused,
                Err(durability::character_progression::CharacterProgressionError::InvalidInput)
            ),
            "placement/number/definition must match case={bad}"
        );
    }
    let accepted = slot
        .commit_native_quest_door(
            &h.root,
            &authority,
            &h.node,
            f::fence(1)?,
            cause,
            &opening,
            Arc::clone(&catalogue),
        )
        .await
        .map_err(f::debug)?;
    assert!(
        matches!(accepted, QuestTransitionOutcome::Committed(_)),
        "captured opening with exact carried key: {accepted:?}"
    );
    let replay = slot
        .commit_native_quest_door(
            &h.root,
            &authority,
            &h.node,
            f::fence(2)?,
            cause,
            &opening,
            catalogue,
        )
        .await
        .map_err(f::debug)?;
    assert!(matches!(
        replay,
        QuestTransitionOutcome::AlreadyCommitted(_)
    ));
    let value:i64=sqlx::query_scalar("SELECT value FROM game_character_quest_tracks WHERE character_id=encode($1,'hex')::uuid AND track_key='oteryn:quest-progress/m01c-door-fixture'").bind(f::id(f::CHARACTER).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(value, 1);
    let preserved:i64=sqlx::query_scalar("SELECT count(*) FROM game_quest_callback_carried(encode($1,'hex')::uuid,encode($2,'hex')::uuid) c JOIN game_item_instances i ON i.item_instance_id=c.item JOIN game_item_keys k ON k.item_instance_id=i.item_instance_id WHERE k.key_number IN (3519,3520) AND i.lifecycle=1 AND i.quantity=1")
        .bind(f::id(f::CHARACTER).as_slice()).bind(f::id(f::WORLD).as_slice()).fetch_one(&h.pool).await?;
    assert_eq!(preserved, 2, "door admission never consumes either key");
    eprintln!(
        "M01C_DOOR native_receiver_fixture plain_command_denied missing_and_wrong_key_refused future_capture_refused placement_number_definition_bound exact_key_accepted replay_safe custody_preserved"
    );
    Ok(())
}
